# Anoma Kind Tables

[![Crates](https://github.com/anoma/risc0-kind-tables/actions/workflows/crates.yml/badge.svg)](https://github.com/anoma/risc0-kind-tables/actions/workflows/crates.yml)

The kind tables the Anoma protocol adapters are committed to — one per chain per environment — and the supported-token list they are built from. The `anoma-risc0-kind-tables` crate computes the roots the way the compliance circuit does, so one source answers what every protocol adapter must store.

## How it fits together

A kind table maps kinds, written as `(logic_ref, label_ref)`, to kind points. Its root — the Merkle root that arm-risc0 builds over the entries — is what a protocol adapter stores via `setKindTableRoot` and what every compliance proof that takes a kind from the table reproduces. A chain's table is derived: the generic call entry from the recorded forwarder, and the ERC20 entries of each supported token from the recorded ERC20 forwarder. The padding kind has no entry: nothing converts padding resources and their speed does not matter, so the compliance circuit computes their kind point, and the tables do not depend on the arm-risc0 release (ADR-0008). The ERC20 entries of one token form a fungibility domain: every listed circuit version under the forwarder's label and, for a token marked for soft migration on a chain that ran a legacy protocol adapter, the legacy ERC20 forwarder's logic ref under its own label, all assigned the kind of the active version under the label of the environment's forwarder, so their resources are fungible. The aliases are always rows. The active version keeps its own kind, which the circuit computes without the table, so it has a row only if the token sets `precompute_kind_point`: the circuit then reads its kind point instead of computing it by hash to curve, and every proof hashes one more row. Every entry is machine-checked — an alias against its fungibility domain, every other entry against its own kind — and no kind point is authored.

A Solana cluster's table is built the same way from different records. Each supported SPL token mint is one fungibility domain under the label `sha256(forwarder program id ‖ mint)`, with every listed SPL token circuit version as a member. The forwarder program comes from the deployment record in `anomapay-spl-token-forwarder-client` and the transfer logic ref from `anomapay-solana-resource`, both pinned by git tag. Solana has no generic call forwarder and no V1 forwarder, so its tables have neither entry (ADR-0010).

## Layout

```
crates/kind-tables/            the library and the generator
├── data/
│   ├── tokens.json            authored: the supported tokens, per chain
│   ├── circuit-versions.json  authored: the ERC20 and SPL token circuit versions, once for every chain
│   └── generated/
│       ├── staging/           <namespace>_<reference>.json tables + roots.json
│       └── production/
└── src/
crates/integration-test/       on-chain token, forwarder and Solana cluster checks, promotion freshness and soft-migration gates, arm-risc0 cross-check
docs/adr/                      the decisions behind this layout
```

Every chain-keyed file — `tokens.json`, `roots.json` — is keyed by the chain's CAIP-2 chain ID, such as `eip155:11155111` for Sepolia, so that a chain of another family than EVM, such as Solana, has a key too. A Solana cluster's reference is the first 32 characters of its base58 genesis hash: devnet is `solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1`. Each section carries the chain name in a `_comment` the loaders ignore. A generated table is named for the same ID with the colon replaced by `_`, such as `eip155_11155111.json`, because `cargo package` refuses a colon in a file name (ADR-0009). It is otherwise `anoma-rm-risc0`'s kind table schema, so `init_kind_table_from_file` reads one unchanged. The lookups `table`, `root` and `tokens::on` take a `Caip2ChainId`, an `alloy_chains::NamedChain` or a `SolanaCluster`.

The `solana-deployments` feature exposes the recorded Solana deployment of each cluster (the protocol adapter and SPL token forwarder programs), read from `anoma-pa-solana-client` and `anomapay-spl-token-forwarder-client`. The generator and the integration tests use it; a consumer that only reads tables does not need it.

The `arm` feature converts an `Entry` into `anoma-rm-risc0`'s `KindTableEntry`, so a prover loads a recorded table with `init_kind_table_from_entries`, without a file.

## Entries

An entry carries its kind, the kind point it is assigned, and a `_metadata` object naming what the kind belongs to. The root covers `logic_ref`, `label_ref` and `kind_point` only, so `_metadata` is free to carry whatever a reviewer needs and never moves the root. `version` is the version of the circuit crate that owns `logic_ref`, read from the resolved dependency graph, so bumping a pin cannot leave a stale version behind. `type` names the resource the kind belongs to: `ERC20Resource`, `SPLTokenResource`, or `GenericCallResource`. An `SPLTokenResource` row carries `mint` and `forwarder` as base58 addresses where an ERC20 row carries `token` and `forwarder`.

```json
{
  "_metadata": {
    "type": "ERC20Resource",
    "version": "2.0.0",
    "name": "WETH",
    "token": "0x4200000000000000000000000000000000000006",
    "forwarder": "0xE54182d915dE447deFc4A17Ec1D4E0dc627551F7"
  },
  "logic_ref": "bc12323668c37c3d381ca798f11116f35fb1639d12239b29da7810df3985e7ad",
  "label_ref": "55008fad9bfccef776960bfca715e365cfba843cfb947190fafc69e4d9fac674",
  "kind_point": "047494d66fd5abf01f4eb01d204f9c288db95873e6696cc0c5e143855d2a36000fa2c0ba7484f59110ffdfdd546e064c595d0ab979f09c48c26285f3aba5dc2471"
}
```

That row is the active member of the WETH fungibility domain on Base Sepolia. A fungibility domain is one forwarder holding one token, and every listed circuit version has a member under that forwarder's label, all carrying the same `kind_point`. The kind point is the kind of the active version under that label, so every deprecated version's row is an alias of it; a version outside the fungibility domain is fungible with nothing. The active version's row assigns its own kind, which the circuit computes anyway, so it is listed only because WETH sets `precompute_kind_point`. Generic call has no fungibility domain, and its row assigns its own kind for the same reason: the circuit reads its kind point instead of computing it.

An alias also says what it is a second name for. Its `alias_of` names the kind it takes its kind point from, so a reviewer reads it without recomputing anything, and its `status` is `deprecated`: a member is `active` if and only if it has no `alias_of`. When version 3.0.0 becomes active, 2.0.0 becomes deprecated, every kind point moves to 3.0.0's kind, and the WETH row for 2.0.0 becomes this:

```json
{
  "_metadata": {
    "type": "ERC20Resource",
    "version": "2.0.0",
    "name": "WETH",
    "token": "0x4200000000000000000000000000000000000006",
    "forwarder": "0xE54182d915dE447deFc4A17Ec1D4E0dc627551F7",
    "status": "deprecated",
    "alias_of": {
      "version": "3.0.0",
      "logic_ref": "<the transfer logic ref of version 3.0.0>",
      "label_ref": "55008fad9bfccef776960bfca715e365cfba843cfb947190fafc69e4d9fac674"
    }
  },
  "logic_ref": "bc12323668c37c3d381ca798f11116f35fb1639d12239b29da7810df3985e7ad",
  "label_ref": "55008fad9bfccef776960bfca715e365cfba843cfb947190fafc69e4d9fac674",
  "kind_point": "<3.0.0's kind under this label, the same as the new 3.0.0 row's>"
}
```

Members are never authored. A circuit release is one appended line in [`crates/kind-tables/data/circuit-versions.json`](crates/kind-tables/data/circuit-versions.json):

```json
{
  "ERC20Resource": [
    { "version": "2.0.0", "logic_ref": "bc12323668c37c3d381ca798f11116f35fb1639d12239b29da7810df3985e7ad", "status": "deprecated" },
    { "version": "3.0.0", "logic_ref": "<the transfer logic ref of version 3.0.0>", "status": "active" }
  ]
}
```

The generator then writes one more member per token on every chain that has the token, so neither a token nor a chain can be left out, and checks every version whose crate is pinned against the logic ref that crate compiles to. A deprecated version leaves the list only when no resource of it exists under the label of a recorded forwarder. Until then it stays, even when the protocol adapter denies its logic ref, with rows that nothing can use (ADR-0008).

## Workflows

Adding a token, listing a circuit version and recording a V1 forwarder all change a chain's entries, and therefore change its root. A transaction built against the new table verifies only after that chain's protocol adapter stores the new root, so all of them end in *Update the deployed roots*.

### Add a token

1. Add the token to [`crates/kind-tables/data/tokens.json`](crates/kind-tables/data/tokens.json), under the CAIP-2 chain ID of the chain it lives on. An ERC20 token records `address` and `soft_migrate_from_legacy_forwarder`; an SPL token records `mint` (base58) and has no such flag. Every token records `precompute_kind_point`: `true` gives its active kind a row, so proofs of its resources skip hash to curve, and every proof hashes one more row. Set it for the tokens that most transactions carry.
2. Run `just generate`.
3. Commit the edited file together with the regenerated tables.

The token validation test runs on every pull request and push. It calls an ERC20 contract and checks that it reports the recorded symbol, name and decimals. A mint reports only its decimals on chain, so for an SPL token it checks that the mint account exists with the recorded decimals; symbol and name are review context.

### Add a circuit version

Listing a version makes its resources fungible with every other listed version's, behind the same forwarder, one for one. Nothing passes from one version to another: both kinds are assigned one kind point. Exactly one version is active — under the label of the environment's forwarder it keeps its own kind, and the backend creates its resources — and it must be the highest listed; every other version is deprecated, an alias of the active one, which the backend only consumes. Making a version active moves every ERC20 kind point on every chain to its kind. Review the change as carefully as a permission to create new tokens.

The generator pins the crate of the active version and the crate of the legacy circuit, and checks that the logic ref each compiles to is listed. Any other deprecated version stays listed and is checked against nothing.

1. Pin the release in `[workspace.dependencies]` in [`Cargo.toml`](Cargo.toml) in place of the previous one:
   ```toml
   anomapay_erc20_library = "=2.0.0-rc.1"
   ```
2. If the release has a new crate name, use it in [`crates/kind-tables/Cargo.toml`](crates/kind-tables/Cargo.toml), as the optional dependency and in the `generate` feature list, and in `versions()` and `pinned_erc20_circuits` in [`crates/kind-tables/src/bin/generate.rs`](crates/kind-tables/src/bin/generate.rs).
3. In [`crates/kind-tables/data/circuit-versions.json`](crates/kind-tables/data/circuit-versions.json), append the version with `"status": "active"` and set the previous active version to `"deprecated"`.
4. Run `just generate`. Read the diff: one new row per token on every chain, and every older version's rows now alias the new one.
5. Commit the edited files together with the regenerated tables.

Old resources leave through the new version once the forwarder accepts it, which is an upgrade in the forwarder repository. The promotion gate reads each chain's forwarder and requires that the logic ref it accepts is listed.

An SPL token circuit release appends its version to `SPLTokenResource` in `circuit-versions.json` and moves the generator's pin of `anomapay-solana-resource`'s `transfer_library` (`anomapay-solana-transfer-library`) to the tag that ships it, so the generator checks the active version's logic ref. On Solana the forwarder accepts the new logic ref after its upgrade, whose `reinitialize` installs it; the promotion gate reads the forwarder's config account for it.

### Alias the legacy ERC20 forwarder

On a chain that ran a legacy protocol adapter, the ERC20 forwarder changes once: the legacy ERC20 forwarder is immutable, and the environment's forwarder is a proxy at a new address. Both accept the same logic ref, so a legacy-forwarder resource differs from a resource of the environment's forwarder only in the forwarder inside its label. On every chain that records both forwarders, the generator writes one member under the legacy forwarder's label for each token the list marks for soft migration, an alias of the active version under the label of the environment's forwarder.

1. Record the legacy ERC20 forwarder, with the logic ref it accepts, in the `legacy` array of the forwarder repository's `deployments.json`, and release the forwarder bindings.
2. In [`crates/kind-tables/data/tokens.json`](crates/kind-tables/data/tokens.json), set `soft_migrate_from_legacy_forwarder` to `true` for every token whose legacy-forwarder resources must be fungible with those of the environment's forwarder, and to `false` for the rest. A token set to `false`, or missing from the list, gets no member under the legacy forwarder's label, so its legacy-forwarder resources stay in the legacy forwarder. Nothing checks this.
3. Pin the new bindings and run `just generate`. Read the diff: one member under the legacy forwarder's label per marked token on every chain that records both forwarders.
4. Commit the edited file together with the regenerated tables.

A member under the legacy forwarder's label lets a legacy-forwarder resource unwrap from the environment's forwarder. Install the table while the protocol adapter is paused, and unpause it only after the legacy forwarder's tokens moved to the environment's forwarder (ADR-0008).

### Update the deployed roots

1. Merge into `next`.
2. Read each chain's root from `crates/kind-tables/data/generated/<environment>/roots.json`.
3. Install it with the protocol adapter repo's `contracts-*-kind-table-*` recipes, which call `setKindTableRoot`. On Solana, the adapter's owner calls `set_kind_table_root`, through the Solana protocol adapter repo's `dev.sh set-kind-table --cluster <cluster>` with `PA_KIND_TABLE_ROOT` set.
4. Open the promotion pull request into `staging` or `main`. It passes only when every protocol adapter of that environment already stores the root this source generates. The soft-migration gate also proves, on a fork of Sepolia, that a legacy-forwarder resource converts and leaves through the environment's forwarder, so the environment's forwarder must already hold the tokens moved from the legacy forwarder.

## Verifying

```sh
just crates-fmt-check && just crates-build && just crates-lint && just generate-check && just crates-test
```

The tests read `ALCHEMY_API_KEY` from the environment (or `.env`, see `.env-example`). The Solana checks use the public cluster RPC unless `SOLANA_RPC_URL_DEVNET` or `SOLANA_RPC_URL_MAINNET_BETA` names another.

The e2e tests fork `E2E_CHAIN_ID` (Sepolia by default) with Foundry's `anvil` and prove through the queue at `QUEUE_BASE_URL` with `QUEUE_AUTH_TOKEN`. CI runs them only on a promotion pull request:

```sh
just crates-test-e2e
```
