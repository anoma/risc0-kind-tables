//! Regenerates `data/generated/` from the authored inputs (`tokens.json`, `aliases.json`) and the pinned
//! dependencies (circuit IDs, forwarder and protocol adapter deployment records). CI reruns this and fails on
//! any diff, so the committed artifacts always match the pins.

use alloy::primitives::Address;
use anoma_generic_call_forwarder_bindings::addresses::Environment as GenericCallEnvironment;
use anoma_kind_tables::{AliasOf, Entry, Metadata, commitment, kind, tokens};
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

/// One chain's authored section. The `_comment` naming the chain is review context and is not deserialized.
#[derive(Deserialize)]
struct ChainAliases {
    aliases: Vec<Alias>,
}

/// One row of `commitments.json`: the chain it belongs to, named for review, and the table commitment.
#[derive(Serialize)]
struct ChainCommitment {
    #[serde(rename = "_comment")]
    comment: String,
    commitment: String,
}

/// One aliasing decision: the token's kind under the `alias` circuit version takes the point it has under
/// `of`, so resources of both versions are one kind. Both versions must be pinned here, which is what makes
/// the circuit IDs derivable rather than authored.
#[derive(Deserialize)]
struct Alias {
    token: Address,
    alias: String,
    of: String,
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

/// The transfer circuits whose kinds can be derived, by crate version. A new circuit release joins this map
/// as a renamed dependency, so `aliases.json` can only name a version whose ID is compiled in.
fn transfer_circuits(versions: &Versions) -> BTreeMap<&str, Digest> {
    BTreeMap::from([(
        versions.transfer.as_str(),
        digest(transfer_library::TOKEN_TRANSFER_ID.as_bytes()),
    )])
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

fn chain_entries(
    environment: Environment,
    chain: alloy_chains::NamedChain,
    aliases: &[Alias],
    versions: &Versions,
) -> Result<Vec<Entry>> {
    let padding_logic = digest(anoma_rm_risc0::constants::PADDING_LOGIC_VK.as_bytes());
    let transfer_logic = digest(transfer_library::TOKEN_TRANSFER_ID.as_bytes());
    let generic_call_logic = digest(anoma_generic_call_library::GENERIC_CALL_ID.as_bytes());

    let mut entries = vec![entry(
        Metadata::Padding {
            version: versions.padding.clone(),
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
                alias_of: None,
            },
            generic_call_logic,
            sha256(forwarder.as_slice()),
        )?);
    }

    let supported = tokens::on(chain);
    let erc20_forwarder = anomapay_erc20_forwarder_bindings::addresses::erc20_forwarder_address(
        erc20_environment(environment),
        &chain,
    );
    match erc20_forwarder {
        Some(forwarder) => {
            for token in supported {
                entries.push(entry(
                    Metadata::Erc20 {
                        version: versions.transfer.clone(),
                        name: token.symbol.clone(),
                        token: token.address,
                        forwarder,
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

    let circuits = transfer_circuits(versions);
    for alias in aliases {
        let forwarder = erc20_forwarder
            .with_context(|| format!("{chain}: an alias is recorded but no ERC20 forwarder is"))?;
        let token = supported
            .iter()
            .find(|token| token.address == alias.token)
            .with_context(|| format!("{chain}: aliased token {} is not supported", alias.token))?;
        let circuit = |version: &str| -> Result<Digest> {
            circuits.get(version).copied().with_context(|| {
                format!("{chain}: transfer circuit {version} is not pinned by this generator")
            })
        };
        let logic_ref = circuit(&alias.alias)?;
        let of_logic_ref = circuit(&alias.of)?;
        let label_ref = sha256(&[forwarder.as_slice(), alias.token.as_slice()].concat());

        let Some(canonical) = entries
            .iter()
            .find(|e| (e.logic_ref, e.label_ref) == (of_logic_ref, label_ref) && e.is_canonical())
        else {
            bail!(
                "{chain}: {} under transfer circuit {} is not a canonical entry of this table",
                token.symbol,
                alias.of
            );
        };
        let point = canonical.kind_point.clone();

        if entries
            .iter()
            .any(|e| (e.logic_ref, e.label_ref) == (logic_ref, label_ref))
        {
            bail!(
                "{chain}: {} under transfer circuit {} already has an entry",
                token.symbol,
                alias.alias
            );
        }
        entries.push(Entry {
            metadata: Some(Metadata::Erc20 {
                version: alias.alias.clone(),
                name: token.symbol.clone(),
                token: alias.token,
                forwarder,
                alias_of: Some(AliasOf {
                    version: alias.of.clone(),
                    logic_ref: of_logic_ref,
                    label_ref,
                }),
            }),
            kind_point: point,
            logic_ref,
            label_ref,
        });
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
    let aliases: BTreeMap<u64, ChainAliases> =
        serde_json::from_str(&fs::read_to_string(data.join("aliases.json"))?)
            .context("aliases.json")?;

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
                aliases
                    .get(&(chain as u64))
                    .map_or(&[], |section| section.aliases.as_slice()),
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
