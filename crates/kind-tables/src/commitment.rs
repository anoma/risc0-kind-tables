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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kind;

    fn entry(seed: u8) -> Entry {
        let logic_ref = Digest::from([seed as u32; 8]);
        let label_ref = Digest::default();
        Entry {
            metadata: None,
            kind_point: kind::point(&logic_ref, &label_ref).unwrap(),
            logic_ref,
            label_ref,
        }
    }

    #[test]
    fn of_commits_to_the_entry_order() {
        let (a, b) = (entry(1), entry(2));
        assert_ne!(of(&[a.clone(), b.clone()]), of(&[b, a]));
    }

    #[test]
    fn of_hashes_the_empty_table_to_the_empty_sha256() {
        assert_eq!(of(&[]), *Impl::hash_bytes(&[]));
    }
}
