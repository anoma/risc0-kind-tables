//! Pins this repo's trees to `anoma-rm-risc0`. The file loader reads every kind point from the file, so an aliased
//! tree is checked like any other, but it fills a process-wide tree that takes one tree per process, so its test
//! reads one file. A compliance unit takes its tree as an argument, so the conversion test covers every recorded tree.

use alloy_chains::NamedChain;
use anoma_risc0_kind_trees::tree::{production, staging};
use anoma_risc0_kind_trees::{Caip2ChainId, Tree};
use anoma_rm_risc0::Digest;
use anoma_rm_risc0::compliance::{self, KindTreeEntry};
use anoma_rm_risc0::constants::{init_kind_tree_from_file, kind_tree_root};
use anoma_rm_risc0::kind_tree::KindTree;
use anoma_rm_risc0::nullifier_key::NullifierKey;
use anoma_rm_risc0::resource::{ConsumedResourceWitness, Resource, ResourceWitness};
use std::path::PathBuf;

fn staging_tree(chain: &Caip2ChainId) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../kind-trees/data/generated/staging")
        .join(chain.file_name())
}

/// The kind tree root a compliance unit reproduces for `tree`. The unit consumes one ephemeral resource of the
/// tree's first kind and creates one of the same kind: a unit that takes no kind from its tree commits to the empty
/// tree instead.
fn unit_root(tree: &KindTree) -> Digest {
    let kind = tree.entries().first().expect("the tree lists a kind");
    let nf_key = NullifierKey::from_bytes([1; 32]);
    let consumed = Resource {
        logic_ref: kind.logic_ref,
        label_ref: kind.label_ref,
        quantity: 1,
        value_ref: Digest::default(),
        is_ephemeral: true,
        nonce: [0; 32],
        nk_commitment: nf_key.commit(),
        rand_seed: [0; 32],
    };
    let nullifier = consumed.nullifier(&nf_key).expect("a nullifier derives");
    let created = Resource {
        nonce: Resource::derive_nonce_from_nullifiers(0, &[nullifier]).expect("a nonce derives"),
        ..consumed
    };
    let witness = compliance::from_resources(
        vec![
            ResourceWitness::Consumed(ConsumedResourceWitness::from_resource(consumed, nf_key)),
            ResourceWitness::Created(created),
        ],
        tree,
    );
    compliance::constrain(&witness)
        .expect("the unit is compliant")
        .kind_tree_root
}

#[test]
fn the_local_root_matches_the_circuit_loader() {
    let path = staging_tree(&NamedChain::Sepolia.into());
    let tree = Tree::load(&path).expect("the staging sepolia tree exists");

    init_kind_tree_from_file(&path).expect("the upstream loader accepts the generated tree");
    assert_eq!(
        kind_tree_root().expect("the loader installed the tree"),
        tree.root(),
    );
}

#[test]
fn every_converted_tree_commits_to_its_recorded_root() {
    for (environment, roots, trees) in [
        ("staging", staging::roots(), staging::trees()),
        ("production", production::roots(), production::trees()),
    ] {
        for (chain, tree) in trees {
            let entries = tree.entries.iter().map(KindTreeEntry::from).collect();
            let converted =
                KindTree::from_entries(entries).expect("arm-risc0 accepts the converted tree");
            assert_eq!(
                unit_root(&converted),
                roots[chain],
                "{environment} {chain}: a unit given the converted tree commits to another tree"
            );
        }
    }
}
