//! The promotion gate for the exit rule — a kind with unspent resources must always have a way out: every
//! ERC20 forwarder of the promoted environment accepts a listed circuit version. Every listed version is a member of every fungibility domain, so an accepted version that is listed
//! gives every other listed version an exit by conversion. Runs only on a pull request into `staging` or
//! `main`, selected as the freshness gate is; elsewhere it skips.

use anoma_risc0_kind_tables::{Chain, circuits, table};
use anoma_risc0_kind_tables_integration_test::provider;
use anomapay_erc20_forwarder_bindings::addresses::{Environment, erc20_forwarder_address};
use anomapay_erc20_forwarder_bindings::contract::erc20_forwarder;
use anyhow::{Context, Result, ensure};
use risc0_zkvm::Digest;

fn promotion_target() -> Option<Environment> {
    let base = std::env::var("PROMOTION_TARGET")
        .or_else(|_| std::env::var("GITHUB_BASE_REF"))
        .ok()?;
    match base.as_str() {
        "staging" => Some(Environment::Staging),
        "main" => Some(Environment::Production),
        _ => None,
    }
}

#[tokio::test]
async fn the_promoted_forwarders_accept_a_listed_circuit_version() -> Result<()> {
    let Some(environment) = promotion_target() else {
        eprintln!("skipped: not a promotion pull request");
        return Ok(());
    };
    let chains = match environment {
        Environment::Staging => table::staging::chains(),
        Environment::Production => table::production::chains(),
    };

    for chain in chains {
        // A Solana cluster's forwarder is checked by the Solana exit gate.
        let Chain::Evm(evm_chain) = Chain::try_from(&chain)? else {
            continue;
        };
        if erc20_forwarder_address(environment, &evm_chain).is_none() {
            continue; // No ERC20 fungibility domain on this chain.
        }
        let provider = provider(&chain)?;
        let forwarder = erc20_forwarder(&provider, environment)
            .await
            .with_context(|| format!("no {environment:?} ERC20 forwarder recorded on {chain}"))?;
        let accepted = forwarder
            .getLogicRef()
            .call()
            .await
            .with_context(|| chain.to_string())?;
        let accepted = Digest::try_from(accepted.as_slice()).context("a logic ref is 32 bytes")?;
        ensure!(
            circuits::erc20_version(&accepted).is_some(),
            "{chain}: the ERC20 forwarder accepts logic ref {}, which circuit-versions.json does not list",
            hex::encode(accepted.as_bytes())
        );
    }
    Ok(())
}
