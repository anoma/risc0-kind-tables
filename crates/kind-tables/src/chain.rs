//! The chain a table belongs to, named by its CAIP-2 chain ID: a namespace and a reference joined by a colon, such as
//! `eip155:11155111` for Sepolia. It names a chain of any family, where an EIP-155 chain ID names an EVM chain only. A
//! Solana cluster's reference is the first 32 characters of its base58 genesis hash, which every tool derives from the
//! cluster the same way (https://github.com/ChainAgnostic/namespaces/blob/main/solana/caip2.md).

use crate::error::{Error, Result};
use alloy_chains::NamedChain;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// A CAIP-2 chain ID. The namespace is 3 to 8 characters of `a-z`, `0-9` and `-`. The reference is 1 to 32 characters
/// of `a-z`, `A-Z`, `0-9`, `-` and `_`. An `eip155` reference is the decimal chain ID without a leading zero, so that
/// one chain has one ID.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Caip2ChainId {
    namespace: String,
    reference: String,
}

impl Caip2ChainId {
    /// The namespace of the EVM chains, whose reference is the EIP-155 chain ID.
    pub const EIP155: &'static str = "eip155";

    /// The namespace of the Solana clusters, whose reference is the start of the genesis hash.
    pub const SOLANA: &'static str = "solana";

    /// The ID of the EVM chain with this EIP-155 chain ID.
    pub fn eip155(chain_id: u64) -> Self {
        Self {
            namespace: Self::EIP155.to_string(),
            reference: chain_id.to_string(),
        }
    }

    /// The chain family, such as `eip155` or `solana`.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// The chain within its family.
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// The EIP-155 chain ID, if the chain is an EVM chain.
    pub fn eip155_chain_id(&self) -> Option<u64> {
        if self.namespace == Self::EIP155 {
            self.reference.parse().ok()
        } else {
            None
        }
    }

    /// The chain's name, if this crate knows the chain: an EVM chain `alloy_chains` names, or a Solana cluster. The
    /// data loaders reject a chain without one.
    pub fn name(&self) -> Option<&'static str> {
        Chain::try_from(self).ok().map(Chain::name)
    }

    /// The file name of the chain's generated table. `cargo package` refuses a colon in a file name, so the colon
    /// becomes `_`. No namespace contains `_`, so the first `_` separates the namespace from the reference.
    pub fn file_name(&self) -> String {
        format!("{}_{}.json", self.namespace, self.reference)
    }
}

impl FromStr for Caip2ChainId {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        let invalid = |reason| Error::InvalidCaip2ChainId {
            value: value.to_string(),
            reason,
        };
        let (namespace, reference) = value
            .split_once(':')
            .ok_or_else(|| invalid("no colon separates the namespace from the reference"))?;
        if !(3..=8).contains(&namespace.len())
            || !namespace
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(invalid(
                "the namespace is not 3 to 8 characters of a-z, 0-9 and -",
            ));
        }
        if !(1..=32).contains(&reference.len())
            || !reference
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err(invalid(
                "the reference is not 1 to 32 characters of a-z, A-Z, 0-9, - and _",
            ));
        }
        if namespace == Self::EIP155
            && !reference
                .parse::<u64>()
                .is_ok_and(|chain_id| chain_id.to_string() == reference)
        {
            return Err(invalid(
                "an eip155 reference is not a decimal chain ID without a leading zero",
            ));
        }
        Ok(Self {
            namespace: namespace.to_string(),
            reference: reference.to_string(),
        })
    }
}

impl fmt::Display for Caip2ChainId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace, self.reference)
    }
}

impl Serialize for Caip2ChainId {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Caip2ChainId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

impl From<NamedChain> for Caip2ChainId {
    fn from(chain: NamedChain) -> Self {
        Self::eip155(chain as u64)
    }
}

impl TryFrom<&Caip2ChainId> for NamedChain {
    type Error = Error;

    fn try_from(chain: &Caip2ChainId) -> Result<Self> {
        chain
            .eip155_chain_id()
            .and_then(|chain_id| NamedChain::try_from(chain_id).ok())
            .ok_or_else(|| Error::UnknownChain(chain.clone()))
    }
}

/// A Solana cluster, named by its CAIP-2 chain ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SolanaCluster {
    MainnetBeta,
    Devnet,
}

impl SolanaCluster {
    const ALL: [Self; 2] = [Self::MainnetBeta, Self::Devnet];

    /// The reference of the cluster's CAIP-2 chain ID: the first 32 characters of the cluster's genesis hash.
    pub const fn reference(self) -> &'static str {
        match self {
            Self::MainnetBeta => "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp",
            Self::Devnet => "EtWTRABZaYq6iMfeYKouRu166VU2xqa1",
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

impl From<SolanaCluster> for Caip2ChainId {
    fn from(cluster: SolanaCluster) -> Self {
        Self {
            namespace: Self::SOLANA.to_string(),
            reference: cluster.reference().to_string(),
        }
    }
}

impl TryFrom<&Caip2ChainId> for SolanaCluster {
    type Error = Error;

    fn try_from(chain: &Caip2ChainId) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|cluster| Caip2ChainId::from(*cluster) == *chain)
            .ok_or_else(|| Error::UnknownChain(chain.clone()))
    }
}

/// A chain this crate knows, by family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Chain {
    Evm(NamedChain),
    Solana(SolanaCluster),
}

impl Chain {
    /// The chain's name, for review context.
    pub fn name(self) -> &'static str {
        match self {
            Self::Evm(chain) => chain.as_str(),
            Self::Solana(cluster) => cluster.name(),
        }
    }
}

impl TryFrom<&Caip2ChainId> for Chain {
    type Error = Error;

    fn try_from(chain: &Caip2ChainId) -> Result<Self> {
        NamedChain::try_from(chain)
            .map(Self::Evm)
            .or_else(|_| SolanaCluster::try_from(chain).map(Self::Solana))
    }
}

impl From<Chain> for Caip2ChainId {
    fn from(chain: Chain) -> Self {
        match chain {
            Chain::Evm(chain) => chain.into(),
            Chain::Solana(cluster) => cluster.into(),
        }
    }
}

impl fmt::Display for Chain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    const SOLANA_MAINNET: &str = "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp";

    fn parse(value: &str) -> Result<Caip2ChainId> {
        value.parse()
    }

    #[test]
    fn from_str_parses_an_eip155_chain_id() {
        let sepolia = parse("eip155:11155111").unwrap();

        assert_eq!(sepolia, Caip2ChainId::eip155(11_155_111));
        assert_eq!(sepolia, Caip2ChainId::from(NamedChain::Sepolia));
        assert_eq!(sepolia.to_string(), "eip155:11155111");
        assert_eq!(sepolia.eip155_chain_id(), Some(11_155_111));
        assert_eq!(sepolia.name(), Some("sepolia"));
        assert_eq!(NamedChain::try_from(&sepolia).unwrap(), NamedChain::Sepolia);
    }

    #[test]
    fn from_str_parses_a_solana_chain_id() {
        let solana = parse(SOLANA_MAINNET).unwrap();

        assert_eq!(solana.namespace(), "solana");
        assert_eq!(solana.reference(), "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp");
        assert_eq!(solana.to_string(), SOLANA_MAINNET);
        assert_eq!(solana.eip155_chain_id(), None);
        assert!(matches!(
            NamedChain::try_from(&solana),
            Err(Error::UnknownChain(_))
        ));
    }

    #[test]
    fn from_str_rejects_a_malformed_chain_id() {
        for value in [
            "11155111",
            "ab:1",
            "namespace9:1",
            "EIP155:1",
            "eip155:",
            "cosmos:a.b",
            "cosmos:123456789012345678901234567890123",
            "eip155:011155111",
            "eip155:sepolia",
            "eip155:18446744073709551616",
        ] {
            assert!(
                matches!(parse(value), Err(Error::InvalidCaip2ChainId { .. })),
                "{value} parses"
            );
        }
    }

    #[test]
    fn name_is_none_for_an_evm_chain_alloy_does_not_name() {
        assert_eq!(Caip2ChainId::eip155(u64::MAX).name(), None);
    }

    #[test]
    fn file_name_replaces_the_colon() {
        assert_eq!(
            Caip2ChainId::eip155(11_155_111).file_name(),
            "eip155_11155111.json"
        );
        assert_eq!(
            parse(SOLANA_MAINNET).unwrap().file_name(),
            "solana_5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp.json"
        );
    }

    #[test]
    fn a_json_map_key_round_trips() {
        let map = BTreeMap::from([
            (parse(SOLANA_MAINNET).unwrap(), 1),
            (Caip2ChainId::eip155(1), 2),
        ]);
        let json = serde_json::to_string(&map).unwrap();

        assert_eq!(json, format!(r#"{{"eip155:1":2,"{SOLANA_MAINNET}":1}}"#));
        assert_eq!(
            serde_json::from_str::<BTreeMap<Caip2ChainId, u8>>(&json).unwrap(),
            map
        );
    }

    #[test]
    fn deserialize_rejects_a_malformed_json_key() {
        assert!(serde_json::from_str::<BTreeMap<Caip2ChainId, u8>>(r#"{"11155111":1}"#).is_err());
    }

    #[test]
    fn a_solana_cluster_is_named_by_its_caip2_chain_id() {
        let devnet = parse("solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1").unwrap();

        assert_eq!(
            SolanaCluster::try_from(&devnet).unwrap(),
            SolanaCluster::Devnet
        );
        assert_eq!(
            Chain::try_from(&devnet).unwrap(),
            Chain::Solana(SolanaCluster::Devnet)
        );
        assert_eq!(devnet.name(), Some("solana-devnet"));
        assert_eq!(
            devnet.file_name(),
            "solana_EtWTRABZaYq6iMfeYKouRu166VU2xqa1.json"
        );
        assert_eq!(
            Chain::try_from(&parse(SOLANA_MAINNET).unwrap()).unwrap(),
            Chain::Solana(SolanaCluster::MainnetBeta)
        );
        assert_eq!(Caip2ChainId::from(SolanaCluster::Devnet), devnet);
        assert_eq!(
            Caip2ChainId::from(SolanaCluster::MainnetBeta),
            parse(SOLANA_MAINNET).unwrap()
        );
    }

    #[test]
    fn an_unknown_chain_fails_loudly() {
        assert!(Chain::try_from(&Caip2ChainId::eip155(u64::MAX)).is_err());
        assert!(Chain::try_from(&parse("solana:nope").unwrap()).is_err());
        assert!(parse("sepolia").is_err());
    }

    #[test]
    fn evm_chains_sort_before_solana_clusters() {
        assert!(
            Caip2ChainId::from(NamedChain::Mainnet)
                < Caip2ChainId::from(SolanaCluster::MainnetBeta)
        );
        assert!(
            Caip2ChainId::from(SolanaCluster::MainnetBeta)
                < Caip2ChainId::from(SolanaCluster::Devnet)
        );
    }
}
