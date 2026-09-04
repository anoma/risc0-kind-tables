# Anoma Kind Tables

The kind tables the Anoma protocol adapters are committed to, one per chain per environment, and the supported-token list they are built from.

## Language

**Kind**:
The fungibility domain of a resource — an elliptic curve point that the compliance circuit uses as the binding generator for that resource's quantity. Two resources balance against each other if and only if they share a kind.
_Avoid_: type, asset, denomination

**Logic ref**:
The verifying key of the circuit that governs a resource. One half of the key that names a kind.
_Avoid_: circuit ID, image ID, VK (all correct upstream, but this repo says logic ref because that is the field name a kind is keyed by)

**Label ref**:
The other half of the key that names a kind, distinguishing instances that share one logic — for an ERC20 resource, the forwarder and token it belongs to.

**Kind table**:
The mapping from `(logic ref, label ref)` keys to kinds that one protocol adapter is committed to. Exactly one per protocol adapter, and therefore one per chain per environment.
_Avoid_: kind registry, kind map, lookup table

**Chain table**:
A kind table named by the chain it belongs to. The unit this repo generates, reviews and publishes.

**Entry**:
One row of a kind table: a key and the kind it names.

**Canonical entry**:
An entry whose kind is the one the key hashes to. It changes nothing about what a transaction may do — the circuit derives the same kind without it — and exists only to spare the circuit that derivation.
_Avoid_: normal entry, plain entry

**Alias**:
An entry whose kind is *not* the one its key hashes to, so that two keys name one kind and their resources become fungible. Generated from a succession, never authored row by row, and the only entry a reviewer must read.
_Avoid_: override, remap, redirect

**Succession**:
The authored decision that one circuit version's kinds carry to its successor. Stated once, globally, and fanned out into one alias per label that circuit owns on every chain — so a token or a chain cannot be left behind by omission.
_Avoid_: upgrade, bump, migration (the migration is what a succession enables, not the record of it)

**Anchor**:
The circuit version whose canonical kind every successor's alias resolves to. A resource's kind is fixed when it is created, so re-pointing a key strands the resources created under it: a succession adds rows pointing back at the anchor and never moves it.

**Vulnerable version**:
A circuit version recorded as compromised. No succession may name it on either side, so fungibility is never extended to or from its resources, and its keys are the one exemption from the anchor rule — they may be deliberately re-pointed, which freezes those resources, honest holders included.
_Avoid_: deprecated (a deprecated version is one a succession has moved past, and stays fungible)

**Kind table commitment**:
The digest a protocol adapter stores and every compliance proof reproduces, covering every entry and their order. The value this repo exists to publish.
_Avoid_: kind table hash, table root

**Supported token**:
An ERC20 contract this project maintains kinds for, on a named chain. Being supported is a standing commitment and does not depend on where anything is deployed.

**Environment**:
One of the two protocol adapter deployments a kind table can be installed on, each tracking a branch. Says which deployment, never which chain.
_Avoid_: network, deployment target

**Staging / Production**:
The two environments, matching the protocol adapter's. Staging tracks `staging`, production tracks `main`.

**Promotion gate**:
The assertion, run only on a pull request into an environment's branch, that every protocol adapter in that environment already stores the commitment this source computes.
