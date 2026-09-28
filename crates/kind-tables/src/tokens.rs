//! The supported tokens — the authored identity list the token entries of every chain table are built from.
//! Being supported is a standing commitment: a chain may list tokens before anything is deployed to it. An EVM
//! chain lists ERC20 tokens by address; a Solana cluster lists SPL token mints.
use crate::chain::{Chain, SolanaCluster};
use crate::solana::SolanaAddress;
use alloy::primitives::Address;
use alloy_chains::NamedChain;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::LazyLock;

/// The identity of a supported ERC20 token: what the contract itself reports, and whether its V1 resources convert.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Token {
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub address: Address,
    /// Whether the V1 forwarder's label joins this token's fungibility domain, which makes its V1 resources
    /// fungible with its current ones, so they convert and leave through the current forwarder. Every token states
    /// it; a token set to `false` keeps its V1 resources where they are.
    pub fungible_with_v1: bool,
}

/// The identity of a supported SPL token: its mint, and the name and symbol review knows it by. A mint reports
/// only its decimals on chain; symbol and name are review context.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SplToken {
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub mint: SolanaAddress,
}

/// One chain's authored section. The `_comment` naming the chain is review context and is not deserialized.
#[derive(Deserialize)]
struct Section<T> {
    tokens: Vec<T>,
}

/// The authored list, split by chain family: the key decides which token type a section holds.
struct Tokens {
    erc20: BTreeMap<NamedChain, Vec<Token>>,
    spl: BTreeMap<SolanaCluster, Vec<SplToken>>,
}

static TOKENS: LazyLock<Tokens> = LazyLock::new(|| {
    let raw: BTreeMap<String, serde_json::Value> =
        serde_json::from_str(include_str!("../data/tokens.json"))
            .expect("tokens.json: invalid JSON");
    let mut tokens = Tokens {
        erc20: BTreeMap::new(),
        spl: BTreeMap::new(),
    };
    for (key, section) in raw {
        // A chain that fails to resolve must fail loudly: dropping it would silently drop its kinds.
        let chain = Chain::from_key(&key).unwrap_or_else(|error| panic!("tokens.json: {error}"));
        match chain {
            Chain::Evm(named) => {
                tokens.erc20.insert(named, typed(chain, section));
            }
            Chain::Solana(cluster) => {
                tokens.spl.insert(cluster, typed(chain, section));
            }
        }
    }
    tokens
});

/// Deserializes a chain's section as the token type its chain family lists, failing loudly on any other shape.
fn typed<T: serde::de::DeserializeOwned>(chain: Chain, section: serde_json::Value) -> Vec<T> {
    serde_json::from_value::<Section<T>>(section)
        .unwrap_or_else(|error| panic!("tokens.json: {chain}: {error}"))
        .tokens
}

/// All supported ERC20 tokens, per EVM chain.
pub fn all() -> &'static BTreeMap<NamedChain, Vec<Token>> {
    &TOKENS.erc20
}

/// The supported ERC20 tokens on the chain.
pub fn on(chain: NamedChain) -> &'static [Token] {
    TOKENS.erc20.get(&chain).map_or(&[], Vec::as_slice)
}

/// All supported SPL token mints, per Solana cluster.
pub fn spl_all() -> &'static BTreeMap<SolanaCluster, Vec<SplToken>> {
    &TOKENS.spl
}

/// The supported SPL token mints on the cluster.
pub fn spl_on(cluster: SolanaCluster) -> &'static [SplToken] {
    TOKENS.spl.get(&cluster).map_or(&[], Vec::as_slice)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_list_carries_the_devnet_test_mint_under_its_caip2_key() {
        let [token] = spl_on(SolanaCluster::Devnet) else {
            panic!("expected exactly one supported mint on solana-devnet");
        };
        assert_eq!(
            token.mint.to_string(),
            "9EHEFzyuY7sZEzTVm7C3uMkNZFMgm5ZeWjGjirZ3MVfr"
        );
        assert_eq!(token.decimals, 6);
    }
}
