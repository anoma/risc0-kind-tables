//! One row of a kind table. The JSON representation is `anoma-rm-risc0`'s kind table schema extended with the
//! kind point, so the upstream loader reads a generated table unchanged.

use crate::circuits::Status;
use crate::kind;
use alloy::primitives::Address;
use risc0_zkvm::Digest;
use serde::{Deserialize, Serialize};

/// A kind, written as its `(logic_ref, label_ref)`, and the kind point it is assigned.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Review context for the entry; not covered by the commitment.
    #[serde(rename = "_metadata", default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
    #[serde(with = "hex_digest")]
    pub logic_ref: Digest,
    #[serde(with = "hex_digest")]
    pub label_ref: Digest,
    /// The assigned kind point, SEC1-encoded and uncompressed (65 bytes).
    #[serde(with = "hex_bytes")]
    pub kind_point: Vec<u8>,
}

/// What a kind belongs to: the circuit version behind `logic_ref`, and the deployment behind `label_ref`.
/// The commitment covers none of it, so it stays free to carry whatever a reviewer needs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Metadata {
    /// One supported token behind an ERC20 forwarder. `forwarder` is the address inside this kind's label. On
    /// a V1 member it is the V1 forwarder, whose tokens move to the current one.
    #[serde(rename = "ERC20Resource")]
    Erc20 {
        version: String,
        name: String,
        #[serde(with = "checksummed")]
        token: Address,
        #[serde(with = "checksummed")]
        forwarder: Address,
        /// `active` if and only if `alias_of` is absent: the backend creates only that member's resources.
        status: Status,
        /// The kind this entry takes its kind point from. Absent only on the active version under the current
        /// forwarder's label.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        alias_of: Option<AliasOf>,
    },
    /// The arbitrary-call kind behind a generic call forwarder.
    #[serde(rename = "GenericCallResource")]
    GenericCall {
        version: String,
        #[serde(with = "checksummed")]
        forwarder: Address,
    },
}

impl Metadata {
    /// The kind an ERC20 alias takes its kind point from; `None` for every other entry.
    pub fn alias_of(&self) -> Option<&AliasOf> {
        match self {
            Self::Erc20 { alias_of, .. } => alias_of.as_ref(),
            Self::GenericCall { .. } => None,
        }
    }
}

/// The kind an alias takes its kind point from: the active circuit version under the current forwarder's
/// label. Written into `_metadata`, so an alias says what it is a second name for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AliasOf {
    pub version: String,
    #[serde(with = "hex_digest")]
    pub logic_ref: Digest,
    #[serde(with = "hex_digest")]
    pub label_ref: Digest,
}

impl Entry {
    /// The sort order: entries sort by `logic_ref ‖ label_ref`.
    pub fn key(&self) -> [u8; 64] {
        let mut key = [0u8; 64];
        key[..32].copy_from_slice(self.logic_ref.as_ref());
        key[32..].copy_from_slice(self.label_ref.as_ref());
        key
    }

    /// Whether the entry is assigned another kind as its kind point, not its own, so that the table is the only
    /// place it can come from. Generic call and the active version under the current forwarder's label are
    /// not.
    pub fn is_alias(&self) -> bool {
        !kind::point(&self.logic_ref, &self.label_ref).is_ok_and(|point| point == self.kind_point)
    }
}

/// The entry as the resource machine loads it, without the metadata the commitment does not cover.
#[cfg(feature = "arm")]
impl From<&Entry> for anoma_rm_risc0::compliance::KindTableEntry {
    fn from(entry: &Entry) -> Self {
        Self {
            logic_ref: entry.logic_ref,
            label_ref: entry.label_ref,
            kind_point: entry.kind_point.clone(),
        }
    }
}

pub(crate) mod hex_digest {
    use hex::FromHex;
    use risc0_zkvm::Digest;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(digest: &Digest, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&hex::encode(digest.as_bytes()))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Digest, D::Error> {
        let string = String::deserialize(deserializer)?;
        Digest::from_hex(&string).map_err(serde::de::Error::custom)
    }
}

mod hex_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let string = String::deserialize(deserializer)?;
        hex::decode(&string).map_err(serde::de::Error::custom)
    }
}

/// Addresses are written checksummed, so a reviewer can paste one into a block explorer.
mod checksummed {
    use alloy::primitives::Address;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(address: &Address, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&address.to_checksum(None))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Address, D::Error> {
        let string = String::deserialize(deserializer)?;
        string.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex::FromHex;

    fn entry() -> Entry {
        let logic_ref =
            Digest::from_hex("898f3d23ccad1ec7f07051100973815ce3687870416bc96e823a7aeaa347c367")
                .unwrap();
        let label_ref = Digest::default();
        let kind_point = kind::point(&logic_ref, &label_ref).unwrap();
        Entry {
            metadata: None,
            logic_ref,
            label_ref,
            kind_point,
        }
    }

    fn erc20_metadata() -> Metadata {
        Metadata::Erc20 {
            version: "2.0.0".into(),
            name: "WETH".into(),
            token: Address::with_last_byte(6),
            forwarder: Address::with_last_byte(7),
            status: Status::Active,
            alias_of: None,
        }
    }

    #[test]
    fn is_alias_rejects_a_derived_point() {
        assert!(!entry().is_alias());
    }

    #[test]
    fn is_alias_accepts_a_point_of_another_key() {
        let mut alias = entry();
        alias.label_ref = Digest::from([1u32; 8]);
        assert!(alias.is_alias());
    }

    #[test]
    fn the_json_representation_round_trips() {
        let mut entry = entry();
        entry.metadata = Some(erc20_metadata());
        let json = serde_json::to_string(&entry).unwrap();
        assert_eq!(serde_json::from_str::<Entry>(&json).unwrap(), entry);
    }

    #[test]
    fn the_metadata_stays_out_of_the_commitment_input() {
        let bare = entry();
        let mut annotated = entry();
        annotated.metadata = Some(erc20_metadata());
        assert_eq!(
            crate::commitment::of(&[bare]),
            crate::commitment::of(&[annotated])
        );
    }
}
