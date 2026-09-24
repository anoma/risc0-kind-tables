//! A Solana cluster is keyed by its CAIP-2 chain id, the first 32 characters of its genesis hash. The key of every
//! recorded Solana cluster is checked against the genesis hash the cluster reports, so a mistyped key can never
//! name a table. Runs on every pull request and push.
use anoma_risc0_kind_tables::{Chain, SolanaCluster, tokens};
use anoma_risc0_kind_tables_integration_test::solana_rpc;
use anyhow::{Result, ensure};

#[tokio::test]
async fn every_recorded_cluster_key_is_its_genesis_hash_prefix() -> Result<()> {
    let recorded: Vec<SolanaCluster> = tokens::all()
        .keys()
        .filter_map(|chain| match chain {
            Chain::Solana(cluster) => Some(*cluster),
            Chain::Evm(_) => None,
        })
        .collect();
    ensure!(
        !recorded.is_empty(),
        "no Solana cluster is recorded in tokens.json"
    );
    for cluster in recorded {
        let genesis_hash = solana_rpc(cluster).genesis_hash().await?;
        let expected = format!("solana:{}", &genesis_hash[..32]);
        ensure!(
            cluster.caip2() == expected,
            "{}: the key is {}, the cluster's genesis hash {genesis_hash} gives {expected}",
            cluster.name(),
            cluster.caip2()
        );
    }
    Ok(())
}
