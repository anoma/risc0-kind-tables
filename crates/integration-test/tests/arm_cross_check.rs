//! Pins this repo's tables to `anoma-rm-risc0`. The file loader reads every kind point from the file, so an aliased
//! table is checked like any other, but it fills a process-wide table that takes one table per process, so its test
//! reads one file. A compliance unit takes its table as an argument, so the conversion test covers every recorded table.

use anoma_risc0_kind_tables::Chain;
use anoma_risc0_kind_tables::Table;
use anoma_risc0_kind_tables::table::{production, staging};
use anoma_rm_risc0::Digest;
use anoma_rm_risc0::compliance::{self, KindTableEntry};
use anoma_rm_risc0::constants::{init_kind_table_from_file, kind_table_hash};
use anoma_rm_risc0::nullifier_key::NullifierKey;
use anoma_rm_risc0::resource::{ConsumedResourceWitness, Resource};
use std::path::PathBuf;

fn staging_table(chain: Chain) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../kind-tables/data/generated/staging")
        .join(format!("{}.json", chain.file_stem()))
}

/// The kind table commitment a compliance unit reproduces for `table`. The unit consumes one ephemeral resource and
/// creates one of the same kind.
fn unit_commitment(table: Vec<KindTableEntry>) -> Digest {
    let nf_key = NullifierKey::from_bytes([1; 32]);
    let consumed = Resource {
        logic_ref: Digest::default(),
        label_ref: Digest::default(),
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
        .kind_table_commitment
}

#[test]
fn the_local_commitment_matches_the_circuit_loader() {
    let path = staging_table(Chain::Evm(alloy_chains::NamedChain::Sepolia));
    let table = Table::load(&path).expect("the staging sepolia table exists");

    init_kind_table_from_file(&path).expect("the upstream loader accepts the generated table");
    assert_eq!(
        kind_table_hash().expect("the loader installed the table"),
        &table.commitment(),
    );
}

#[test]
fn every_converted_table_commits_to_its_recorded_commitment() {
    for (environment, commitments, tables) in [
        ("staging", staging::commitments(), staging::tables()),
        (
            "production",
            production::commitments(),
            production::tables(),
        ),
    ] {
        for (chain, table) in tables {
            let entries = table.entries.iter().map(KindTableEntry::from).collect();
            assert_eq!(
                unit_commitment(entries),
                commitments[chain],
                "{environment} {chain}: a unit given the converted table commits to another table"
            );
        }
    }
}
