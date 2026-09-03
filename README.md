# Anoma Kind Tables

[![Crates](https://github.com/anoma/kind-tables/actions/workflows/crates.yml/badge.svg)](https://github.com/anoma/kind-tables/actions/workflows/crates.yml)

The kind tables the Anoma protocol adapters are committed to — one per chain per environment — and the supported-token list they are built from. The `anoma-kind-tables` crate computes the commitments the way the compliance circuit does, so one source answers what every protocol adapter must store.

## How it fits together

A kind table maps `(logic_ref, label_ref)` keys to kind points. Its commitment — SHA-256 over the ordered entries — is what a protocol adapter stores via `setKindTableCommitment` and what every compliance proof reproduces. A chain's table is derived: the padding entry from `anoma-rm-risc0`, the generic call entry from the recorded forwarder, and one ERC20 entry per supported token from the recorded ERC20 forwarder. Aliases — entries whose point belongs to another key, the migration path between resource logic versions — are the only authored rows; everything else is machine-checked against its key.

## Layout

```
crates/kind-tables/            the library and the generator
├── data/
│   ├── tokens.json            authored: the supported tokens, per chain
│   ├── aliases.json           authored: the aliasing decisions, per chain
│   └── generated/
│       ├── staging/           <chain id>.json tables + commitments.json
│       └── production/
└── src/
crates/integration-test/       on-chain token validation, promotion freshness gate, arm-risc0 cross-check
docs/adr/                      the decisions behind this layout
```

Every chain-keyed file — `tokens.json`, `aliases.json`, `commitments.json` — is keyed by chain ID, as the forwarder and protocol adapter deployment records they are generated from are, and each section carries the chain name in a `_comment` the loaders ignore. A generated table is named for the chain ID it belongs to and is otherwise `anoma-rm-risc0`'s kind table schema, so `init_kind_table_from_file` reads one unchanged.

## Entries

An entry carries its key, its kind point, and a `_metadata` object naming what the kind belongs to. The commitment covers `logic_ref`, `label_ref` and `kind_point` only, so `_metadata` is free to carry whatever a reviewer needs and never moves the commitment. `version` is the version of the circuit crate that owns `logic_ref`, read from the resolved dependency graph, so bumping a pin cannot leave a stale version behind. `type` is `Padding`, `ERC20`, or `GenericCall`.

```json
{
  "_metadata": {
    "type": "ERC20",
    "version": "2.0.0",
    "name": "WETH",
    "token": "0x4200000000000000000000000000000000000006",
    "forwarder": "0xE54182d915dE447deFc4A17Ec1D4E0dc627551F7"
  },
  "logic_ref": "bc12323668c37c3d381ca798f11116f35fb1639d12239b29da7810df3985e7ad",
  "label_ref": "55008fad9bfccef776960bfca715e365cfba843cfb947190fafc69e4d9fac674",
  "kind_point": "04545c399026b2d1ec31c63488d9b4cc58807611997d66b3dea4a144332c5a0e6a2bd246b7d721cd335199c17bfc17706aa8fee9decb6042d516d6a6ae88bb4d72"
}
```

An aliased entry names a newer circuit version in `logic_ref` but takes the `kind_point` of the entry named in `alias_of`, so resources of both versions share one kind and stay fungible. `label_ref` is unchanged when the same forwarder holds the same token, so only `logic_ref` distinguishes the two rows.

An alias is authored as a token and two circuit versions — never as a ref:

```json
{
  "84532": {
    "_comment": "base-sepolia",
    "aliases": [
      { "token": "0x4200000000000000000000000000000000000006", "alias": "3.0.0", "of": "2.0.0" }
    ]
  }
}
```

The generator derives both circuit IDs from the pinned crates, derives the label from the chain's recorded ERC20 forwarder, and fills in `_metadata` and `alias_of`. A version it does not pin fails the run, so an alias can never name a kind that cannot be derived here, and it generates:

```json
{
  "_metadata": {
    "type": "ERC20",
    "version": "3.0.0",
    "name": "WETH",
    "token": "0x4200000000000000000000000000000000000006",
    "forwarder": "0xE54182d915dE447deFc4A17Ec1D4E0dc627551F7",
    "alias_of": {
      "version": "2.0.0",
      "logic_ref": "bc12323668c37c3d381ca798f11116f35fb1639d12239b29da7810df3985e7ad",
      "label_ref": "55008fad9bfccef776960bfca715e365cfba843cfb947190fafc69e4d9fac674"
    }
  },
  "logic_ref": "<the transfer circuit ID of version 3.0.0>",
  "label_ref": "55008fad9bfccef776960bfca715e365cfba843cfb947190fafc69e4d9fac674",
  "kind_point": "04545c399026b2d1ec31c63488d9b4cc58807611997d66b3dea4a144332c5a0e6a2bd246b7d721cd335199c17bfc17706aa8fee9decb6042d516d6a6ae88bb4d72"
}
```

## Workflows

Add a token: edit `data/tokens.json`, run `just generate`, commit both. The token validation test checks the contract reports the recorded identity on every pull request and push.

Add an alias: pin the circuit release as a renamed dependency and add it to `transfer_circuits` in the generator, edit `data/aliases.json`, run `just generate`, review the alias in the generated diff — an alias makes two kinds fungible, so it carries the weight of a mint authorization. It also moves that chain's commitment, so it needs a `setKindTableCommitment` update before transactions built against the new table verify.

Update the deployed commitments: after a merge into `next`, install each chain's commitment from `data/generated/<environment>/commitments.json` with the protocol adapter repo's `contracts-*-kind-table-*` recipes. The promotion pull request into `staging` or `main` then proves every protocol adapter of that environment stores what this source generates.

## Verifying

```sh
just crates-fmt-check && just crates-build && just crates-lint && just generate-check && just crates-test
```

The tests read `ALCHEMY_API_KEY` from the environment (or `.env`, see `.env-example`).
