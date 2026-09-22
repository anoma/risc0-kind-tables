//! The exit rule on Solana — a kind with unspent resources must always have a way out: the SPL token forwarder of
//! every recorded cluster accepts a listed SPL token circuit version, so every other listed version has an exit by
//! conversion. The forwarder's config pins the logic ref it accepts; the same check the EVM forwarders get from
//! their promotion gate. Runs on every pull request and push, since the devnet forwarder is the only one recorded.
use anoma_risc0_kind_tables::{Chain, SolanaAddress, circuits, tokens};
use anoma_risc0_kind_tables_integration_test::{solana_forwarder, solana_rpc};
use anyhow::{Context, Result, ensure};
use risc0_zkvm::Digest;

#[tokio::test]
async fn every_recorded_forwarder_accepts_a_listed_circuit_version() -> Result<()> {
    for chain in tokens::all().keys() {
        let Chain::Solana(cluster) = Chain::try_from(chain)? else {
            continue;
        };
        let forwarder: SolanaAddress = solana_forwarder(cluster)
            .with_context(|| format!("no SPL token forwarder recorded for {}", cluster.name()))?;
        let config = solana_rpc(cluster)
            .forwarder_config(&forwarder)
            .await?
            .with_context(|| {
                format!(
                    "{}: the forwarder {forwarder} has no config",
                    cluster.name()
                )
            })?;
        let accepted = Digest::from(config.logic_ref);
        ensure!(
            circuits::spl_token_version(&accepted).is_some(),
            "{}: the forwarder accepts {accepted}, which circuit-versions.json does not list",
            cluster.name()
        );
    }
    Ok(())
}
