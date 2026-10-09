use super::*;

#[test]
fn devnet_records_the_v2_programs_and_mainnet_beta_none() {
    let devnet = solana_deployment(SolanaCluster::Devnet).unwrap();
    assert_eq!(
        devnet.adapter.to_string(),
        "5zeqkB3kc9fd1RvaXB2GeMB53Jgf98QJtaFK38e6tTsc"
    );
    assert_eq!(
        devnet.forwarder.to_string(),
        "BsfuXpxw8oCmZXnYijyQkUYNcCnuskFZbYizmWLnpSU7"
    );
    assert_eq!(solana_deployment(SolanaCluster::MainnetBeta), None);
}
