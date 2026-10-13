# The hash is owned here and cross-checked

The library computes the commitment — the Merkle root over the leaves `sha256(logic_ref ‖ label_ref ‖ kind_point)` — and hash-to-curve locally, so that a consumer that only reads tables does not depend on arm-risc0. The tie to the circuit is a permanent test that loads one table through arm-risc0's public `init_kind_table_from_file` path and asserts the local root equals `kind_table_root()`, and a test that gives every recorded table to a compliance unit and compares the root the unit commits to.

## Consequences

- The cross-check is not scaffolding: it stays as the only assertion tying published commitments to what the circuit computes.
- arm-risc0 2.0.0-rc.7 exposes a non-global `KindTable` with its root. The tests use it; the library does not, so that its consumers need no arm-risc0.
