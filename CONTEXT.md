# Anoma Kind Tables

The kind tables the Anoma protocol adapters are committed to, one per chain per environment, and the supported-token list they are built from.

## Language

**Kind**:
The hash of a resource's logic ref and label ref, as an elliptic curve point. A resource's commitment covers both, so its kind is fixed when it is created. A kind table is indexed by kind, written as the pair.
_Avoid_: type, asset, denomination, key

**Kind point**:
The point the compliance circuit uses as the binding generator for a resource's quantity in the delta: the one the table in force assigns to the resource's kind, or the kind itself when the table has no entry for it. Two resources balance against each other if and only if their kind points are equal.
_Avoid_: point (on its own), generator

**Logic ref**:
The verifying key of the circuit that governs a resource. With the label ref, it fixes the resource's kind.
_Avoid_: circuit ID, image ID, VK (all correct upstream, but this repo says logic ref because that is the field name a kind is written with)

**Label ref**:
The other half of what a kind is hashed from, distinguishing instances that share one logic — for an ERC20 resource, the forwarder and token it belongs to; for an SPL token resource, the forwarder program and mint.

**Kind table**:
The mapping from kinds to kind points that one protocol adapter is committed to. Exactly one per protocol adapter, and therefore one per chain per environment.
_Avoid_: kind registry, kind map, lookup table

**Chain table**:
A kind table named by the chain it belongs to. The unit this repo generates, reviews and publishes.

**Chain key**:
The identifier a chain is keyed by in every chain-keyed file and table name: an EVM chain's chain ID, or a Solana cluster's CAIP-2 chain ID (`solana:` and the first 32 characters of its genesis hash).
_Avoid_: network ID, cluster name (a name such as `devnet` is the review comment, not the key)

**Entry**:
One row of a kind table: a kind, written as its `(logic ref, label ref)`, and the kind point it is assigned.

**Fungibility domain**:
The resources whose kind points are equal, and the kinds the table assigns that kind point to. On this table that is one forwarder holding one token on one chain, together with every kind whose resources those tokens back. Their kind point is the kind of the active circuit version under the current forwarder's label, so that entry is assigned its own kind, and every other member is an alias of it.
_Avoid_: domain, alias set, anchor

**Member**:
An entry inside a fungibility domain: one circuit version under one forwarder's label, assigned the fungibility domain's kind point. Every member except the active version under the current forwarder's label is an alias.

**Alias**:
An entry assigned another kind as its kind point, not its own. The table makes two kinds share one kind point, so their resources are fungible. Every member of a fungibility domain is one, except the active version under the current forwarder's label. Its `alias_of` names the kind it takes its kind point from. Only the table can express an alias, and an alias is a permission to create tokens of its fungibility domain: the only entry a reviewer must read.
_Avoid_: override, remap, redirect, canonical (for the others: an entry assigned its own kind)

**Circuit version**:
One release of the ERC20 or SPL token transfer circuit, listed in `data/circuit-versions.json` with its logic ref and a status. The two lists follow the same rules, each over its own chains. Exactly one version is **active**: under the current forwarder's label it keeps its own kind, and the backend creates its resources. Every other is **deprecated**: an alias of the active one, which the backend consumes and converts. The active version is the highest listed, and an older version never becomes active again. Every listed version is a member of every fungibility domain of its resource type, and nothing is ever removed.
_Avoid_: succession, upgrade, migration (the migration is what listing enables, not the record of it), inherit (nothing passes from one version to another — both kinds are assigned one kind point)

**V1 forwarder**:
The immutable ERC20 forwarder that ran with a chain's v1 protocol adapter. Under its label, the logic ref it accepts is a member of every ERC20 fungibility domain on its chain, so its resources convert and leave through the current forwarder. Recorded in the forwarder repository's deployment record, never here.
_Avoid_: retired forwarder, old forwarder, legacy forwarder

**V1 resource**:
An ERC20 resource whose label names a V1 forwarder.
_Avoid_: 2.0.0 resource (a circuit version names a logic ref, and a logic ref does not tell a V1 resource from a current one)

**Kind table commitment**:
The digest a protocol adapter stores and every compliance proof reproduces, covering every entry and their order. The value this repo exists to publish.
_Avoid_: kind table hash, table root

**Supported token**:
An ERC20 contract or SPL token mint this project maintains kinds for, on a named chain. Being supported is a standing commitment and does not depend on where anything is deployed. Each ERC20 token says whether its V1 resources are fungible with its current ones; Solana has no V1 forwarder.

**V1 fungibility**:
Whether the V1 forwarder's label joins the fungibility domain of one supported token, which makes that token's V1 resources fungible with its current ones, so they convert and leave. `data/tokens.json` states it per chain and per token in `fungible_with_v1`, and only a token set to `true` gets a member under the V1 forwarder's label.
_Avoid_: migration (the forwarder repository moves the tokens, and the protocol adapter copies the state; neither is this flag), conversion (what the fungibility permits, not the permission)

**Environment**:
One of the two protocol adapter deployments a kind table can be installed on, each tracking a branch. Says which deployment, never which chain.
_Avoid_: network, deployment target

**Staging / Production**:
The two environments, matching the protocol adapter's. Staging tracks `staging`, production tracks `main`.

**Promotion gate**:
The assertion, run only on a pull request into an environment's branch, that every protocol adapter in that environment already stores the commitment this source computes.
