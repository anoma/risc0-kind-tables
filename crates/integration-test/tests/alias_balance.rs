//! Every ERC20 member of a generated table balances the active member of its fungibility domain in a compliance unit.
//! The resources take their labels from the transfer circuit's derivation, and the compliance constraints take their
//! kinds from the table, so the generated keys and kind points are checked against both circuits. It needs no chain.

use anoma_risc0_kind_tables::table::{production, staging};
use anoma_risc0_kind_tables::{Entry, Metadata, Table};
use anoma_rm_risc0::Digest;
use anoma_rm_risc0::compliance::{self, KindTableEntry};
use anoma_rm_risc0::nullifier_key::NullifierKey;
use anoma_rm_risc0::resource::{ConsumedResourceWitness, Resource};
use transfer_witness::calculate_label_ref;

/// A resource kind, written as its `(logic_ref, label_ref)`.
type Kind = (Digest, Digest);

/// The kind of this member's resources: its logic, and the label the transfer circuit derives for its token.
fn resource_kind(entry: &Entry) -> Option<Kind> {
    let Some(Metadata::Erc20 {
        token, forwarder, ..
    }) = &entry.metadata
    else {
        return None;
    };
    Some((
        entry.logic_ref,
        calculate_label_ref(forwarder.as_slice(), token.as_slice()),
    ))
}

/// The active member this entry takes its kind point from, or the entry itself.
fn active<'table>(table: &'table Table, entry: &'table Entry) -> &'table Entry {
    let Some(alias_of) = entry.metadata.as_ref().and_then(Metadata::alias_of) else {
        return entry;
    };
    table
        .entries
        .iter()
        .find(|candidate| {
            (candidate.logic_ref, candidate.label_ref) == (alias_of.logic_ref, alias_of.label_ref)
        })
        .expect("the table lists the active member of every alias")
}

/// The delta of a unit that consumes one unit of `consumed` and creates one unit of `created`. `rcv` is fixed to 1, so
/// two deltas differ only in the kind points the unit looks up.
fn delta(consumed: Kind, created: Kind, table: &[KindTableEntry]) -> ([u32; 8], [u32; 8]) {
    let nf_key = NullifierKey::from_bytes([1; 32]);
    let nk_commitment = nf_key.commit();
    let resource = |(logic_ref, label_ref): Kind, nonce| Resource {
        logic_ref,
        label_ref,
        quantity: 1,
        value_ref: Digest::default(),
        is_ephemeral: true,
        nonce,
        nk_commitment,
        rand_seed: [0; 32],
    };
    let consumed = resource(consumed, [0; 32]);
    let nullifier = consumed.nullifier(&nf_key).expect("a nullifier derives");
    let nonce = Resource::derive_nonce_from_nullifiers(0, &[nullifier]).expect("a nonce derives");
    let created = resource(created, nonce);
    let mut witness = compliance::from_resources(
        vec![ConsumedResourceWitness::from_resource(consumed, nf_key)],
        vec![created],
        table.to_vec(),
    );
    let mut rcv = [0; 32];
    rcv[31] = 1;
    witness.rcv = rcv.to_vec();
    let instance = compliance::constrain(&witness).expect("the unit is compliant");
    (instance.delta_x, instance.delta_y)
}

#[test]
fn every_erc20_member_balances_its_active_member() {
    for (module, tables) in [
        ("staging", staging::tables()),
        ("production", production::tables()),
    ] {
        for (chain, table) in tables {
            let entries: Vec<KindTableEntry> = table
                .entries
                .iter()
                .map(|entry| KindTableEntry {
                    logic_ref: entry.logic_ref,
                    label_ref: entry.label_ref,
                    kind_point: entry.kind_point.clone(),
                })
                .collect();
            for entry in &table.entries {
                let Some(member) = resource_kind(entry) else {
                    continue;
                };
                assert_eq!(
                    member.1, entry.label_ref,
                    "{module} {chain}: the transfer circuit labels a resource of {} differently",
                    entry.label_ref
                );
                let active = resource_kind(active(table, entry)).expect("an active ERC20 member");
                let balanced = delta(active, active, &entries);
                assert_eq!(
                    delta(member, active, &entries),
                    balanced,
                    "{module} {chain}: {} does not balance its active member",
                    entry.label_ref
                );
                if entry.is_alias() {
                    assert_ne!(
                        delta(member, active, &[]),
                        delta(active, active, &[]),
                        "{module} {chain}: {} balances its active member without the table",
                        entry.label_ref
                    );
                }
            }
        }
    }
}
