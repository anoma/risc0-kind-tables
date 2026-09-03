# Tables are generated and committed

A chain table is derived from authored inputs (`data/tokens.json`, `data/successions.json`, `data/vulnerabilities.json`, the forwarder deployment records, the circuit IDs) but the derived artifact is committed under `data/generated/` and CI regenerates it and fails on `git diff --exit-code`, the same pattern as `bindings-check` in the contract repos. An alias is a mint authorization — two keys sharing a kind are one asset at par — so the entry it produces and the commitment change it causes must be legible in a pull request diff, next to the reviewed decision; a mistyped kind cannot survive because regeneration rejects anything the generator would not emit.

## Consequences

- The review surface of a generated diff is exactly the alias set; every canonical entry is machine-checked against its key.
- The generated schema is arm-risc0's kind table JSON extended with the kind point, so the upstream loader can read it once it stops deriving points unconditionally.
