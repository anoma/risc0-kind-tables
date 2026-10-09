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
