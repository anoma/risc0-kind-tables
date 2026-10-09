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

environment_module!(
    staging,
    ("eip155", "11155111"),
    ("eip155", "84532"),
    ("solana", "EtWTRABZaYq6iMfeYKouRu166VU2xqa1")
);
environment_module!(production);

#[cfg(test)]
mod tests;
