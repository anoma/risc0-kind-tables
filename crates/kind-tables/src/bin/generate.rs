//! Regenerates `data/generated/` from the authored inputs (`tokens.json`, `circuit-versions.json`) and the
//! pinned dependencies (the generic-call logic ref, the forwarder and protocol adapter deployment
//! records). CI reruns this and fails on any diff, so the committed artifacts always match the pins.

use alloy::primitives::Address;
use alloy_chains::NamedChain;
use anoma_generic_call_forwarder_bindings::addresses::Environment as GenericCallEnvironment;
use anoma_pa_evm_bindings::addresses::{Environment, protocol_adapter_deployments_map};
use anoma_risc0_kind_tables::{
    AliasOf, Caip2ChainId, Chain, CircuitVersion, Entry, Metadata, SolanaAddress, SolanaCluster,
    Status, circuits, commitment, deployments, kind, tokens,
};
use anomapay_erc20_forwarder_bindings::addresses::Environment as Erc20Environment;
use anyhow::{Context, Result, bail, ensure};
use risc0_zkvm::Digest;
use risc0_zkvm::sha::{Impl, Sha256};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

/// One row of `commitments.json`: the chain it belongs to, named for review, and the table commitment.
#[derive(Serialize)]
struct ChainCommitment {
    #[serde(rename = "_comment")]
    comment: String,
    commitment: String,
}

/// The immutable ERC20 forwarder of a chain's v1 protocol adapter. Under its label, the logic ref it accepts is a
/// member of every ERC20 fungibility domain of the chain, so its resources convert and leave through the current
/// forwarder.
struct V1Forwarder {
    address: Address,
    /// The logic ref it accepts, the only one its resources carry.
    logic_ref: Digest,
}

/// The V1 ERC20 forwarder a chain records, from the forwarder repository's deployment record, where its fork tests
/// verify it against the chain.
fn v1_erc20_forwarder(chain: &NamedChain) -> Option<V1Forwarder> {
    anomapay_erc20_forwarder_bindings::addresses::erc20_forwarder_v1(chain).map(|forwarder| {
        V1Forwarder {
            address: forwarder.address,
            logic_ref: digest(forwarder.logic_ref.as_slice()),
        }
    })
}

fn digest(bytes: &[u8]) -> Digest {
    Digest::try_from(bytes).expect("a logic ref is 32 bytes")
}

fn sha256(bytes: &[u8]) -> Digest {
    *Impl::hash_bytes(bytes)
}

fn hex(digest: &Digest) -> String {
    hex::encode(digest.as_bytes())
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

/// An entry outside every fungibility domain: it is assigned its own kind.
fn derived(metadata: Metadata, logic_ref: Digest, label_ref: Digest) -> Result<Entry> {
    Ok(Entry {
        metadata: Some(metadata),
        kind_point: kind::point(&logic_ref, &label_ref)?,
        logic_ref,
        label_ref,
    })
}

/// A member of a token's fungibility domain: one circuit version under one forwarder's label, assigned the
/// fungibility domain's kind point. `alias_of` names the kind that point is; only the active version under the
/// current forwarder's label carries none, and only that member is active. `metadata` names the token and
/// forwarder the member belongs to, given the member's version, status and `alias_of`.
fn member(
    circuit: &CircuitVersion,
    label_ref: Digest,
    domain_point: &[u8],
    alias_of: Option<AliasOf>,
    metadata: impl FnOnce(String, Status, Option<AliasOf>) -> Metadata,
) -> Entry {
    let status = if alias_of.is_none() {
        Status::Active
    } else {
        Status::Deprecated
    };
    Entry {
        metadata: Some(metadata(circuit.version.clone(), status, alias_of)),
        logic_ref: circuit.logic_ref,
        label_ref,
        kind_point: domain_point.to_vec(),
    }
}

/// One token's fungibility domain under the current forwarder's label `label_ref`: every deprecated circuit version as
/// an alias of the active version under that label, and the active version itself, assigned its own kind, only if
/// `precompute` is set for the token. Returns the domain's kind point and the kind every alias names, for members added
/// under another label.
fn domain(
    listed: &[CircuitVersion],
    active: &CircuitVersion,
    label_ref: Digest,
    precompute: bool,
    metadata: impl Fn(String, Status, Option<AliasOf>) -> Metadata,
    entries: &mut Vec<Entry>,
) -> Result<(Vec<u8>, AliasOf)> {
    let point = kind::point(&active.logic_ref, &label_ref)?;
    let active_kind = AliasOf {
        version: active.version.clone(),
        logic_ref: active.logic_ref,
        label_ref,
    };
    for circuit in listed {
        let alias_of = (circuit.status == Status::Deprecated).then(|| active_kind.clone());
        if alias_of.is_none() && !precompute {
            continue;
        }
        entries.push(member(circuit, label_ref, &point, alias_of, &metadata));
    }
    Ok((point, active_kind))
}

/// A table's entries in table order, sorted by kind, with no kind listed twice.
fn sorted(mut entries: Vec<Entry>, chain: &Chain) -> Result<Vec<Entry>> {
    entries.sort_by_key(Entry::key);
    if entries
        .windows(2)
        .any(|pair| pair[0].key() == pair[1].key())
    {
        bail!("{chain}: duplicate kind");
    }
    Ok(entries)
}

/// The circuit version behind the generic-call logic ref, and those of the pinned ERC20 and SPL token crates.
struct Versions {
    transfer: String,
    generic_call: String,
    spl_transfer: String,
    spl_transfer_rc2: String,
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
                .is_some_and(|id| id.contains(env!("CARGO_PKG_NAME")))
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
        transfer: version_of("transfer_library")?,
        generic_call: version_of("anoma_generic_call_library")?,
        spl_transfer: version_of("anomapay_solana_transfer_library")?,
        spl_transfer_rc2: version_of("anomapay_solana_transfer_library_rc2")?,
    })
}

/// The ERC20 circuit crates this generator pins, each with the logic ref it compiles to. A release joins as
/// a renamed dependency and one more line here. A version whose crate is no longer pinned stays listed in
/// `circuit-versions.json` and is checked against nothing.
fn pinned_erc20_circuits(versions: &Versions) -> Vec<(String, Digest)> {
    vec![(
        versions.transfer.clone(),
        digest(transfer_library::TOKEN_TRANSFER_ID.as_bytes()),
    )]
}

/// The SPL token circuit crates this generator pins, as `pinned_erc20_circuits` for the Solana transfer circuit.
fn pinned_spl_token_circuits(versions: &Versions) -> Vec<(String, Digest)> {
    vec![
        (
            versions.spl_transfer.clone(),
            digest(anomapay_solana_transfer_library::TOKEN_TRANSFER_ID.as_bytes()),
        ),
        (
            versions.spl_transfer_rc2.clone(),
            digest(anomapay_solana_transfer_library_rc2::TOKEN_TRANSFER_ID.as_bytes()),
        ),
    ]
}

/// Each list must pass its own checks, and every pinned circuit crate must be listed with the logic ref it
/// compiles to. Raising a pin without listing the release stops here, and so does a mistyped logic ref.
fn check_circuit_versions(versions: &Versions) -> Result<()> {
    for (resource, check, listed, pinned) in [
        (
            "ERC20Resource",
            circuits::check_erc20 as fn() -> std::result::Result<(), String>,
            circuits::erc20(),
            pinned_erc20_circuits(versions),
        ),
        (
            "SPLTokenResource",
            circuits::check_spl_token,
            circuits::spl_token(),
            pinned_spl_token_circuits(versions),
        ),
    ] {
        check().map_err(|error| anyhow::anyhow!("circuit-versions.json: {resource}: {error}"))?;
        for (version, compiled) in pinned {
            let listed = listed
                .iter()
                .find(|circuit| circuit.version == version)
                .with_context(|| {
                    format!("a {resource} circuit crate at {version} is pinned, but circuit-versions.json does not list it")
                })?;
            ensure!(
                listed.logic_ref == compiled,
                "circuit-versions.json records {resource} {version} with logic ref {}, but the pinned crate compiles to {}",
                hex(&listed.logic_ref),
                hex(&compiled)
            );
        }
    }
    Ok(())
}

/// The Solana cluster an environment's table is built for: devnet is staging, mainnet-beta is production.
fn solana_cluster(environment: Environment) -> SolanaCluster {
    match environment {
        Environment::Staging => SolanaCluster::Devnet,
        Environment::Production => SolanaCluster::MainnetBeta,
    }
}

/// A Solana cluster's entries: one fungibility domain per supported mint under the forwarder program's label.
/// As on EVM chains, the padding kind is not listed. Solana has no generic call forwarder, so no generic call
/// entry.
fn solana_entries(cluster: SolanaCluster, forwarder: SolanaAddress) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for token in tokens::spl_on(cluster) {
        domain(
            circuits::spl_token(),
            circuits::spl_token_active(),
            kind::spl_token_label_ref(&forwarder, &token.mint),
            token.precompute_kind_point,
            |version, status, alias_of| Metadata::SplToken {
                version,
                name: token.symbol.clone(),
                mint: token.mint,
                forwarder,
                status,
                alias_of,
            },
            &mut entries,
        )?;
    }
    Ok(entries)
}

fn chain_entries(
    environment: Environment,
    chain: NamedChain,
    versions: &Versions,
) -> Result<Vec<Entry>> {
    // The padding kind is not listed: its resources never convert and need no precomputed kind point, and its logic
    // ref would tie every table to the arm-risc0 release.
    let mut entries = Vec::new();

    if let Some(forwarder) =
        anoma_generic_call_forwarder_bindings::addresses::generic_call_forwarder_address(
            generic_call_environment(environment),
            &chain,
        )
    {
        entries.push(derived(
            Metadata::GenericCall {
                version: versions.generic_call.clone(),
                forwarder,
            },
            digest(anoma_generic_call_library::GENERIC_CALL_ID.as_bytes()),
            sha256(forwarder.as_slice()),
        )?);
    }

    // One fungibility domain per token, assigned the kind of the active version under the current forwarder's label:
    // every deprecated circuit version under that label and, for a token the list marks for conversion, the V1
    // forwarder's logic ref under its own label, as aliases. The active version keeps its own kind, which the circuit
    // computes without the table, so it gets a row only if the token asks for a precomputed kind point.
    let supported = tokens::on(chain);
    match anomapay_erc20_forwarder_bindings::addresses::erc20_forwarder_address(
        erc20_environment(environment),
        &chain,
    ) {
        Some(current) => {
            let v1 = v1_erc20_forwarder(&chain);
            for token in supported {
                let erc20 = |forwarder: Address| {
                    move |version, status, alias_of| Metadata::Erc20 {
                        version,
                        name: token.symbol.clone(),
                        token: token.address,
                        forwarder,
                        status,
                        alias_of,
                    }
                };
                let (point, active_kind) = domain(
                    circuits::erc20(),
                    circuits::erc20_active(),
                    kind::erc20_label_ref(&current, &token.address),
                    token.precompute_kind_point,
                    erc20(current),
                    &mut entries,
                )?;
                if token.fungible_with_v1
                    && let Some(v1) = &v1
                {
                    let circuit = circuits::erc20_version(&v1.logic_ref).with_context(|| {
                        format!(
                            "{chain}: the V1 forwarder {} accepts logic ref {}, which circuit-versions.json does not list",
                            v1.address,
                            hex(&v1.logic_ref)
                        )
                    })?;
                    entries.push(member(
                        circuit,
                        kind::erc20_label_ref(&v1.address, &token.address),
                        &point,
                        Some(active_kind),
                        erc20(v1.address),
                    ));
                }
            }
        }
        None if supported.is_empty() => {}
        None => eprintln!(
            "{chain}: {} supported tokens but no ERC20 forwarder recorded - no token kinds emitted",
            supported.len()
        ),
    }

    Ok(entries)
}

fn main() -> Result<()> {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");

    let versions = versions()?;
    check_circuit_versions(&versions)?;

    // Everything is generated before anything is written: a rejected input must not leave the tree without
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

        let mut tables = Vec::new();
        for chain in chains {
            tables.push((
                Chain::Evm(chain),
                chain_entries(environment, chain, &versions)?,
            ));
        }
        let cluster = solana_cluster(environment);
        match deployments::solana_deployment(cluster) {
            Some(deployment) => tables.push((
                Chain::Solana(cluster),
                solana_entries(cluster, deployment.forwarder)?,
            )),
            None if tokens::spl_on(cluster).is_empty() => {}
            None => eprintln!(
                "{}: {} supported mints but no deployment recorded - no token kinds emitted",
                cluster.name(),
                tokens::spl_on(cluster).len()
            ),
        }

        let mut files = Vec::new();
        let mut commitments = BTreeMap::new();
        for (chain, entries) in tables {
            let entries = sorted(entries, &chain)?;
            let id = Caip2ChainId::from(chain);
            files.push((
                id.file_name(),
                serde_json::to_string_pretty(&entries)? + "\n",
            ));
            commitments.insert(
                id,
                ChainCommitment {
                    comment: chain.to_string(),
                    commitment: hex::encode(commitment::of(&entries).as_bytes()),
                },
            );
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
