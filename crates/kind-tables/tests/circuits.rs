use anoma_risc0_kind_tables::circuits::*;
use risc0_zkvm::Digest;

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

/// A listed version whose logic ref is `word` repeated.
fn listed(version: &str, word: u32, status: Status) -> CircuitVersion {
    CircuitVersion {
        version: version.to_string(),
        logic_ref: Digest::new([word; 8]),
        status,
    }
}

#[test]
fn check_accepts_an_active_version_listed_last_below_a_higher_version() {
    let versions = [
        listed("3.0.0-rc.3", 1, Status::Deprecated),
        listed("2.0.0-rc.1", 2, Status::Active),
    ];

    assert_eq!(check(&versions), Ok(()));
}

#[test]
fn check_rejects_an_active_version_listed_before_a_lower_version() {
    let versions = [
        listed("3.0.0-rc.3", 1, Status::Active),
        listed("2.0.0-rc.1", 2, Status::Deprecated),
    ];

    assert_eq!(
        check(&versions),
        Err("3.0.0-rc.3 is active, but 2.0.0-rc.1 is listed after it; the active version is the last listed".to_string())
    );
}
