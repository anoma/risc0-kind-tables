//! Generated chain tables and their commitments, one set per environment. A chain's table lives in
//! `data/generated/<environment>/<chain id>.json`; the tables and commitments are embedded and looked up per chain.

use crate::chain::Chain;
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

    /// Reads a table from a generated `<chain>.json` file.
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

fn parse_commitments(json: &str) -> BTreeMap<Chain, Digest> {
    use hex::FromHex;
    let raw: BTreeMap<String, ChainCommitment> =
        serde_json::from_str(json).expect("commitments.json: invalid JSON");
    raw.into_iter()
        .map(|(key, recorded)| {
            // A chain that fails to resolve must fail loudly: dropping it would erase its freshness assertion.
            let chain =
                Chain::from_key(&key).unwrap_or_else(|error| panic!("commitments.json: {error}"));
            let digest = Digest::from_hex(&recorded.commitment)
                .unwrap_or_else(|_| panic!("invalid commitment for {chain}"));
            (chain, digest)
        })
        .collect()
}

macro_rules! environment_module {
    ($name:ident, $commitments_path:literal $(, ($key:literal, $table_path:literal))*) => {
        pub mod $name {
            use super::*;
            use std::sync::LazyLock;

            static COMMITMENTS: LazyLock<BTreeMap<Chain, Digest>> =
                LazyLock::new(|| parse_commitments(include_str!($commitments_path)));

            static TABLES: LazyLock<BTreeMap<Chain, Table>> = LazyLock::new(|| {
                let tables: Vec<(Chain, Table)> = vec![$((
                    Chain::from_key($key).unwrap_or_else(|error| panic!("{error}")),
                    Table::from_json(include_str!($table_path))
                        .unwrap_or_else(|error| panic!("invalid table for {}: {error}", $key)),
                )),*];
                tables.into_iter().collect()
            });

            /// The chains this environment records a table for.
            pub fn chains() -> Vec<Chain> {
                COMMITMENTS.keys().copied().collect()
            }

            /// The commitments of all recorded chains.
            pub fn commitments() -> &'static BTreeMap<Chain, Digest> {
                &COMMITMENTS
            }

            /// The commitment recorded for the chain.
            pub fn commitment(chain: Chain) -> Result<Digest> {
                COMMITMENTS
                    .get(&chain)
                    .copied()
                    .ok_or(Error::UnrecordedChain(chain))
            }

            /// The tables of all recorded chains.
            pub fn tables() -> &'static BTreeMap<Chain, Table> {
                &TABLES
            }

            /// The table recorded for the chain.
            pub fn table(chain: Chain) -> Result<&'static Table> {
                TABLES.get(&chain).ok_or(Error::UnrecordedChain(chain))
            }
        }
    };
}

environment_module!(
    staging,
    "../data/generated/staging/commitments.json",
    ("11155111", "../data/generated/staging/11155111.json"),
    (
        "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1",
        "../data/generated/staging/solana-EtWTRABZaYq6iMfeYKouRu166VU2xqa1.json"
    )
);
environment_module!(production, "../data/generated/production/commitments.json");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::Metadata;
    use crate::kind;
    use anomapay_erc20_forwarder_bindings::addresses::{Environment, erc20_forwarder_address};

    /// The alias rules of a fungibility domain, checked on one member: it is assigned the domain's kind point
    /// (the kind of the active version under the current forwarder's label), its version is listed, it is active
    /// if and only if it has no `alias_of`, and an `alias_of` names the active version under that label.
    fn check_member(
        context: &str,
        entry: &Entry,
        active: &crate::circuits::CircuitVersion,
        active_label: Digest,
        listed: bool,
        status: crate::circuits::Status,
    ) {
        let domain = kind::point(&active.logic_ref, &active_label).expect("a kind derives");
        assert_eq!(
            entry.kind_point, domain,
            "{context}: a member is not assigned its fungibility domain's kind point"
        );
        assert!(listed, "{context}: a member of an unlisted version");
        let alias_of = entry.metadata.as_ref().and_then(Metadata::alias_of);
        assert_eq!(
            status == crate::circuits::Status::Active,
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
    /// Every other entry is assigned its own kind. This reads the embedded tables only; the ERC20 forwarder comes
    /// from the forwarder bindings, the SPL token forwarder from the entries themselves, since the library carries
    /// no Solana deployment record (the generator and the integration tests check it against the cluster).
    #[test]
    fn token_entries_share_their_fungibility_domains_kind_point() {
        for (module, environment, tables) in [
            ("staging", Environment::Staging, staging::tables()),
            ("production", Environment::Production, production::tables()),
        ] {
            for (chain, table) in tables {
                let context = format!("{module} {chain}");
                let erc20_forwarder = match chain {
                    Chain::Evm(named) => erc20_forwarder_address(environment, named),
                    Chain::Solana(_) => None,
                };
                for entry in &table.entries {
                    let (active, active_label, listed, status) = match &entry.metadata {
                        Some(Metadata::Erc20 { token, status, .. }) => {
                            let forwarder = erc20_forwarder.unwrap_or_else(|| {
                                panic!("{context}: an ERC20 entry but no ERC20 forwarder recorded")
                            });
                            (
                                crate::circuits::erc20_active(),
                                kind::erc20_label_ref(&forwarder, token),
                                crate::circuits::erc20_version(&entry.logic_ref).is_some(),
                                *status,
                            )
                        }
                        Some(Metadata::SplToken {
                            mint,
                            forwarder,
                            status,
                            ..
                        }) => {
                            assert!(
                                matches!(chain, Chain::Solana(_)),
                                "{context}: an SPL token entry on an EVM chain"
                            );
                            (
                                crate::circuits::spl_token_active(),
                                kind::spl_token_label_ref(forwarder, mint),
                                crate::circuits::spl_token_version(&entry.logic_ref).is_some(),
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
                    check_member(&context, entry, active, active_label, listed, status);
                }
            }
        }
    }

    /// The staging environment records the solana-devnet table: the test mint's active member under the devnet
    /// forwarder's label, and nothing else.
    #[test]
    fn staging_records_the_solana_devnet_table() {
        use crate::chain::SolanaCluster;
        let devnet = Chain::Solana(SolanaCluster::Devnet);
        let table = staging::table(devnet).expect("solana-devnet is recorded in staging");
        assert_eq!(table.entries.len(), 1, "one SPL token member");
        let member = table
            .entries
            .iter()
            .find(|entry| matches!(entry.metadata, Some(Metadata::SplToken { .. })))
            .expect("an SPL token member");
        assert_eq!(
            member.label_ref.to_string(),
            "53dcbb3ebad803b9f20fc2457f1271b2981e52ed602c240cfbbfafb1d58f7b7a",
            "the label is sha256(devnet forwarder ‖ test mint)"
        );
        assert_eq!(
            member.logic_ref,
            crate::circuits::spl_token_active().logic_ref
        );
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
            let recorded: Vec<&Chain> = commitments.keys().collect();
            let embedded: Vec<&Chain> = tables.keys().collect();
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
