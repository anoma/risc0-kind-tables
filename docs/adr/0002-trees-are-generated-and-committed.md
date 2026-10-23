# Trees are generated and committed

A chain tree is derived from authored inputs (`data/tokens.json`, `data/circuit-versions.json`, the forwarder deployment records, the logic refs) but the derived artifact is committed under `data/generated/` and CI regenerates it and fails on `git diff --exit-code`, the same pattern as `bindings-check` in the contract repos. A fungibility domain member is a mint authorization — two kinds sharing a kind point are one asset at par — so the entry it produces and the root change it causes must be legible in a pull request diff, next to the reviewed decision; a mistyped kind cannot survive because regeneration rejects anything the generator would not emit.

## Consequences

- Every entry is machine-checked: an alias against the kind point of its fungibility domain, every other entry against its own kind.
- The generated schema is arm-risc0's kind tree JSON extended with the kind point, so the upstream loader can read it once it stops deriving points unconditionally.
