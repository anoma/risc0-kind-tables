# Show commands before running (helps debug failures)
set shell := ["bash", "-euo", "pipefail", "-c"]

# Recipes read `ALCHEMY_API_KEY` (token validation, freshness gate, e2e) and the queue's
# `QUEUE_BASE_URL` and `QUEUE_AUTH_TOKEN` (e2e) from the environment. The file is absent
# in CI, where the values come from secrets.
set dotenv-load := true
set dotenv-required := false

GREEN := "\\033[0;32m"
RED := "\\033[0;31m"
NORMAL := "\\033[0m"

# Default recipe
default:
    @just --list

# --- Generation ---

# Regenerate `data/generated/` from the authored inputs and the pinned dependencies
generate:
    cargo run --package anoma-risc0-kind-tables --features generate --bin generate

# Verify the committed generated tables match a regeneration
generate-check: generate
    #!/usr/bin/env bash
    set -euo pipefail
    if ! git diff --exit-code -- crates/kind-tables/data/generated; then
        printf '{{RED}}The generated tables are stale. Run `just generate` and commit the diff.{{NORMAL}}\n'
        exit 1
    fi
    printf '{{GREEN}}The generated tables match the source.{{NORMAL}}\n'

# --- Crates ---

# Clean all crates
crates-clean:
    cargo clean

# Build all crates
crates-build *args:
    cargo build {{ args }}

# Test all crates
crates-test *args:
    cargo test {{ args }}

# Test the e2e cases on a fork of `E2E_CHAIN_ID` (Sepolia by default), proven by the queue at `QUEUE_BASE_URL`.
# One thread, unlike the other tests: the queue's CDN blocks bursts.
crates-test-e2e *args:
    RUST_TEST_THREADS=1 cargo test --features e2e e2e_test {{ args }}

# Publish anoma-risc0-kind-tables to crates.io
crates-publish *args:
    cargo publish --package anoma-risc0-kind-tables {{ args }}

# Lint all crates (clippy)
crates-lint:
    cargo clippy --all-targets --no-deps -- -Dwarnings
    cargo clippy --all-targets --features generate --no-deps -- -Dwarnings
    cargo clippy --all-targets --features e2e --no-deps -- -Dwarnings

# Format all crates
crates-fmt *args:
    cargo fmt --all {{ args }}

# Check all crates formatting
crates-fmt-check:
    cargo fmt --all -- --check
