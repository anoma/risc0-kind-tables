use alloy_chains::NamedChain;
use anoma_risc0_kind_tables::chain::*;
use anoma_risc0_kind_tables::{Error, Result};
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
        Caip2ChainId::from(NamedChain::Mainnet) < Caip2ChainId::from(SolanaCluster::MainnetBeta)
    );
    assert!(
        Caip2ChainId::from(SolanaCluster::MainnetBeta) < Caip2ChainId::from(SolanaCluster::Devnet)
    );
}
