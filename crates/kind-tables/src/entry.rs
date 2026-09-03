//! One row of a kind table. The JSON representation is `anoma-rm-risc0`'s kind table schema extended with the
//! kind point, so the upstream loader reads a generated table unchanged.

use crate::kind;
use alloy::primitives::Address;
use risc0_zkvm::Digest;
use serde::{Deserialize, Serialize};

/// A `(logic_ref, label_ref)` key and the kind point it names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Review context for the entry; not covered by the commitment.
    #[serde(rename = "_metadata", default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
    #[serde(with = "hex_digest")]
    pub logic_ref: Digest,
    #[serde(with = "hex_digest")]
    pub label_ref: Digest,
    /// Uncompressed SEC1-encoded point (65 bytes).
    #[serde(with = "hex_bytes")]
    pub kind_point: Vec<u8>,
}

/// What a kind belongs to: the circuit version behind `logic_ref`, and the deployment behind `label_ref`.
/// The commitment covers none of it, so it stays free to carry whatever a reviewer needs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Metadata {
    /// The padding kind, whose logic ships with the resource machine.
    Padding { version: String },
    /// One supported token behind an ERC20 forwarder.
    #[serde(rename = "ERC20")]
    Erc20 {
        version: String,
        name: String,
        #[serde(with = "checksummed")]
        token: Address,
        #[serde(with = "checksummed")]
        forwarder: Address,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        alias_of: Option<AliasOf>,
    },
    /// The arbitrary-call kind behind a generic call forwarder.
    GenericCall {
        version: String,
        #[serde(with = "checksummed")]
        forwarder: Address,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        alias_of: Option<AliasOf>,
    },
}

impl Metadata {
    /// The version of the circuit that owns the entry's `logic_ref`.
    pub fn version(&self) -> &str {
        match self {
            Self::Padding { version }
            | Self::Erc20 { version, .. }
            | Self::GenericCall { version, .. } => version,
        }
    }

    /// Records the canonical entry this one takes its point from. Padding is never aliased.
    pub fn aliased_to(self, canonical: AliasOf) -> Self {
        match self {
            Self::Erc20 {
                version,
                name,
                token,
                forwarder,
                ..
            } => Self::Erc20 {
                version,
                name,
                token,
                forwarder,
                alias_of: Some(canonical),
            },
            Self::GenericCall {
                version, forwarder, ..
            } => Self::GenericCall {
                version,
                forwarder,
                alias_of: Some(canonical),
            },
            padding => padding,
        }
    }
}

/// The canonical entry an alias takes its point from. The two keys name one kind, so resources of both
/// versions are fungible — the migration path between circuit versions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AliasOf {
    pub version: String,
    #[serde(with = "hex_digest")]
    pub logic_ref: Digest,
    #[serde(with = "hex_digest")]
    pub label_ref: Digest,
}

impl Entry {
    /// The ordering key: entries sort by `logic_ref ‖ label_ref`.
    pub fn key(&self) -> [u8; 64] {
        let mut key = [0u8; 64];
        key[..32].copy_from_slice(self.logic_ref.as_ref());
        key[32..].copy_from_slice(self.label_ref.as_ref());
        key
    }

    /// Whether the point is the one the key hashes to. A `false` marks an alias.
    pub fn is_canonical(&self) -> bool {
        kind::point(&self.logic_ref, &self.label_ref).is_ok_and(|point| point == self.kind_point)
    }
}

mod hex_digest {
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
            alias_of: None,
        }
    }

    #[test]
    fn is_canonical_accepts_a_derived_point() {
        assert!(entry().is_canonical());
    }

    #[test]
    fn is_canonical_rejects_an_aliased_point() {
        let mut aliased = entry();
        aliased.label_ref = Digest::from([1u32; 8]);
        assert!(!aliased.is_canonical());
    }

    #[test]
    fn the_json_representation_round_trips() {
        let mut entry = entry();
        entry.metadata = Some(erc20_metadata());
        let json = serde_json::to_string(&entry).unwrap();
        assert_eq!(serde_json::from_str::<Entry>(&json).unwrap(), entry);
    }

    #[test]
    fn aliased_to_records_the_canonical_key() {
        let canonical = AliasOf {
            version: "2.0.0".into(),
            logic_ref: Digest::from([1u32; 8]),
            label_ref: Digest::from([2u32; 8]),
        };
        let Metadata::Erc20 { alias_of, .. } = erc20_metadata().aliased_to(canonical.clone())
        else {
            panic!("aliasing an ERC20 entry keeps it an ERC20 entry");
        };
        assert_eq!(alias_of, Some(canonical));
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
