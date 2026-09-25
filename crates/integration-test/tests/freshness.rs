//! The promotion gate: every protocol adapter of the promoted environment stores the commitment this source
//! generates. Runs only on a pull request into `staging` or `main` (selected via `GITHUB_BASE_REF`, or
//! `PROMOTION_TARGET` for a local run); a table change is deployed after merging into `next`, and the promotion
//! pull request turning green is the proof it happened.

use anoma_pa_evm_bindings::addresses::Environment;
use anoma_pa_evm_bindings::contract::protocol_adapter;
use anoma_risc0_kind_tables::{Caip2ChainId, table};
use anoma_risc0_kind_tables_integration_test::provider;
use anyhow::{Context, Result, ensure};
use risc0_zkvm::Digest;
use std::collections::BTreeMap;

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

fn commitments(environment: Environment) -> &'static BTreeMap<Caip2ChainId, Digest> {
    match environment {
        Environment::Staging => table::staging::commitments(),
        Environment::Production => table::production::commitments(),
    }
}

#[tokio::test]
async fn the_promoted_environment_stores_the_generated_commitments() -> Result<()> {
    let Some(environment) = promotion_target() else {
        eprintln!("skipped: not a promotion pull request");
        return Ok(());
    };

    for (chain, expected) in commitments(environment) {
        let provider = provider(chain)?;
        let adapter = protocol_adapter(&provider, environment)
            .await
            .with_context(|| format!("no {environment:?} protocol adapter recorded on {chain}"))?;
        let stored = adapter
            .getKindTableCommitment()
            .call()
            .await
            .with_context(|| chain)?;
        ensure!(
            stored.as_slice() == expected.as_bytes(),
            "{chain}: the protocol adapter stores {stored}, the source generates {expected}"
        );
    }
    Ok(())
}
