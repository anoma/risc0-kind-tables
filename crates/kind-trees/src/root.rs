//! The kind tree root — the Merkle root that arm-risc0 builds over a tree. Its leaves are
//! `sha256(logic_ref ‖ label_ref ‖ kind_point)` in `(logic_ref, label_ref)` byte order, padded to a power of two with
//! `sha256("EMPTY")`, and each node hashes its two children. The value `setKindTreeRoot` installs and every
//! compliance proof that takes a kind from the tree reproduces.

use crate::entry::Entry;
use risc0_zkvm::Digest;
use risc0_zkvm::sha::{Impl, Sha256};

/// Computes the root of a tree given as its entries, in any order.
pub fn of(entries: &[Entry]) -> Digest {
    let mut sorted: Vec<&Entry> = entries.iter().collect();
    sorted.sort_by(|a, b| {
        a.logic_ref
            .as_bytes()
            .cmp(b.logic_ref.as_bytes())
            .then_with(|| a.label_ref.as_bytes().cmp(b.label_ref.as_bytes()))
    });
    let padding = *Impl::hash_bytes(b"EMPTY");
    let mut layer: Vec<Digest> = sorted.into_iter().map(leaf).collect();
    if layer.is_empty() {
        return padding;
    }
    layer.resize(layer.len().next_power_of_two(), padding);
    while layer.len() > 1 {
        layer = layer
            .chunks(2)
            .map(|pair| node(&pair[0], &pair[1]))
            .collect();
    }
    layer[0]
}

fn leaf(entry: &Entry) -> Digest {
    let mut bytes = Vec::with_capacity(64 + entry.kind_point.len());
    bytes.extend_from_slice(entry.logic_ref.as_bytes());
    bytes.extend_from_slice(entry.label_ref.as_bytes());
    bytes.extend_from_slice(&entry.kind_point);
    *Impl::hash_bytes(&bytes)
}

fn node(left: &Digest, right: &Digest) -> Digest {
    let mut words = Vec::with_capacity(16);
    words.extend_from_slice(left.as_words());
    words.extend_from_slice(right.as_words());
    *Impl::hash_words(&words)
}
