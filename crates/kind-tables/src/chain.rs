//! The chain a table belongs to, keyed as the chain identifies itself. An EVM chain is keyed by its EIP-155
//! chain id, the number every EVM tool agrees on. A Solana cluster has no chain id; it is keyed by its CAIP-2
//! chain id, the first 32 characters of its base58 genesis hash, which every tool derives from the cluster the
//! same way (https://github.com/ChainAgnostic/namespaces/blob/main/solana/caip2.md). Both are what the
//! chain-keyed files (`tokens.json`, `commitments.json`) use as keys and what a generated table is named for.
use crate::error::{Error, Result};
use alloy_chains::NamedChain;
use std::fmt;

/// A Solana cluster, identified by its CAIP-2 chain id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SolanaCluster {
    MainnetBeta,
    Devnet,
}

impl SolanaCluster {
    const ALL: [Self; 2] = [Self::MainnetBeta, Self::Devnet];

    /// The CAIP-2 chain id: `solana:` and the first 32 characters of the cluster's genesis hash.
    pub const fn caip2(self) -> &'static str {
        match self {
            Self::MainnetBeta => "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp",
            Self::Devnet => "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1",
        }
    }

    /// The cluster's name, for review context.
    pub const fn name(self) -> &'static str {
        match self {
            Self::MainnetBeta => "solana-mainnet-beta",
            Self::Devnet => "solana-devnet",
        }
    }
}

/// A chain a table is generated for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Chain {
    Evm(NamedChain),
    Solana(SolanaCluster),
}

impl Chain {
    /// Parses a chain-keyed file's key: an EIP-155 chain id, or a Solana CAIP-2 chain id.
    pub fn from_key(key: &str) -> Result<Self> {
        if let Ok(id) = key.parse::<u64>() {
            return NamedChain::try_from(id)
                .map(Self::Evm)
                .map_err(|_| Error::UnknownChain(key.to_string()));
        }
        SolanaCluster::ALL
            .into_iter()
            .find(|cluster| cluster.caip2() == key)
            .map(Self::Solana)
            .ok_or_else(|| Error::UnknownChain(key.to_string()))
    }

    /// The key the chain-keyed files use.
    pub fn key(&self) -> String {
        match self {
            Self::Evm(chain) => (*chain as u64).to_string(),
            Self::Solana(cluster) => cluster.caip2().to_string(),
        }
    }

    /// The stem of the chain's generated table file: the key, with the CAIP-2 namespace separator made
    /// filename-safe.
    pub fn file_stem(&self) -> String {
        self.key().replace(':', "-")
    }
}

impl fmt::Display for Chain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evm(chain) => chain.fmt(f),
            Self::Solana(cluster) => f.write_str(cluster.name()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_chains::NamedChain;

    #[test]
    fn an_evm_chain_is_keyed_by_its_chain_id() {
        let chain = Chain::from_key("11155111").unwrap();
        assert_eq!(chain, Chain::Evm(NamedChain::Sepolia));
        assert_eq!(chain.key(), "11155111");
        assert_eq!(chain.file_stem(), "11155111");
        assert_eq!(chain.to_string(), "sepolia");
    }

    #[test]
    fn a_solana_cluster_is_keyed_by_its_caip2_chain_id() {
        let devnet = Chain::from_key("solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1").unwrap();
        assert_eq!(devnet, Chain::Solana(SolanaCluster::Devnet));
        assert_eq!(devnet.key(), "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1");
        assert_eq!(
            devnet.file_stem(),
            "solana-EtWTRABZaYq6iMfeYKouRu166VU2xqa1"
        );
        assert_eq!(devnet.to_string(), "solana-devnet");
        assert_eq!(
            Chain::from_key("solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp").unwrap(),
            Chain::Solana(SolanaCluster::MainnetBeta)
        );
    }

    #[test]
    fn the_caip2_id_is_the_genesis_hash_prefix() {
        assert_eq!(
            SolanaCluster::Devnet.caip2(),
            "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1"
        );
        assert_eq!(
            SolanaCluster::MainnetBeta.caip2(),
            "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp"
        );
    }

    #[test]
    fn an_unknown_key_fails_loudly() {
        assert!(Chain::from_key("18446744073709551615").is_err());
        assert!(Chain::from_key("solana:nope").is_err());
        assert!(Chain::from_key("sepolia").is_err());
    }

    #[test]
    fn evm_chains_sort_before_solana_clusters() {
        assert!(Chain::Evm(NamedChain::Mainnet) < Chain::Solana(SolanaCluster::MainnetBeta));
        assert!(Chain::Solana(SolanaCluster::MainnetBeta) < Chain::Solana(SolanaCluster::Devnet));
    }
}
