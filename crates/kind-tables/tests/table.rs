use alloy_chains::NamedChain;
use anoma_risc0_kind_tables::chain::{Chain, SolanaCluster};
use anoma_risc0_kind_tables::circuits::{self, CircuitVersion, Status};
use anoma_risc0_kind_tables::deployments::solana_deployment;
use anoma_risc0_kind_tables::entry::Metadata;
use anoma_risc0_kind_tables::kind;
use anoma_risc0_kind_tables::table::*;
use anoma_risc0_kind_tables::{Caip2ChainId, Entry, Error};
use anomapay_erc20_forwarder_bindings::addresses::{Environment, erc20_forwarder_address};
use risc0_zkvm::Digest;

/// The alias rules of a fungibility domain, checked on one member: it is assigned the domain's kind point
/// (the kind of the active version under the current forwarder's label), its version is in `listed`, it is
/// active if and only if it has no `alias_of`, and an `alias_of` names the active version under that label.
fn check_member(
    context: &str,
    entry: &Entry,
    listed: &[CircuitVersion],
    active: &CircuitVersion,
    active_label: Digest,
    status: Status,
) {
    let domain = kind::point(&active.logic_ref, &active_label).expect("a kind derives");
    assert_eq!(
        entry.kind_point, domain,
        "{context}: a member is not assigned its fungibility domain's kind point"
    );
    assert!(
        listed
            .iter()
            .any(|circuit| circuit.logic_ref == entry.logic_ref),
        "{context}: a member of an unlisted version"
    );
    let alias_of = entry.metadata.as_ref().and_then(Metadata::alias_of);
    assert_eq!(
        status == Status::Active,
        alias_of.is_none(),
        "{context}: a member is active if and only if it has no alias_of"
    );
    match alias_of {
        None => {
            assert!(
                !entry.is_alias(),
                "{context}: an entry without alias_of is an alias"
            );
            assert_eq!(
                (entry.logic_ref, entry.label_ref),
                (active.logic_ref, active_label),
                "{context}: only the active version under the current forwarder has no alias_of"
            );
        }
        Some(alias_of) => {
            assert!(
                entry.is_alias(),
                "{context}: an entry with alias_of is not an alias"
            );
            assert_eq!(
                (&alias_of.version, alias_of.logic_ref, alias_of.label_ref),
                (&active.version, active.logic_ref, active_label),
                "{context}: alias_of does not name the active version under the current forwarder"
            );
        }
    }
}

/// No kind point is authored. A token entry is assigned its fungibility domain's kind point — the kind of the
/// active version under the label of the chain's current forwarder — and every alias says so in `alias_of`.
/// Every other entry is assigned its own kind. This reads the embedded tables only; the current forwarder comes
/// from the deployment records (the ERC20 forwarder bindings, and the Solana deployment record), never from the
/// entries themselves, so a table built for the wrong forwarder fails here.
#[test]
fn token_entries_share_their_fungibility_domains_kind_point() {
    for (module, environment, tables) in [
        ("staging", Environment::Staging, staging::tables()),
        ("production", Environment::Production, production::tables()),
    ] {
        for (chain, table) in tables {
            let context = format!("{module} {chain}");
            let known = Chain::try_from(chain).unwrap_or_else(|error| panic!("{context}: {error}"));
            let (erc20_forwarder, spl_forwarder) = match known {
                Chain::Evm(named) => (erc20_forwarder_address(environment, &named), None),
                Chain::Solana(cluster) => (
                    None,
                    solana_deployment(cluster).map(|deployment| deployment.forwarder),
                ),
            };
            for entry in &table.entries {
                let (listed, active, active_label, status) = match &entry.metadata {
                    Some(Metadata::Erc20 { token, status, .. }) => {
                        let forwarder = erc20_forwarder.unwrap_or_else(|| {
                            panic!("{context}: an ERC20 entry but no ERC20 forwarder recorded")
                        });
                        (
                            circuits::erc20(),
                            circuits::erc20_active(),
                            kind::erc20_label_ref(&forwarder, token),
                            *status,
                        )
                    }
                    Some(Metadata::SplToken { mint, status, .. }) => {
                        let forwarder = spl_forwarder.unwrap_or_else(|| {
                            panic!(
                                "{context}: an SPL token entry but no Solana deployment recorded"
                            )
                        });
                        (
                            circuits::spl_token(),
                            circuits::spl_token_active(),
                            kind::spl_token_label_ref(&forwarder, mint),
                            *status,
                        )
                    }
                    Some(Metadata::GenericCall { .. }) | None => {
                        assert!(
                            !entry.is_alias(),
                            "{context}: an entry outside every fungibility domain is not assigned its own kind"
                        );
                        continue;
                    }
                };
                check_member(&context, entry, listed, active, active_label, status);
            }
        }
    }
}

/// A token's active kind has a row if and only if the token asks for a precomputed kind point. Without the row, the
/// circuit computes the same kind point by hash to curve.
#[test]
fn a_token_has_an_active_row_if_and_only_if_it_asks_for_a_precomputed_kind_point() {
    let mut checked = 0;
    for (module, environment, tables) in [
        ("staging", Environment::Staging, staging::tables()),
        ("production", Environment::Production, production::tables()),
    ] {
        for (chain, table) in tables {
            let context = format!("{module} {chain}");
            let known = Chain::try_from(chain).unwrap_or_else(|error| panic!("{context}: {error}"));
            // (symbol, active kind, whether the token asks for a precomputed kind point)
            let tokens: Vec<(&str, (Digest, Digest), bool)> = match known {
                Chain::Evm(named) => match erc20_forwarder_address(environment, &named) {
                    Some(current) => anoma_risc0_kind_tables::tokens::on(named)
                        .iter()
                        .map(|token| {
                            let label_ref = kind::erc20_label_ref(&current, &token.address);
                            let active = (circuits::erc20_active().logic_ref, label_ref);
                            (token.symbol.as_str(), active, token.precompute_kind_point)
                        })
                        .collect(),
                    None => Vec::new(),
                },
                Chain::Solana(cluster) => match solana_deployment(cluster) {
                    Some(deployment) => anoma_risc0_kind_tables::tokens::spl_on(cluster)
                        .iter()
                        .map(|token| {
                            let label_ref =
                                kind::spl_token_label_ref(&deployment.forwarder, &token.mint);
                            let active = (circuits::spl_token_active().logic_ref, label_ref);
                            (token.symbol.as_str(), active, token.precompute_kind_point)
                        })
                        .collect(),
                    None => Vec::new(),
                },
            };
            for (symbol, active, asks) in tokens {
                let listed = table
                    .entries
                    .iter()
                    .any(|entry| (entry.logic_ref, entry.label_ref) == active);
                assert_eq!(
                    listed, asks,
                    "{context}: {symbol} has an active row if and only if it asks for a precomputed kind point"
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 0, "the tables carry a token to check");
}

/// An EVM consumer passes the chain it already has, so the lookups accept a `NamedChain`.
#[test]
fn table_accepts_an_evm_chain() {
    let sepolia = Caip2ChainId::eip155(11_155_111);

    assert_eq!(
        staging::table(NamedChain::Sepolia).unwrap(),
        staging::table(sepolia.clone()).unwrap()
    );
    assert_eq!(
        staging::commitment(NamedChain::Sepolia).unwrap(),
        staging::commitment(sepolia).unwrap()
    );
    assert!(matches!(
        staging::table(NamedChain::Mainnet),
        Err(Error::UnrecordedChain(chain)) if chain == Caip2ChainId::eip155(1)
    ));
}

/// The staging environment records the solana-devnet table: the test mint's active member under the devnet
/// forwarder's label, and nothing else.
#[test]
fn staging_records_the_solana_devnet_table() {
    let devnet = SolanaCluster::Devnet;
    let table = staging::table(devnet).expect("solana-devnet is recorded in staging");
    let [member] = &table.entries[..] else {
        panic!("expected exactly one entry, got {}", table.entries.len());
    };
    assert!(
        matches!(member.metadata, Some(Metadata::SplToken { .. })),
        "the entry is an SPL token member"
    );
    assert_eq!(
        member.label_ref.to_string(),
        "6b883f562948fd8812d0c3c26559b237088850f957283cb777c59130da1f7c17",
        "the label is sha256(devnet forwarder ‖ test mint)"
    );
    assert_eq!(member.logic_ref, circuits::spl_token_active().logic_ref);
    assert!(!member.is_alias(), "the active version keeps its own kind");
    assert_eq!(staging::commitment(devnet).unwrap(), table.commitment());
}

/// The macro invocation lists the table files by hand, so pin it to `commitments.json`.
#[test]
fn embedded_tables_match_the_recorded_commitments() {
    for (module, commitments, tables) in [
        ("staging", staging::commitments(), staging::tables()),
        (
            "production",
            production::commitments(),
            production::tables(),
        ),
    ] {
        let recorded: Vec<&Caip2ChainId> = commitments.keys().collect();
        let embedded: Vec<&Caip2ChainId> = tables.keys().collect();
        assert_eq!(recorded, embedded, "{module}: chains out of sync");
        for (chain, commitment) in commitments {
            assert_eq!(
                tables[chain].commitment(),
                *commitment,
                "{module}: stale table for {chain}"
            );
        }
    }
}
