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

/// One chain's supported tokens, typed by the chain they are on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChainTokens {
    Erc20(Vec<Token>),
    Spl(Vec<SplToken>),
}

/// One chain's authored section. The `_comment` naming the chain is review context and is not deserialized.
#[derive(Deserialize)]
struct Section {
    tokens: serde_json::Value,
}

static TOKENS: LazyLock<BTreeMap<Chain, ChainTokens>> = LazyLock::new(|| {
    let raw: BTreeMap<String, Section> = serde_json::from_str(include_str!("../data/tokens.json"))
        .expect("tokens.json: invalid JSON");
    raw.into_iter()
        .map(|(key, section)| {
            // A chain that fails to resolve must fail loudly: dropping it would silently drop its kinds.
            let chain =
                Chain::from_key(&key).unwrap_or_else(|error| panic!("tokens.json: {error}"));
            let tokens = match chain {
                Chain::Evm(_) => ChainTokens::Erc20(typed(chain, section.tokens)),
                Chain::Solana(_) => ChainTokens::Spl(typed(chain, section.tokens)),
            };
            (chain, tokens)
        })
        .collect()
});

/// Deserializes a chain's tokens as the type its chain kind lists, failing loudly on a token of the other shape.
fn typed<T: serde::de::DeserializeOwned>(chain: Chain, tokens: serde_json::Value) -> Vec<T> {
    serde_json::from_value(tokens).unwrap_or_else(|error| panic!("tokens.json: {chain}: {error}"))
}

/// All supported tokens, per chain.
pub fn all() -> &'static BTreeMap<Chain, ChainTokens> {
    &TOKENS
}

/// The supported ERC20 tokens on the chain.
pub fn on(chain: NamedChain) -> &'static [Token] {
    match TOKENS.get(&Chain::Evm(chain)) {
        Some(ChainTokens::Erc20(tokens)) => tokens,
        None => &[],
        Some(ChainTokens::Spl(_)) => unreachable!("an EVM chain lists ERC20 tokens"),
    }
}

/// The supported SPL token mints on the cluster.
pub fn spl_on(cluster: SolanaCluster) -> &'static [SplToken] {
    match TOKENS.get(&Chain::Solana(cluster)) {
        Some(ChainTokens::Spl(tokens)) => tokens,
        None => &[],
        Some(ChainTokens::Erc20(_)) => unreachable!("a Solana cluster lists SPL token mints"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::{Chain, SolanaCluster};

    #[test]
    fn the_embedded_list_carries_the_devnet_test_mint_under_its_caip2_key() {
        let devnet = Chain::Solana(SolanaCluster::Devnet);
        assert!(
            all().contains_key(&devnet),
            "tokens.json has no solana-devnet section"
        );
        let [token] = spl_on(SolanaCluster::Devnet) else {
            panic!("expected exactly one supported mint on solana-devnet");
        };
        assert_eq!(
            token.mint.to_string(),
            "9EHEFzyuY7sZEzTVm7C3uMkNZFMgm5ZeWjGjirZ3MVfr"
        );
        assert_eq!(token.decimals, 6);
        assert!(on(NamedChain::Sepolia).len() >= 5);
    }
}
