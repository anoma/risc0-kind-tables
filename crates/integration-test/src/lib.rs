//! Test-support helpers for the kind tables integration tests.

use alloy::providers::{DynProvider, Provider, ProviderBuilder};
use alloy_chains::NamedChain;
use anoma_pa_evm_bindings::helpers::alchemy_url;
use anyhow::{Context, Result, ensure};

/// Connects an Alchemy-backed provider for the chain. Every chain carrying supported tokens or a recorded
/// deployment must be reachable this way; an unknown chain is a hard failure, never a skip.
pub fn provider(chain: NamedChain) -> Result<DynProvider> {
    let url = alchemy_url(&chain).with_context(|| format!("no RPC route for {chain}"))?;
    Ok(ProviderBuilder::new().connect_http(url).erased())
}

use anoma_risc0_kind_tables::{SolanaAddress, SolanaCluster};
use base64::Engine;
use risc0_zkvm::Digest;
use risc0_zkvm::sha::{Impl, Sha256};
use serde_json::{Value, json};
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use std::sync::LazyLock;

/// One HTTP client for every cluster: the TLS setup and the connection pool are built once per process.
static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

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
    }
}

/// A JSON-RPC connection to one cluster, with the few reads the gates need.
pub struct SolanaRpc {
    url: String,
}

fn pubkey(address: &SolanaAddress) -> Pubkey {
    Pubkey::new_from_array(*address.as_bytes())
}

impl SolanaRpc {
    async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let Value::Object(mut response) = CLIENT
            .post(&self.url)
            .json(&json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}))
            .send()
            .await
            .with_context(|| format!("{method} on {}", self.url))?
            .json::<Value>()
            .await
            .with_context(|| format!("{method} on {}: not JSON", self.url))?
        else {
            anyhow::bail!("{method} on {}: not a JSON-RPC response", self.url);
        };
        if let Some(error) = response.get("error") {
            anyhow::bail!("{method} on {}: {error}", self.url);
        }
        response
            .remove("result")
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

    /// The decimals of the SPL token mint at the address, or `None` when no account is there.
    pub async fn mint_decimals(&self, address: &SolanaAddress) -> Result<Option<u8>> {
        let Some((owner, data)) = self.account(address).await? else {
            return Ok(None);
        };
        ensure!(
            owner == spl_token::id().to_string(),
            "{address}: owned by {owner}, not the SPL Token program"
        );
        let mint = spl_token::state::Mint::unpack(&data)
            .with_context(|| format!("{address}: not an SPL token mint"))?;
        Ok(Some(mint.decimals))
    }

    /// The logic ref the SPL token forwarder's config accepts, or `None` when the forwarder is not initialized.
    /// The config is the forwarder's `Config` account: the Anchor discriminator, the adapter program id, the
    /// logic ref, the emergency committee, the emergency caller and the bump.
    pub async fn forwarder_logic_ref(&self, forwarder: &SolanaAddress) -> Result<Option<Digest>> {
        let (config, _) = anoma_pa_solana_client::derive_forwarder_config_pda(&pubkey(forwarder));
        let config = SolanaAddress::new(config.to_bytes());
        let Some((owner, data)) = self.account(&config).await? else {
            return Ok(None);
        };
        ensure!(
            owner == forwarder.to_string(),
            "{config}: owned by {owner}, not the forwarder {forwarder}"
        );
        ensure!(
            data.len() == 8 + 32 + 32 + 32 + 32 + 1
                && data[..8] == Impl::hash_bytes(b"account:Config").as_bytes()[..8],
            "{config}: not a forwarder Config account"
        );
        Ok(Some(Digest::try_from(&data[40..72]).expect("32 bytes")))
    }

    /// The kind-table commitment the protocol adapter stores, or `None` when it is not initialized.
    pub async fn adapter_kind_table_commitment(
        &self,
        adapter: &SolanaAddress,
    ) -> Result<Option<Digest>> {
        let (state, _) = anoma_pa_solana_client::derive_pa_state_pda(&pubkey(adapter));
        let state = SolanaAddress::new(state.to_bytes());
        let Some((owner, data)) = self.account(&state).await? else {
            return Ok(None);
        };
        ensure!(
            owner == adapter.to_string(),
            "{state}: owned by {owner}, not the adapter {adapter}"
        );
        let decoded = anoma_pa_solana_client::decode_pa_state(&data)
            .map_err(|error| anyhow::anyhow!("{state}: {error:?}"))?;
        Ok(Some(Digest::from(decoded.kind_table_commitment)))
    }
}
