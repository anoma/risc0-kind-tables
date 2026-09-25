//! The supported tokens — the authored identity list the ERC20 entries of every chain table are built from.
//! Being supported is a standing commitment: a chain may list tokens before anything is deployed to it.

use crate::chain::Caip2ChainId;
use alloy::primitives::Address;
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

/// One chain's authored section. The `_comment` naming the chain is review context and is not deserialized.
#[derive(Deserialize)]
struct ChainTokens {
    tokens: Vec<Token>,
}

static TOKENS: LazyLock<BTreeMap<Caip2ChainId, Vec<Token>>> = LazyLock::new(|| {
    let raw: BTreeMap<Caip2ChainId, ChainTokens> =
        serde_json::from_str(include_str!("../data/tokens.json"))
            .expect("tokens.json: invalid JSON");
    raw.into_iter()
        .map(|(chain, section)| {
            // A chain that fails to resolve must fail loudly: dropping it would silently drop its kinds.
            assert!(chain.name().is_some(), "unknown chain: {chain}");
            (chain, section.tokens)
        })
        .collect()
});

/// All supported tokens, per chain.
pub fn all() -> &'static BTreeMap<Caip2ChainId, Vec<Token>> {
    &TOKENS
}

/// The supported tokens on the chain.
pub fn on(chain: impl Into<Caip2ChainId>) -> &'static [Token] {
    TOKENS.get(&chain.into()).map_or(&[], Vec::as_slice)
}
