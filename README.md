# Anoma Kind Tables

[![Crates](https://github.com/anoma/risc0-kind-tables/actions/workflows/crates.yml/badge.svg)](https://github.com/anoma/risc0-kind-tables/actions/workflows/crates.yml)

The kind tables the Anoma protocol adapters are committed to — one per chain per environment — and the supported-token list they are built from. The `anoma-risc0-kind-tables` crate computes the commitments the way the compliance circuit does, so one source answers what every protocol adapter must store.

## How it fits together

A kind table maps kinds, written as `(logic_ref, label_ref)`, to kind points. Its commitment — SHA-256 over the ordered entries — is what a protocol adapter stores via `setKindTableCommitment` and what every compliance proof reproduces. A chain's table is derived: the generic call entry from the recorded forwarder, and one ERC20 entry per supported token from the recorded ERC20 forwarder. The padding kind is not listed: the circuit derives it by hash to curve, which is the point a table would assign it anyway. The ERC20 entries of one token form a fungibility domain: every listed circuit version under the forwarder's label and, for a token marked for conversion on a chain that ran v1, the V1 forwarder's logic ref under its own label, all assigned the kind of the active version under the current forwarder's label, so their resources are fungible. Every entry is machine-checked — an alias against its fungibility domain, every other entry against its own kind — and no kind point is authored.

A Solana cluster's table is built the same way from different records. Each supported SPL token mint is one fungibility domain under the label `sha256(forwarder program id ‖ mint)`, with every listed SPL token circuit version as a member. The forwarder program comes from the deployment record in `anoma-pa-solana-client` and the transfer logic ref from `anomapay-solana-resource`, both pinned by commit. Solana has no generic call forwarder and no V1 forwarder, so its tables have neither entry (ADR-0009).

## Layout

```
crates/kind-tables/            the library and the generator
├── data/
│   ├── tokens.json            authored: the supported tokens, per chain
│   ├── circuit-versions.json  authored: the ERC20 and SPL token circuit versions, once for every chain
│   └── generated/
│       ├── staging/           <chain key>.json tables + commitments.json
│       └── production/
└── src/
crates/integration-test/       on-chain token, forwarder and Solana cluster checks, promotion freshness gate, arm-risc0 cross-check
docs/adr/                      the decisions behind this layout
```

Every chain-keyed file — `tokens.json`, `commitments.json` — is keyed by the chain's own identifier, and each section carries the chain name in a `_comment` the loaders ignore. An EVM chain is keyed by its chain ID, as the forwarder and protocol adapter deployment records are. A Solana cluster has no chain ID, so it is keyed by its CAIP-2 chain ID, `solana:` followed by the first 32 characters of its base58 genesis hash (devnet is `solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1`). A generated table is named for that key, with the colon replaced by a dash (`solana-EtWTRABZaYq6iMfeYKouRu166VU2xqa1.json`), and is otherwise `anoma-rm-risc0`'s kind table schema, so `init_kind_table_from_file` reads one unchanged.

The `solana-deployments` feature exposes the recorded Solana deployment of each cluster (the protocol adapter and SPL token forwarder programs), read from `anoma-pa-solana-client`. The generator and the integration tests use it; a consumer that only reads tables does not need it.

The `arm` feature converts an `Entry` into `anoma-rm-risc0`'s `KindTableEntry`, so a prover loads a recorded table with `init_kind_table_from_entries`, without a file.

## Entries

An entry carries its kind, the kind point it is assigned, and a `_metadata` object naming what the kind belongs to. The commitment covers `logic_ref`, `label_ref` and `kind_point` only, so `_metadata` is free to carry whatever a reviewer needs and never moves the commitment. `version` is the version of the circuit crate that owns `logic_ref`, read from the resolved dependency graph, so bumping a pin cannot leave a stale version behind. `type` names the resource the kind belongs to: `ERC20Resource`, `SPLTokenResource`, or `GenericCallResource`. An `SPLTokenResource` row carries `mint` and `forwarder` as base58 addresses where an ERC20 row carries `token` and `forwarder`.

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

That row is a member of the WETH fungibility domain on Base Sepolia. A fungibility domain is one forwarder holding one token, and every listed circuit version has a member under that forwarder's label, all carrying the same `kind_point`. The kind point is the kind of the active version under that label, so that version's entry is assigned its own kind and every deprecated version's are aliases of it; a version outside the fungibility domain is fungible with nothing. Generic call has no fungibility domain and is assigned its own kind, which the circuit would use anyway.

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

The generator then writes one more member per token on every chain that has the token, so neither a token nor a chain can be left out, and checks every version whose crate is pinned against the logic ref that crate compiles to. Nothing is ever removed from the list: a version the protocol adapter refuses stays listed, with rows that nothing can use (ADR-0008).

## Workflows

Adding a token, listing a circuit version and recording a V1 forwarder all change a chain's entries, and therefore change its commitment. A transaction built against the new table verifies only after that chain's protocol adapter stores the new commitment, so all of them end in *Update the deployed commitments*.

### Add a token

1. Add the token to [`crates/kind-tables/data/tokens.json`](crates/kind-tables/data/tokens.json), under the chain it lives on. An ERC20 token records `address` and `fungible_with_v1`; an SPL token records `mint` (base58) and has no V1 flag.
2. Run `just generate`.
3. Commit the edited file together with the regenerated tables.

The token validation test runs on every pull request and push. It calls an ERC20 contract and checks that it reports the recorded symbol, name and decimals. A mint reports only its decimals on chain, so for an SPL token it checks that the mint account exists with the recorded decimals; symbol and name are review context.

### Add a circuit version

Listing a version makes its resources fungible with every other listed version's, behind the same forwarder, one for one. Nothing passes from one version to another: both kinds are assigned one kind point. Exactly one version is active — under the current forwarder's label it keeps its own kind, and the backend creates its resources — and it must be the highest listed; every other version is deprecated, an alias of the active one, which the backend only consumes. Making a version active moves every ERC20 kind point on every chain to its kind. Review the change as carefully as a permission to create new tokens.

The release joins the dependency graph under a new name, so that the generator can check the logic ref you list against the one the crate compiles to. The current pin stays.

1. Add the release to `[workspace.dependencies]` in [`Cargo.toml`](Cargo.toml) under a new name:
   ```toml
   transfer_library = "2.0.0"                                                 # unchanged
   transfer_library_v3 = { package = "transfer_library", version = "3.0.0" }
   ```
2. Add the same name to [`crates/kind-tables/Cargo.toml`](crates/kind-tables/Cargo.toml), as an optional dependency and in the `generate` feature list.
3. In [`crates/kind-tables/src/bin/generate.rs`](crates/kind-tables/src/bin/generate.rs), add its version to `Versions` and `versions()`, and one line to `pinned_erc20_circuits`:
   ```rust
   (versions.transfer_v3.clone(), digest(transfer_library_v3::TOKEN_TRANSFER_ID.as_bytes())),
   ```
4. In [`crates/kind-tables/data/circuit-versions.json`](crates/kind-tables/data/circuit-versions.json), append the version with `"status": "active"` and set the previous active version to `"deprecated"`.
5. Run `just generate`. Read the diff: one new row per token on every chain, and every older version's rows now alias the new one.
6. Commit the edited files together with the regenerated tables.

Old resources leave through the new version once the forwarder accepts it, which is an upgrade in the forwarder repository. The promotion gate reads each chain's forwarder and requires that the logic ref it accepts is listed.

An SPL token circuit release follows the same steps with the Solana crate: the release of `anomapay-solana-resource`'s `transfer_library` is pinned by commit under a new name next to `anomapay-solana-transfer-library`, its line goes into `pinned_spl_token_circuits`, and the version is appended to `SPLTokenResource` in `circuit-versions.json`. On Solana the forwarder accepts the new logic ref after its upgrade authority calls `set_logic_ref`; the promotion gate reads the forwarder's config account for it.

### Alias the V1 forwarder

On a chain that ran v1, the ERC20 forwarder changes once: the V1 forwarder is immutable, and the current forwarder is a proxy at a new address. Both accept the same logic ref, so a V1 resource differs from a current one only in the forwarder inside its label. On every chain that records both forwarders, the generator writes one member under the V1 forwarder's label for each token the list marks for conversion, an alias of the active version under the current forwarder's label.

1. Record the V1 forwarder, with the logic ref it accepts, in the `v1` array of the forwarder repository's `deployments.json`, and release the forwarder bindings.
2. In [`crates/kind-tables/data/tokens.json`](crates/kind-tables/data/tokens.json), set `fungible_with_v1` to `true` for every token whose V1 resources must be fungible with its current ones, and to `false` for the rest. A token set to `false`, or missing from the list, gets no V1 member, so its V1 resources stay in v1. Nothing checks this.
3. Pin the new bindings and run `just generate`. Read the diff: one V1 member per marked token on every chain that records both forwarders.
4. Commit the edited file together with the regenerated tables.

A V1 member lets a V1 resource unwrap from the current forwarder. Install the table while the protocol adapter is paused, and unpause it only after the V1 balances moved to the current forwarder (ADR-0008).

### Update the deployed commitments

1. Merge into `next`.
2. Read each chain's commitment from `crates/kind-tables/data/generated/<environment>/commitments.json`.
3. Install it with the protocol adapter repo's `contracts-*-kind-table-*` recipes, which call `setKindTableCommitment`. On Solana, the adapter authority calls `set_kind_table_commitment`, through the Solana protocol adapter repo's `dev.sh set-kind-table --cluster <cluster>` with `PA_KIND_TABLE_COMMITMENT` set.
4. Open the promotion pull request into `staging` or `main`. It passes only when every protocol adapter of that environment already stores the commitment this source generates.

## Verifying

```sh
just crates-fmt-check && just crates-build && just crates-lint && just generate-check && just crates-test
```

The tests read `ALCHEMY_API_KEY` from the environment (or `.env`, see `.env-example`). The Solana checks use the public cluster RPC unless `SOLANA_RPC_URL_DEVNET` or `SOLANA_RPC_URL_MAINNET_BETA` names another.
