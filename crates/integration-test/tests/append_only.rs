//! Kinds only accumulate. A resource's kind is bound into its commitment when it is created, so a key that
//! resolves to one point today and another tomorrow strands every resource created under it. This compares
//! each table against its published form — the base branch of the pull request, or `origin/next` locally —
//! and fails when an existing key's kind changed. A version recorded in `data/vulnerabilities.json` is the
//! one exemption, because freezing its resources is the point: see ADR-0008.

use anoma_kind_tables::{Status, Table, table};
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the crate lives inside the repository")
}

fn base() -> String {
    std::env::var("APPEND_ONLY_BASE")
        .or_else(|_| std::env::var("GITHUB_BASE_REF").map(|branch| format!("origin/{branch}")))
        .unwrap_or_else(|_| "origin/next".to_string())
}

/// The file as the base branch has it, or `None` when the base does not carry it yet.
fn published(base: &str, path: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["show", &format!("{base}:{path}")])
        .current_dir(repository())
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

#[test]
fn published_keys_keep_their_kinds() -> Result<()> {
    let base = base();
    if published(&base, "README.md").is_none() {
        eprintln!("skipped: {base} is not available");
        return Ok(());
    }

    for (environment, tables) in [
        ("staging", table::staging::tables()),
        ("production", table::production::tables()),
    ] {
        for (chain, current) in tables {
            let path = format!(
                "crates/kind-tables/data/generated/{environment}/{}.json",
                *chain as u64
            );
            let Some(contents) = published(&base, &path) else {
                continue; // The chain is new on this branch, so it has nothing to contradict.
            };
            let published: BTreeMap<_, _> = Table::from_json(&contents)
                .with_context(|| format!("{path} at {base}"))?
                .entries
                .into_iter()
                .map(|entry| ((entry.logic_ref, entry.label_ref), entry.kind_point))
                .collect();

            for entry in &current.entries {
                let Some(was) = published.get(&(entry.logic_ref, entry.label_ref)) else {
                    continue; // A new key is free to name any kind.
                };
                if entry.metadata.as_ref().and_then(|m| m.status()) == Some(Status::Vulnerable) {
                    continue;
                }
                ensure!(
                    *was == entry.kind_point,
                    "{path}: the kind of ({}, {}) changed, stranding every resource created under it",
                    entry.logic_ref,
                    entry.label_ref
                );
            }
        }
    }
    Ok(())
}
