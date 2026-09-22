//! Test-support helpers for the kind tables integration tests.

use alloy::providers::{DynProvider, Provider, ProviderBuilder};
use alloy_chains::NamedChain;
use anoma_pa_evm_bindings::helpers::alchemy_url;
use anoma_risc0_kind_tables::Caip2ChainId;
use anyhow::{Context, Result, ensure};

/// Connects an Alchemy-backed provider for the chain. Every chain carrying supported tokens or a recorded
/// deployment must be reachable this way; an unknown chain, or a chain that is not an EVM chain, is a hard failure,
/// never a skip.
pub fn provider(chain: &Caip2ChainId) -> Result<DynProvider> {
    let evm_chain = NamedChain::try_from(chain)?;
    let url = alchemy_url(&evm_chain).with_context(|| format!("no RPC route for {chain}"))?;
    Ok(ProviderBuilder::new().connect_http(url).erased())
}

pub use anoma_risc0_kind_tables::deployments::solana_forwarder;
use anoma_risc0_kind_tables::{SolanaAddress, SolanaCluster};
use base64::Engine;
use risc0_zkvm::sha::{Impl, Sha256};
use serde_json::{Value, json};

/// The RPC endpoint of a Solana cluster: `SOLANA_RPC_URL_DEVNET` / `SOLANA_RPC_URL_MAINNET_BETA` when set, else
/// the cluster's public endpoint.
pub fn solana_rpc(cluster: SolanaCluster) -> SolanaRpc {
    let (variable, public) = match cluster {
        SolanaCluster::Devnet => ("SOLANA_RPC_URL_DEVNET", "https://api.devnet.solana.com"),
        SolanaCluster::MainnetBeta => (
            "SOLANA_RPC_URL_MAINNET_BETA",
            "https://api.mainnet-beta.solana.com",
        ),
    };
    SolanaRpc {
        url: std::env::var(variable).unwrap_or_else(|_| public.to_string()),
        client: reqwest::Client::new(),
    }
}

/// A JSON-RPC connection to one cluster, with the few reads the gates need.
pub struct SolanaRpc {
    url: String,
    client: reqwest::Client,
}

/// An SPL token mint account, as the token validation reads it.
pub struct Mint {
    pub decimals: u8,
}

/// The SPL token forwarder's config account, as the exit gate reads it.
pub struct ForwarderConfig {
    pub logic_ref: [u8; 32],
}

/// The SPL Token program, the only owner a supported mint may have.
const SPL_TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
/// The Anchor discriminator of the forwarder's `Config` account: the first eight bytes of
/// `sha256("account:Config")`.
fn config_discriminator() -> [u8; 8] {
    let digest = Impl::hash_bytes(b"account:Config");
    digest.as_bytes()[..8].try_into().expect("eight bytes")
}
/// The forwarder's config PDA, as the client derives it.
fn config_address(forwarder: &SolanaAddress) -> SolanaAddress {
    let program = solana_program::pubkey::Pubkey::new_from_array(*forwarder.as_bytes());
    let (address, _) = anoma_pa_solana_client::derive_forwarder_config_pda(&program);
    SolanaAddress::new(address.to_bytes())
}

impl SolanaRpc {
    async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let response: Value = self
            .client
            .post(&self.url)
            .json(&json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}))
            .send()
            .await
            .with_context(|| format!("{method} on {}", self.url))?
            .json()
            .await
            .with_context(|| format!("{method} on {}: not JSON", self.url))?;
        if let Some(error) = response.get("error") {
            anyhow::bail!("{method} on {}: {error}", self.url);
        }
        response
            .get("result")
            .cloned()
            .with_context(|| format!("{method} on {}: no result", self.url))
    }

    /// The cluster's genesis hash, base58.
    pub async fn genesis_hash(&self) -> Result<String> {
        self.call("getGenesisHash", json!([]))
            .await?
            .as_str()
            .map(str::to_string)
            .context("getGenesisHash: not a string")
    }

    /// An account's owner and data, or `None` when it does not exist.
    async fn account(&self, address: &SolanaAddress) -> Result<Option<(String, Vec<u8>)>> {
        let result = self
            .call(
                "getAccountInfo",
                json!([address.to_string(), {"encoding": "base64"}]),
            )
            .await?;
        let Some(value) = result.get("value").filter(|value| !value.is_null()) else {
            return Ok(None);
        };
        let owner = value["owner"]
            .as_str()
            .context("getAccountInfo: no owner")?
            .to_string();
        let encoded = value["data"][0]
            .as_str()
            .context("getAccountInfo: no data")?;
        let data = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .context("getAccountInfo: data is not base64")?;
        Ok(Some((owner, data)))
    }

    /// The SPL token mint at the address: an 82-byte account owned by the SPL Token program, whose byte 44 is
    /// the decimals (after the 36-byte optional mint authority and the 8-byte supply).
    pub async fn mint(&self, address: &SolanaAddress) -> Result<Option<Mint>> {
        let Some((owner, data)) = self.account(address).await? else {
            return Ok(None);
        };
        ensure!(
            owner == SPL_TOKEN_PROGRAM,
            "{address}: owned by {owner}, not the SPL Token program"
        );
        ensure!(
            data.len() == 82,
            "{address}: {} bytes, not an SPL token mint",
            data.len()
        );
        Ok(Some(Mint { decimals: data[44] }))
    }

    /// The forwarder's config: its `Config` account (Anchor discriminator, adapter program id, logic ref,
    /// emergency committee, emergency caller, bump), or `None` when the forwarder is not initialized.
    pub async fn forwarder_config(
        &self,
        forwarder: &SolanaAddress,
    ) -> Result<Option<ForwarderConfig>> {
        let address = config_address(forwarder);
        let Some((owner, data)) = self.account(&address).await? else {
            return Ok(None);
        };
        ensure!(
            owner == forwarder.to_string(),
            "{address}: owned by {owner}, not the forwarder {forwarder}"
        );
        ensure!(
            data.len() == 8 + 32 + 32 + 32 + 32 + 1 && data[..8] == config_discriminator(),
            "{address}: not a forwarder Config account"
        );
        Ok(Some(ForwarderConfig {
            logic_ref: data[40..72].try_into().expect("32 bytes"),
        }))
    }
}
