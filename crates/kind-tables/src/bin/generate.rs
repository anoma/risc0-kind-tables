//! Regenerates `data/generated/` from the authored inputs (`tokens.json`, `successions.json`, `vulnerabilities.json`) and the pinned
//! dependencies (circuit IDs, forwarder and protocol adapter deployment records). CI reruns this and fails on
//! any diff, so the committed artifacts always match the pins.

use anoma_generic_call_forwarder_bindings::addresses::Environment as GenericCallEnvironment;
use anoma_kind_tables::{AliasOf, Entry, Metadata, Status, commitment, kind, tokens};
use anoma_pa_evm_bindings::addresses::{Environment, protocol_adapter_deployments_map};
use anomapay_erc20_forwarder_bindings::addresses::Environment as Erc20Environment;
use anyhow::{Context, Result, bail};
use risc0_zkvm::Digest;
use risc0_zkvm::sha::{Impl, Sha256};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

/// The circuits a kind can belong to. Padding is absent: its resources are ephemeral and zero-quantity, so
/// nothing is ever made fungible with them.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
enum Circuit {
    #[serde(rename = "ERC20")]
    Erc20,
    GenericCall,
}

/// The authored successions: one circuit version's kinds carry to its successor, on every chain that has
/// entries for it.
#[derive(Deserialize)]
struct Successions {
    successions: Vec<Succession>,
}

#[derive(Clone, Deserialize)]
struct Succession {
    #[serde(rename = "type")]
    circuit: Circuit,
    alias: String,
    of: String,
}

/// The circuit versions recorded as compromised.
#[derive(Deserialize)]
struct Vulnerabilities {
    vulnerable: Vec<Vulnerable>,
}

#[derive(Deserialize)]
struct Vulnerable {
    #[serde(rename = "type")]
    circuit: Circuit,
    version: String,
}

/// One row of `commitments.json`: the chain it belongs to, named for review, and the table commitment.
#[derive(Serialize)]
struct ChainCommitment {
    #[serde(rename = "_comment")]
    comment: String,
    commitment: String,
}

fn digest(bytes: &[u8]) -> Digest {
    Digest::try_from(bytes).expect("a circuit ID is 32 bytes")
}

fn sha256(bytes: &[u8]) -> Digest {
    *Impl::hash_bytes(bytes)
}

/// The protocol adapter's environment is the generator's; each forwarder crate declares its own.
fn erc20_environment(environment: Environment) -> Erc20Environment {
    match environment {
        Environment::Staging => Erc20Environment::Staging,
        Environment::Production => Erc20Environment::Production,
    }
}

fn generic_call_environment(environment: Environment) -> GenericCallEnvironment {
    match environment {
        Environment::Staging => GenericCallEnvironment::Staging,
        Environment::Production => GenericCallEnvironment::Production,
    }
}

fn entry(metadata: Metadata, logic_ref: Digest, label_ref: Digest) -> Result<Entry> {
    Ok(Entry {
        metadata: Some(metadata),
        kind_point: kind::point(&logic_ref, &label_ref)?,
        logic_ref,
        label_ref,
    })
}

/// The logic ref of a circuit version, or `None` if this generator does not pin it. A circuit release joins
/// as a renamed dependency and one more arm, so a succession can only name a version whose ID is compiled in.
fn logic_ref(circuit: Circuit, version: &str, versions: &Versions) -> Option<Digest> {
    match circuit {
        Circuit::Erc20 if version == versions.transfer => {
            Some(digest(transfer_library::TOKEN_TRANSFER_ID.as_bytes()))
        }
        Circuit::GenericCall if version == versions.generic_call => Some(digest(
            anoma_generic_call_library::GENERIC_CALL_ID.as_bytes(),
        )),
        _ => None,
    }
}

/// The circuit versions behind the logic refs.
struct Versions {
    padding: String,
    transfer: String,
    generic_call: String,
}

/// Reads the circuit versions from the resolved dependency graph, so bumping a pin cannot leave a stale
/// version in the metadata.
fn versions() -> Result<Versions> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let output = Command::new(cargo)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--all-features",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .context("failed to run cargo metadata")?;
    if !output.status.success() {
        bail!(
            "cargo metadata: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let metadata: Value =
        serde_json::from_slice(&output.stdout).context("cargo metadata output")?;

    let version_by_id: BTreeMap<&str, &str> = metadata["packages"]
        .as_array()
        .context("cargo metadata reported no packages")?
        .iter()
        .filter_map(|package| Some((package["id"].as_str()?, package["version"].as_str()?)))
        .collect();

    let node = metadata["resolve"]["nodes"]
        .as_array()
        .context("cargo metadata reported no resolve graph")?
        .iter()
        .find(|node| {
            node["id"]
                .as_str()
                .is_some_and(|id| id.contains("anoma-kind-tables"))
        })
        .context("this package is missing from the resolve graph")?;

    let version_of = |lib: &str| -> Result<String> {
        node["deps"]
            .as_array()
            .context("the resolve node carries no dependencies")?
            .iter()
            .find(|dep| dep["name"].as_str() == Some(lib))
            .and_then(|dep| version_by_id.get(dep["pkg"].as_str()?).copied())
            .map(str::to_string)
            .with_context(|| format!("{lib} is not a resolved dependency"))
    };

    Ok(Versions {
        padding: version_of("anoma_rm_risc0")?,
        transfer: version_of("transfer_library")?,
        generic_call: version_of("anoma_generic_call_library")?,
    })
}

/// The successor's metadata: the predecessor's identity under the new version, pointing back at it.
fn succeeded(
    metadata: &Metadata,
    version: &str,
    status: Option<Status>,
    alias_of: AliasOf,
) -> Metadata {
    match metadata {
        Metadata::Erc20 {
            name,
            token,
            forwarder,
            ..
        } => Metadata::Erc20 {
            version: version.to_string(),
            name: name.clone(),
            token: *token,
            forwarder: *forwarder,
            status,
            alias_of: Some(alias_of),
        },
        Metadata::GenericCall { forwarder, .. } => Metadata::GenericCall {
            version: version.to_string(),
            forwarder: *forwarder,
            status,
            alias_of: Some(alias_of),
        },
        Metadata::Padding { .. } => metadata.clone(),
    }
}

fn chain_entries(
    environment: Environment,
    chain: alloy_chains::NamedChain,
    successions: &[Succession],
    vulnerable: &[Vulnerable],
    versions: &Versions,
) -> Result<Vec<Entry>> {
    let padding_logic = digest(anoma_rm_risc0::constants::PADDING_LOGIC_VK.as_bytes());
    let transfer_logic = digest(transfer_library::TOKEN_TRANSFER_ID.as_bytes());
    let generic_call_logic = digest(anoma_generic_call_library::GENERIC_CALL_ID.as_bytes());

    let is_vulnerable = |circuit: Circuit, version: &str| {
        vulnerable
            .iter()
            .any(|marked| marked.circuit == circuit && marked.version == version)
    };
    // Active is the absence of both a mark and a succession moving past the version.
    let status = |circuit: Circuit, version: &str| -> Option<Status> {
        if is_vulnerable(circuit, version) {
            Some(Status::Vulnerable)
        } else if successions
            .iter()
            .any(|succession| succession.circuit == circuit && succession.of == version)
        {
            Some(Status::Deprecated)
        } else {
            None
        }
    };

    let mut entries = vec![entry(
        Metadata::Padding {
            version: versions.padding.clone(),
            status: None,
        },
        padding_logic,
        Digest::default(),
    )?];

    if let Some(forwarder) =
        anoma_generic_call_forwarder_bindings::addresses::generic_call_forwarder_address(
            generic_call_environment(environment),
            &chain,
        )
    {
        entries.push(entry(
            Metadata::GenericCall {
                version: versions.generic_call.clone(),
                forwarder,
                status: status(Circuit::GenericCall, &versions.generic_call),
                alias_of: None,
            },
            generic_call_logic,
            sha256(forwarder.as_slice()),
        )?);
    }

    let supported = tokens::on(chain);
    match anomapay_erc20_forwarder_bindings::addresses::erc20_forwarder_address(
        erc20_environment(environment),
        &chain,
    ) {
        Some(forwarder) => {
            for token in supported {
                entries.push(entry(
                    Metadata::Erc20 {
                        version: versions.transfer.clone(),
                        name: token.symbol.clone(),
                        token: token.address,
                        forwarder,
                        status: status(Circuit::Erc20, &versions.transfer),
                        alias_of: None,
                    },
                    transfer_logic,
                    sha256(&[forwarder.as_slice(), token.address.as_slice()].concat()),
                )?);
            }
        }
        None if supported.is_empty() => {}
        None => eprintln!(
            "{chain}: {} supported tokens but no ERC20 forwarder recorded - no token kinds emitted",
            supported.len()
        ),
    }

    // A succession carries every kind of its predecessor to its successor, so no token and no chain is left
    // behind by omission. The successor takes the predecessor's point and never the reverse: see ADR-0008.
    for succession in successions {
        let Succession {
            circuit, alias, of, ..
        } = succession;
        for version in [of, alias] {
            if is_vulnerable(*circuit, version) {
                bail!("succession {of} -> {alias}: {version} is recorded as vulnerable");
            }
        }
        let resolve = |version: &str| -> Result<Digest> {
            logic_ref(*circuit, version, versions).with_context(|| {
                format!("succession {of} -> {alias}: {version} is not pinned by this generator")
            })
        };
        let (of_logic, alias_logic) = (resolve(of)?, resolve(alias)?);
        let status = status(*circuit, alias);

        let carried: Vec<Entry> = entries
            .iter()
            .filter(|entry| entry.logic_ref == of_logic && entry.is_canonical())
            .map(|canonical| Entry {
                metadata: canonical.metadata.as_ref().map(|metadata| {
                    succeeded(
                        metadata,
                        alias,
                        status,
                        AliasOf {
                            version: of.clone(),
                            logic_ref: of_logic,
                            label_ref: canonical.label_ref,
                        },
                    )
                }),
                kind_point: canonical.kind_point.clone(),
                logic_ref: alias_logic,
                label_ref: canonical.label_ref,
            })
            .collect();
        entries.extend(carried);
    }

    entries.sort_by_key(Entry::key);
    if entries
        .windows(2)
        .any(|pair| pair[0].key() == pair[1].key())
    {
        bail!("{chain}: duplicate entry key");
    }
    Ok(entries)
}

fn main() -> Result<()> {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let successions: Successions =
        serde_json::from_str(&fs::read_to_string(data.join("successions.json"))?)
            .context("successions.json")?;
    let vulnerabilities: Vulnerabilities =
        serde_json::from_str(&fs::read_to_string(data.join("vulnerabilities.json"))?)
            .context("vulnerabilities.json")?;

    let versions = versions()?;

    // Everything is generated before anything is written: a rejected alias must not leave the tree without
    // the tables the crate embeds.
    let mut generated = Vec::new();
    for (environment, name) in [
        (Environment::Staging, "staging"),
        (Environment::Production, "production"),
    ] {
        let mut chains: Vec<_> = protocol_adapter_deployments_map(environment)
            .into_keys()
            .collect();
        chains.sort();

        let mut files = Vec::new();
        let mut commitments = BTreeMap::new();
        for chain in chains {
            let entries = chain_entries(
                environment,
                chain,
                &successions.successions,
                &vulnerabilities.vulnerable,
                &versions,
            )?;
            commitments.insert(
                chain as u64,
                ChainCommitment {
                    comment: chain.to_string(),
                    commitment: hex::encode(commitment::of(&entries).as_bytes()),
                },
            );
            files.push((
                format!("{}.json", chain as u64),
                serde_json::to_string_pretty(&entries)? + "\n",
            ));
        }
        files.push((
            "commitments.json".to_string(),
            serde_json::to_string_pretty(&commitments)? + "\n",
        ));
        generated.push((name, files, commitments.len()));
    }

    for (name, files, chains) in generated {
        let out = data.join("generated").join(name);
        if out.exists() {
            fs::remove_dir_all(&out)?;
        }
        fs::create_dir_all(&out)?;
        for (file, contents) in files {
            fs::write(out.join(file), contents)?;
        }
        println!("{name}: {chains} chains");
    }
    Ok(())
}
