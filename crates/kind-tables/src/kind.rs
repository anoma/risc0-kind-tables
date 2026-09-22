//! The kind derivation — RFC 9380 hash-to-curve over `logic_ref ‖ label_ref`, exactly as the compliance
//! circuit computes it, pinned by the cross-check test against `anoma-rm-risc0`. A fungibility domain's kind
//! point is the kind of the active version under the current forwarder's label, so this is the only
//! derivation.
use crate::error::{Error, Result};
use crate::solana::SolanaAddress;
use alloy::primitives::Address;
use k256::Secp256k1;
use k256::elliptic_curve::hash2curve::{ExpandMsgXmd, GroupDigest};
use k256::elliptic_curve::sec1::ToEncodedPoint;
use risc0_zkvm::Digest;
use risc0_zkvm::sha::rust_crypto::Sha256;
use risc0_zkvm::sha::{Impl, Sha256 as _};

/// The RFC 9380 domain separation tag the compliance circuit uses.
const DST: &[u8] = b"QUUX-V01-CS02-with-secp256k1_XMD:SHA-256_SSWU_RO_";

/// Derives a kind: the uncompressed SEC1 encoding (65 bytes).
pub fn point(logic_ref: &Digest, label_ref: &Digest) -> Result<Vec<u8>> {
    let mut bytes = [0u8; 64];
    bytes[..32].copy_from_slice(logic_ref.as_ref());
    bytes[32..].copy_from_slice(label_ref.as_ref());
    let point =
        Secp256k1::hash_from_bytes::<ExpandMsgXmd<Sha256>>(&[&bytes], &[DST]).map_err(|_| {
            Error::KindDerivationFailed {
                logic_ref: logic_ref.to_string(),
                label_ref: label_ref.to_string(),
            }
        })?;
    Ok(point.to_encoded_point(false).as_bytes().to_vec())
}

/// The label of an ERC20 resource: `sha256(forwarder ‖ token)`, as the transfer circuit computes it.
pub fn erc20_label_ref(forwarder: &Address, token: &Address) -> Digest {
    label_ref(forwarder.as_slice(), token.as_slice())
}

/// The label of an SPL token resource: `sha256(forwarder program id ‖ mint)`, as the Solana transfer circuit
/// computes it (`transfer_witness::calculate_label_ref`), the ERC20 shape over 32-byte inputs.
pub fn spl_token_label_ref(forwarder: &SolanaAddress, mint: &SolanaAddress) -> Digest {
    label_ref(forwarder.as_bytes(), mint.as_bytes())
}

/// A token resource's label: the hash of the forwarder holding it and the token it holds.
fn label_ref(forwarder: &[u8], token: &[u8]) -> Digest {
    *Impl::hash_bytes(&[forwarder, token].concat())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spl_token_label_is_sha256_over_the_program_id_and_the_mint() {
        let forwarder: SolanaAddress = "5CrHbBeHjg53UyL3Htn9dCYYTy68fMcrbDoeAdo4yQrx"
            .parse()
            .unwrap();
        let mint: SolanaAddress = "9EHEFzyuY7sZEzTVm7C3uMkNZFMgm5ZeWjGjirZ3MVfr"
            .parse()
            .unwrap();
        assert_eq!(
            spl_token_label_ref(&forwarder, &mint).to_string(),
            "53dcbb3ebad803b9f20fc2457f1271b2981e52ed602c240cfbbfafb1d58f7b7a"
        );
    }

    #[test]
    fn the_label_is_sha256_over_the_two_addresses() {
        let forwarder: Address = "0xE54182d915dE447deFc4A17Ec1D4E0dc627551F7"
            .parse()
            .unwrap();
        let token: Address = "0x4200000000000000000000000000000000000006"
            .parse()
            .unwrap();
        assert_eq!(
            erc20_label_ref(&forwarder, &token).to_string(),
            "55008fad9bfccef776960bfca715e365cfba843cfb947190fafc69e4d9fac674"
        );
    }
}
