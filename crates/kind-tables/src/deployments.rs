//! The Solana deployment record: the SPL token forwarder program of each recorded cluster, whose label the SPL
//! token members carry. anoma-pa-solana-client records the devnet deployment as its program ids; a mainnet-beta
//! record appears there when V2 deploys to it, and this reads it then.
use crate::chain::SolanaCluster;
use crate::solana::SolanaAddress;

/// The SPL token forwarder program recorded for the cluster, if V2 is deployed there.
pub fn solana_forwarder(cluster: SolanaCluster) -> Option<SolanaAddress> {
    match cluster {
        SolanaCluster::Devnet => Some(SolanaAddress::new(
            anoma_pa_solana_client::FORWARDER_PROGRAM_ID.to_bytes(),
        )),
        SolanaCluster::MainnetBeta => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devnet_records_the_v2_forwarder_and_mainnet_beta_none() {
        assert_eq!(
            solana_forwarder(SolanaCluster::Devnet).unwrap().to_string(),
            "5CrHbBeHjg53UyL3Htn9dCYYTy68fMcrbDoeAdo4yQrx"
        );
        assert_eq!(solana_forwarder(SolanaCluster::MainnetBeta), None);
    }
}
