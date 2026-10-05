//! The recorded Solana deployments' accounts decode with this repository's readers: the adapter's state through
//! anoma-pa-solana-client and the SPL token forwarder's config through anomapay-spl-token-forwarder-client. The
//! promotion gates read both; this runs on every pull request and push, so a layout change on chain fails here first.

use anoma_risc0_kind_tables::{SolanaCluster, deployments};
use anoma_risc0_kind_tables_integration_test::solana_rpc;
use anyhow::{Context, Result};

#[tokio::test]
async fn the_recorded_solana_deployments_accounts_decode() -> Result<()> {
    for cluster in [SolanaCluster::Devnet, SolanaCluster::MainnetBeta] {
        let Some(deployment) = deployments::solana_deployment(cluster) else {
            continue;
        };
        let rpc = solana_rpc(cluster);
        let commitment = rpc
            .adapter_kind_table_commitment(&deployment.adapter)
            .await?
            .with_context(|| {
                format!(
                    "{}: the adapter {} is not initialized",
                    cluster.name(),
                    deployment.adapter
                )
            })?;
        let logic_ref = rpc
            .forwarder_logic_ref(&deployment.forwarder)
            .await?
            .with_context(|| {
                format!(
                    "{}: the forwarder {} is not initialized",
                    cluster.name(),
                    deployment.forwarder
                )
            })?;
        eprintln!(
            "{}: adapter stores {commitment}, forwarder accepts {logic_ref}",
            cluster.name()
        );
    }
    Ok(())
}
