//! Pins this repo's tables to `anoma-rm-risc0`. The file loader reads every kind point from the file, so an aliased
//! table is checked like any other, but it fills a process-wide table that takes one table per process, so its test
//! reads one file. A compliance unit takes its table as an argument, so the conversion test covers every recorded table.

use alloy_chains::NamedChain;
use anoma_risc0_kind_tables::table::{production, staging};
use anoma_risc0_kind_tables::{Caip2ChainId, Table};
use anoma_rm_risc0::Digest;
use anoma_rm_risc0::compliance::{self, KindTreeEntry};
use anoma_rm_risc0::constants::{init_kind_tree_from_file, kind_tree_root};
use anoma_rm_risc0::kind_tree::KindTree;
use anoma_rm_risc0::nullifier_key::NullifierKey;
use anoma_rm_risc0::resource::{ConsumedResourceWitness, Resource};
use std::path::PathBuf;

fn staging_table(chain: &Caip2ChainId) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../kind-tables/data/generated/staging")
        .join(chain.file_name())
}

/// The kind table root a compliance unit reproduces for `table`. The unit consumes one ephemeral resource of the
/// table's first kind and creates one of the same kind: a unit that takes no kind from its table commits to the empty
/// table instead.
fn unit_root(table: &KindTree) -> Digest {
    let kind = table.entries().first().expect("the table lists a kind");
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
        vec![ConsumedResourceWitness::from_resource(consumed, nf_key)],
        vec![created],
        table,
    );
    compliance::constrain(&witness)
        .expect("the unit is compliant")
        .kind_tree_root
}

#[test]
fn the_local_root_matches_the_circuit_loader() {
    let path = staging_table(&NamedChain::Sepolia.into());
    let table = Table::load(&path).expect("the staging sepolia table exists");

    init_kind_tree_from_file(&path).expect("the upstream loader accepts the generated table");
    assert_eq!(
        kind_tree_root().expect("the loader installed the table"),
        table.root(),
    );
}

#[test]
fn every_converted_table_commits_to_its_recorded_root() {
    for (environment, roots, tables) in [
        ("staging", staging::roots(), staging::tables()),
        ("production", production::roots(), production::tables()),
    ] {
        for (chain, table) in tables {
            let entries = table.entries.iter().map(KindTreeEntry::from).collect();
            let converted =
                KindTree::from_entries(entries).expect("arm-risc0 accepts the converted table");
            assert_eq!(
                unit_root(&converted),
                roots[chain],
                "{environment} {chain}: a unit given the converted table commits to another table"
            );
        }
    }
}
