use alloy::primitives::Address;
use anoma_risc0_kind_tables::SolanaAddress;
use anoma_risc0_kind_tables::kind::*;

#[test]
fn the_spl_token_label_is_sha256_over_the_program_id_and_the_mint() {
    let forwarder: SolanaAddress = "5CrHbBeHjg53UyL3Htn9dCYYTy68fMcrbDoeAdo4yQrx"
        .parse()
        .unwrap();
    let mint: SolanaAddress = "9EHEFzyuY7sZEzTVm7C3uMkNZFMgm5ZeWjGjirZ3MVfr"
        .parse()
        .unwrap();
    assert_eq!(
        spl_token_label_ref(&forwarder, &mint).to_string(),
        "53dcbb3ebad803b9f20fc2457f1271b2981e52ed602c240cfbbfafb1d58f7b7a"
    );
}

#[test]
fn the_label_is_sha256_over_the_two_addresses() {
    let forwarder: Address = "0xE54182d915dE447deFc4A17Ec1D4E0dc627551F7"
        .parse()
        .unwrap();
    let token: Address = "0x4200000000000000000000000000000000000006"
        .parse()
        .unwrap();
    assert_eq!(
        erc20_label_ref(&forwarder, &token).to_string(),
        "55008fad9bfccef776960bfca715e365cfba843cfb947190fafc69e4d9fac674"
    );
}
