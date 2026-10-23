use anoma_risc0_kind_trees::Entry;
use anoma_risc0_kind_trees::kind;
use anoma_risc0_kind_trees::root::*;
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
fn of_is_independent_of_the_entry_order() {
    let (a, b) = (entry(1), entry(2));
    assert_eq!(of(&[a.clone(), b.clone()]), of(&[b, a]));
}

#[test]
fn of_gives_a_one_entry_tree_its_leaf() {
    let a = entry(1);
    let leaf = [
        a.logic_ref.as_bytes(),
        a.label_ref.as_bytes(),
        a.kind_point.as_slice(),
    ]
    .concat();
    assert_eq!(of(std::slice::from_ref(&a)), *Impl::hash_bytes(&leaf));
}

#[test]
fn of_gives_the_empty_tree_the_padding_leaf() {
    assert_eq!(of(&[]), *Impl::hash_bytes(b"EMPTY"));
}
