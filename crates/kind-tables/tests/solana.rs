use anoma_risc0_kind_tables::solana::*;

#[test]
fn an_address_round_trips_through_base58() {
    let address: SolanaAddress = "5CrHbBeHjg53UyL3Htn9dCYYTy68fMcrbDoeAdo4yQrx"
        .parse()
        .unwrap();
    assert_eq!(
        hex::encode(address.as_bytes()),
        "3e77dd4f346d113ac4f5239d59090c52287b6d541fb3f78bf7361d5662f31cbd"
    );
    assert_eq!(
        address.to_string(),
        "5CrHbBeHjg53UyL3Htn9dCYYTy68fMcrbDoeAdo4yQrx"
    );
    let json = serde_json::to_string(&address).unwrap();
    assert_eq!(json, "\"5CrHbBeHjg53UyL3Htn9dCYYTy68fMcrbDoeAdo4yQrx\"");
    assert_eq!(
        serde_json::from_str::<SolanaAddress>(&json).unwrap(),
        address
    );
}

#[test]
fn an_address_must_decode_to_32_bytes() {
    assert!("5CrHb".parse::<SolanaAddress>().is_err());
    assert!(
        "0xE54182d915dE447deFc4A17Ec1D4E0dc627551F7"
            .parse::<SolanaAddress>()
            .is_err()
    );
}
