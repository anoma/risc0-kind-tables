//! The kind table commitment — SHA-256 over the concatenated `logic_ref ‖ label_ref ‖ kind_point` of every entry
//! in order. The value `setKindTableCommitment` installs and every compliance proof reproduces; entry order is
//! part of it.

use crate::entry::Entry;
use risc0_zkvm::Digest;
use risc0_zkvm::sha::{Impl, Sha256};

/// Computes the commitment of a table given as its ordered entries.
pub fn of(entries: &[Entry]) -> Digest {
    let mut bytes = Vec::new();
    for entry in entries {
        bytes.extend_from_slice(entry.logic_ref.as_bytes());
        bytes.extend_from_slice(entry.label_ref.as_bytes());
        bytes.extend_from_slice(&entry.kind_point);
    }
    *Impl::hash_bytes(&bytes)
}
