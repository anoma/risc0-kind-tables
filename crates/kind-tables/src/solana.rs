//! A Solana account address: a 32-byte key written in base58, as program ids and mints are written everywhere
//! on Solana. Kept as bytes here so the library carries no Solana SDK; the generator and the integration tests
//! convert from the SDK's `Pubkey` where they read one.
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// A 32-byte Solana address.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SolanaAddress([u8; 32]);

impl SolanaAddress {
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl FromStr for SolanaAddress {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let bytes = bs58::decode(text)
            .into_vec()
            .map_err(|error| format!("{text}: not base58: {error}"))?;
        <[u8; 32]>::try_from(bytes)
            .map(Self)
            .map_err(|bytes| format!("{text}: decodes to {} bytes, not 32", bytes.len()))
    }
}

impl fmt::Display for SolanaAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&bs58::encode(self.0).into_string())
    }
}

impl fmt::Debug for SolanaAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SolanaAddress({self})")
    }
}

impl Serialize for SolanaAddress {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for SolanaAddress {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_address_round_trips_through_base58() {
        let address: SolanaAddress = "5CrHbBeHjg53UyL3Htn9dCYYTy68fMcrbDoeAdo4yQrx"
            .parse()
            .unwrap();
        assert_eq!(
            hex::encode(address.as_bytes()),
            "3e77dd4f346d113ac4f5239d59090c52287b6d541fb3f78bf7361d5662f31cbd"
        );
        assert_eq!(
            address.to_string(),
            "5CrHbBeHjg53UyL3Htn9dCYYTy68fMcrbDoeAdo4yQrx"
        );
        let json = serde_json::to_string(&address).unwrap();
        assert_eq!(json, "\"5CrHbBeHjg53UyL3Htn9dCYYTy68fMcrbDoeAdo4yQrx\"");
        assert_eq!(
            serde_json::from_str::<SolanaAddress>(&json).unwrap(),
            address
        );
    }

    #[test]
    fn an_address_must_decode_to_32_bytes() {
        assert!("5CrHb".parse::<SolanaAddress>().is_err());
        assert!(
            "0xE54182d915dE447deFc4A17Ec1D4E0dc627551F7"
                .parse::<SolanaAddress>()
                .is_err()
        );
    }
}
