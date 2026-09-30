//! Generated chain tables and their commitments, one set per environment. A chain's table lives in
//! `data/generated/<environment>/<namespace>_<reference>.json`, named for the chain's CAIP-2 chain ID; the tables and
//! commitments are embedded and looked up per chain.

use crate::chain::Caip2ChainId;
use crate::commitment;
use crate::entry::Entry;
use crate::error::{Error, Result};
use risc0_zkvm::Digest;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// The ordered entries of one chain's kind table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table {
    pub entries: Vec<Entry>,
}

impl Table {
    /// Parses a table from its JSON representation: an array of entries.
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(Self {
            entries: serde_json::from_str(json)?,
        })
    }

    /// Reads a table from a generated `<namespace>_<reference>.json` file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_json(&std::fs::read_to_string(path)?)
    }

    /// The commitment over the entries in their table order.
    pub fn commitment(&self) -> Digest {
        commitment::of(&self.entries)
    }

    /// Whether the entries are sorted by `logic_ref ‖ label_ref`, as the generator emits them.
    pub fn is_sorted(&self) -> bool {
        self.entries
            .windows(2)
            .all(|pair| pair[0].key() < pair[1].key())
    }

    /// The aliases: the entries assigned another kind as their kind point. Only the table can express them, and they
    /// are the review surface.
    pub fn aliases(&self) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|entry| entry.is_alias())
            .collect()
    }
}

/// One chain's recorded commitment. The `_comment` naming the chain is review context and is not deserialized.
#[derive(Deserialize)]
struct ChainCommitment {
    commitment: String,
}

fn parse_commitments(json: &str) -> BTreeMap<Caip2ChainId, Digest> {
    use hex::FromHex;
    let raw: BTreeMap<Caip2ChainId, ChainCommitment> =
        serde_json::from_str(json).expect("commitments.json: invalid JSON");
    raw.into_iter()
        .map(|(chain, recorded)| {
            // A chain that fails to resolve must fail loudly: dropping it would erase its freshness assertion.
            assert!(chain.name().is_some(), "unknown chain: {chain}");
            let digest = Digest::from_hex(&recorded.commitment)
                .unwrap_or_else(|_| panic!("invalid commitment for {chain}"));
            (chain, digest)
        })
        .collect()
}

/// Embeds one environment's commitments and tables. Each chain is listed as the namespace and reference of its CAIP-2
/// chain ID, from which the macro builds both the ID and the table's file name.
macro_rules! environment_module {
    ($name:ident $(, ($namespace:literal, $reference:literal))*) => {
        pub mod $name {
            use super::*;
            use std::sync::LazyLock;

            static COMMITMENTS: LazyLock<BTreeMap<Caip2ChainId, Digest>> = LazyLock::new(|| {
                parse_commitments(include_str!(concat!(
                    "../data/generated/",
                    stringify!($name),
                    "/commitments.json"
                )))
            });

            static TABLES: LazyLock<BTreeMap<Caip2ChainId, Table>> = LazyLock::new(|| {
                let tables: Vec<(Caip2ChainId, Table)> = vec![$((
                    concat!($namespace, ":", $reference)
                        .parse()
                        .unwrap_or_else(|error| panic!("{error}")),
                    Table::from_json(include_str!(concat!(
                        "../data/generated/",
                        stringify!($name),
                        "/",
                        $namespace,
                        "_",
                        $reference,
                        ".json"
                    )))
                    .unwrap_or_else(|error| {
                        panic!("invalid table for {}: {error}", concat!($namespace, ":", $reference))
                    }),
                )),*];
                tables.into_iter().collect()
            });

            /// The chains this environment records a table for.
            pub fn chains() -> Vec<Caip2ChainId> {
                COMMITMENTS.keys().cloned().collect()
            }

            /// The commitments of all recorded chains.
            pub fn commitments() -> &'static BTreeMap<Caip2ChainId, Digest> {
                &COMMITMENTS
            }

            /// The commitment recorded for the chain.
            pub fn commitment(chain: impl Into<Caip2ChainId>) -> Result<Digest> {
                let chain = chain.into();
                COMMITMENTS
                    .get(&chain)
                    .copied()
                    .ok_or(Error::UnrecordedChain(chain))
            }

            /// The tables of all recorded chains.
            pub fn tables() -> &'static BTreeMap<Caip2ChainId, Table> {
                &TABLES
            }

            /// The table recorded for the chain.
            pub fn table(chain: impl Into<Caip2ChainId>) -> Result<&'static Table> {
                let chain = chain.into();
                TABLES.get(&chain).ok_or(Error::UnrecordedChain(chain))
            }
        }
    };
}

environment_module!(staging, ("eip155", "11155111"));
environment_module!(production);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::Metadata;
    use crate::kind;
    use alloy_chains::NamedChain;
    use anomapay_erc20_forwarder_bindings::addresses::{Environment, erc20_forwarder_address};

    /// No kind point is authored. An ERC20 entry is assigned its fungibility domain's kind point — the kind of the
    /// active version under the label of the chain's current forwarder — and every alias says so in `alias_of`.
    /// Every other entry is assigned its own kind. This reads the embedded tables only.
    #[test]
    fn erc20_entries_share_their_fungibility_domains_kind_point() {
        for (module, environment, tables) in [
            ("staging", Environment::Staging, staging::tables()),
            ("production", Environment::Production, production::tables()),
        ] {
            for (chain, table) in tables {
                let evm_chain =
                    NamedChain::try_from(chain).expect("every recorded chain is an EVM chain");
                let current = erc20_forwarder_address(environment, &evm_chain);
                for entry in &table.entries {
                    let Some(Metadata::Erc20 { token, status, .. }) = &entry.metadata else {
                        assert!(
                            !entry.is_alias(),
                            "{module} {chain}: an entry outside every fungibility domain is not assigned its own kind"
                        );
                        continue;
                    };
                    let current = current.unwrap_or_else(|| {
                        panic!("{module} {chain}: an ERC20 entry but no forwarder recorded")
                    });
                    let active = crate::circuits::erc20_active();
                    let active_label = kind::erc20_label_ref(&current, token);
                    let domain =
                        kind::point(&active.logic_ref, &active_label).expect("a kind derives");
                    assert_eq!(
                        entry.kind_point, domain,
                        "{module} {chain}: an ERC20 entry is not assigned its fungibility domain's kind point"
                    );
                    assert!(
                        crate::circuits::erc20_version(&entry.logic_ref).is_some(),
                        "{module} {chain}: an ERC20 entry of an unlisted version"
                    );
                    let alias_of = entry.metadata.as_ref().and_then(Metadata::alias_of);
                    assert_eq!(
                        *status == crate::circuits::Status::Active,
                        alias_of.is_none(),
                        "{module} {chain}: a member is active if and only if it has no alias_of"
                    );
                    match alias_of {
                        None => {
                            assert!(
                                !entry.is_alias(),
                                "{module} {chain}: an entry without alias_of is an alias"
                            );
                            assert_eq!(
                                (entry.logic_ref, entry.label_ref),
                                (active.logic_ref, active_label),
                                "{module} {chain}: only the active version under the current forwarder has no alias_of"
                            );
                        }
                        Some(alias_of) => {
                            assert!(
                                entry.is_alias(),
                                "{module} {chain}: an entry with alias_of is not an alias"
                            );
                            assert_eq!(
                                (&alias_of.version, alias_of.logic_ref, alias_of.label_ref),
                                (&active.version, active.logic_ref, active_label),
                                "{module} {chain}: alias_of does not name the active version under the current forwarder"
                            );
                        }
                    }
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
                let evm_chain =
                    NamedChain::try_from(chain).expect("every recorded chain is an EVM chain");
                let Some(current) = erc20_forwarder_address(environment, &evm_chain) else {
                    continue; // No ERC20 fungibility domain on this chain.
                };
                for token in crate::tokens::on(evm_chain) {
                    let active = (
                        crate::circuits::erc20_active().logic_ref,
                        kind::erc20_label_ref(&current, &token.address),
                    );
                    let listed = table
                        .entries
                        .iter()
                        .any(|entry| (entry.logic_ref, entry.label_ref) == active);
                    assert_eq!(
                        listed, token.precompute_kind_point,
                        "{context}: {} has an active row if and only if it asks for a precomputed kind point",
                        token.symbol
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
}
