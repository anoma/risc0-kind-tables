//! Test-support helpers for the kind tables integration tests.

use alloy::providers::{DynProvider, Provider, ProviderBuilder};
use alloy_chains::NamedChain;
use anoma_pa_evm_bindings::helpers::alchemy_url;
use anoma_risc0_kind_tables::Caip2ChainId;
use anyhow::{Context, Result};

/// Connects an Alchemy-backed provider for the chain. Every chain carrying supported tokens or a recorded
/// deployment must be reachable this way; an unknown chain, or a chain that is not an EVM chain, is a hard failure,
/// never a skip.
pub fn provider(chain: &Caip2ChainId) -> Result<DynProvider> {
    let evm_chain = NamedChain::try_from(chain)?;
    let url = alchemy_url(&evm_chain).with_context(|| format!("no RPC route for {chain}"))?;
    Ok(ProviderBuilder::new().connect_http(url).erased())
}
