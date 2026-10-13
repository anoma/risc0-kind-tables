use anoma_risc0_kind_tables::circuits::*;

#[test]
fn the_embedded_list_passes_its_own_checks() {
    check_erc20().unwrap();
    assert_eq!(erc20_active().status, Status::Active);
}

#[test]
fn the_embedded_spl_token_list_passes_its_own_checks() {
    check_spl_token().unwrap();
    let active = spl_token_active();
    assert_eq!(active.status, Status::Active);
    assert_eq!(spl_token_version(&active.logic_ref), Some(active));
}
