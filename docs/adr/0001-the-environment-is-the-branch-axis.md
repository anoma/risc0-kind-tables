# The environment is the branch axis

A kind tree depends on which circuit versions an environment runs, and a circuit version is a dependency pin in `Cargo.toml`, not a datum — one checkout cannot hold two revisions of one crate. Environments are therefore carried the way the protocol adapter carries them (pa-evm ADR-0003): `next` accumulates changes, `staging` pins what the staging protocol adapters are committed to, `main` pins production, and changes flow one way through promotion pull requests.

## Consequences

- One checkout answers for one environment; what production is committed to is read from `main` or from the chain, never from `next`.
- A listed logic ref outlives its crate, so it is checked-in data in `data/circuit-versions.json`, not a value imported from a dependency.
