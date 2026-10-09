use anoma_risc0_kind_tables::Entry;
use anoma_risc0_kind_tables::commitment::*;
use anoma_risc0_kind_tables::kind;
use risc0_zkvm::Digest;
use risc0_zkvm::sha::{Impl, Sha256};

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
