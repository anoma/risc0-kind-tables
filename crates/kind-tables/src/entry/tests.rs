use super::*;
use crate::circuits::Status;
use hex::FromHex;

fn entry() -> Entry {
    let logic_ref =
        Digest::from_hex("898f3d23ccad1ec7f07051100973815ce3687870416bc96e823a7aeaa347c367")
            .unwrap();
    let label_ref = Digest::default();
    let kind_point = kind::point(&logic_ref, &label_ref).unwrap();
    Entry {
        metadata: None,
        logic_ref,
        label_ref,
        kind_point,
    }
}

fn erc20_metadata() -> Metadata {
    Metadata::Erc20 {
        version: "2.0.0".into(),
        name: "WETH".into(),
        token: Address::with_last_byte(6),
        forwarder: Address::with_last_byte(7),
        status: Status::Active,
        alias_of: None,
    }
}

#[test]
fn is_alias_rejects_a_derived_point() {
    assert!(!entry().is_alias());
}

#[test]
fn is_alias_accepts_a_point_of_another_key() {
    let mut alias = entry();
    alias.label_ref = Digest::from([1u32; 8]);
    assert!(alias.is_alias());
}

#[test]
fn the_json_representation_round_trips() {
    let mut entry = entry();
    entry.metadata = Some(erc20_metadata());
    let json = serde_json::to_string(&entry).unwrap();
    assert_eq!(serde_json::from_str::<Entry>(&json).unwrap(), entry);
}

#[test]
fn the_metadata_stays_out_of_the_commitment_input() {
    let bare = entry();
    let mut annotated = entry();
    annotated.metadata = Some(erc20_metadata());
    assert_eq!(
        crate::commitment::of(&[bare]),
        crate::commitment::of(&[annotated])
    );
}

#[test]
fn spl_token_metadata_round_trips_with_base58_addresses() {
    let metadata = Metadata::SplToken {
        version: "2.0.0-rc.1".to_string(),
        name: "AnomaPay devnet test token".to_string(),
        mint: "9EHEFzyuY7sZEzTVm7C3uMkNZFMgm5ZeWjGjirZ3MVfr"
            .parse()
            .unwrap(),
        forwarder: "5CrHbBeHjg53UyL3Htn9dCYYTy68fMcrbDoeAdo4yQrx"
            .parse()
            .unwrap(),
        status: Status::Active,
        alias_of: None,
    };
    let json = serde_json::to_value(&metadata).unwrap();
    assert_eq!(json["type"], "SPLTokenResource");
    assert_eq!(json["mint"], "9EHEFzyuY7sZEzTVm7C3uMkNZFMgm5ZeWjGjirZ3MVfr");
    assert_eq!(
        json["forwarder"],
        "5CrHbBeHjg53UyL3Htn9dCYYTy68fMcrbDoeAdo4yQrx"
    );
    assert_eq!(json.get("alias_of"), None);
    assert_eq!(serde_json::from_value::<Metadata>(json).unwrap(), metadata);
}
