//! The Solana deployment record: the protocol adapter each recorded cluster's table is installed on, and the SPL
//! token forwarder program whose label the SPL token members carry. anoma-pa-solana-client records the devnet
//! deployment as its program ids; a mainnet-beta record appears there when V2 deploys to it, and this reads it
//! then.
use crate::chain::SolanaCluster;
use crate::solana::SolanaAddress;

/// A cluster's V2 deployment: the protocol adapter program and the SPL token forwarder program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SolanaDeployment {
    pub adapter: SolanaAddress,
    pub forwarder: SolanaAddress,
}

/// The V2 deployment recorded for the cluster, if there is one.
pub fn solana_deployment(cluster: SolanaCluster) -> Option<SolanaDeployment> {
    match cluster {
        SolanaCluster::Devnet => Some(SolanaDeployment {
            adapter: SolanaAddress::new(anoma_pa_solana_client::PA_PROGRAM_ID.to_bytes()),
            forwarder: SolanaAddress::new(anoma_pa_solana_client::FORWARDER_PROGRAM_ID.to_bytes()),
        }),
        SolanaCluster::MainnetBeta => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devnet_records_the_v2_programs_and_mainnet_beta_none() {
        let devnet = solana_deployment(SolanaCluster::Devnet).unwrap();
        assert_eq!(
            devnet.adapter.to_string(),
            "28Hvr1YFv2ouGN2fS99aF3ZzYXzkncJVVaHcZNhquLFT"
        );
        assert_eq!(
            devnet.forwarder.to_string(),
            "5CrHbBeHjg53UyL3Htn9dCYYTy68fMcrbDoeAdo4yQrx"
        );
        assert_eq!(solana_deployment(SolanaCluster::MainnetBeta), None);
    }
}
