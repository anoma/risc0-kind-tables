//! The ERC20 circuit versions, read from `data/circuit-versions.json`. Every listed version is a member of every
//! ERC20 fungibility domain, so its resources are fungible with every other listed version's behind the same
//! forwarder. Exactly one version is active: under the label of the environment's forwarder it keeps its own kind,
//! which is the kind point of every ERC20 fungibility domain, and the backend creates its resources. Every other
//! version is deprecated: an alias of the active one, which the backend consumes and converts. Nothing is removed. A
//! version whose logic ref the protocol adapter denies stays listed as deprecated, with rows that nothing can use.

use crate::entry::hex_digest;
use risc0_zkvm::Digest;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// Whether a circuit version, or a member, is the one the backend creates resources of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// The version that keeps its own kind under the label of the environment's forwarder. The backend creates its
    /// resources.
    Active,
    /// An alias of the active version. The backend consumes its resources and converts them.
    Deprecated,
}

/// One listed version of the ERC20 transfer circuit.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CircuitVersion {
    /// The circuit's release version. The crate that ships it can carry another version.
    pub version: String,
    /// The circuit's verifying key, which every resource of this version carries as its logic ref.
    #[serde(with = "hex_digest")]
    pub logic_ref: Digest,
    pub status: Status,
}

#[derive(Deserialize)]
struct File {
    #[serde(rename = "ERC20Resource")]
    erc20: Vec<CircuitVersion>,
    #[serde(rename = "SPLTokenResource")]
    spl_token: Vec<CircuitVersion>,
}

static FILE: LazyLock<File> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../data/circuit-versions.json"))
        .expect("circuit-versions.json: invalid JSON")
});

/// The listed ERC20 circuit versions, in file order.
pub fn erc20() -> &'static [CircuitVersion] {
    &FILE.erc20
}

/// The listed SPL token circuit versions (the AnomaPay Solana transfer circuit), in file order, under the same
/// rules as the ERC20 list.
pub fn spl_token() -> &'static [CircuitVersion] {
    &FILE.spl_token
}

/// The active ERC20 circuit version. Its kind under the label of the environment's forwarder is the kind point of
/// every ERC20 fungibility domain. `check_erc20` says why there is exactly one.
pub fn erc20_active() -> &'static CircuitVersion {
    active(erc20(), "ERC20")
}

/// The active SPL token circuit version; `check_spl_token` says why there is exactly one.
pub fn spl_token_active() -> &'static CircuitVersion {
    active(spl_token(), "SPL token")
}

/// The listed ERC20 version a logic ref belongs to, if any.
pub fn erc20_version(logic_ref: &Digest) -> Option<&'static CircuitVersion> {
    version(erc20(), logic_ref)
}

/// The listed SPL token version a logic ref belongs to, if any.
pub fn spl_token_version(logic_ref: &Digest) -> Option<&'static CircuitVersion> {
    version(spl_token(), logic_ref)
}

/// `check` over the ERC20 list.
pub fn check_erc20() -> Result<(), String> {
    check(erc20())
}

/// `check` over the SPL token list.
pub fn check_spl_token() -> Result<(), String> {
    check(spl_token())
}

fn active(versions: &'static [CircuitVersion], resource: &str) -> &'static CircuitVersion {
    versions
        .iter()
        .find(|circuit| circuit.status == Status::Active)
        .unwrap_or_else(|| {
            panic!("circuit-versions.json lists no active {resource} circuit version")
        })
}

fn version(
    versions: &'static [CircuitVersion],
    logic_ref: &Digest,
) -> Option<&'static CircuitVersion> {
    versions
        .iter()
        .find(|circuit| circuit.logic_ref == *logic_ref)
}

/// Checks a list: every version and logic ref once, and exactly one active version, the last listed. A list is in
/// release order, whatever its version numbers, so the active version is the latest release.
pub fn check(versions: &[CircuitVersion]) -> Result<(), String> {
    let mut seen_versions = std::collections::BTreeSet::new();
    let mut seen_refs = std::collections::BTreeSet::new();
    for circuit in versions {
        if !seen_versions.insert(&circuit.version) {
            return Err(format!("{} is listed twice", circuit.version));
        }
        if !seen_refs.insert(circuit.logic_ref) {
            return Err(format!(
                "the logic ref of {} is listed twice",
                circuit.version
            ));
        }
        semver::Version::parse(&circuit.version)
            .map_err(|error| format!("{} is not a version: {error}", circuit.version))?;
    }
    let mut actives = versions
        .iter()
        .filter(|circuit| circuit.status == Status::Active);
    let active = actives
        .next()
        .ok_or_else(|| "no version is active".to_string())?;
    if let Some(other) = actives.next() {
        return Err(format!(
            "{} and {} are both active; exactly one version is",
            active.version, other.version
        ));
    }
    if let Some(last) = versions.last()
        && last != active
    {
        return Err(format!(
            "{} is active, but {} is listed after it; the active version is the last listed",
            active.version, last.version
        ));
    }
    Ok(())
}
