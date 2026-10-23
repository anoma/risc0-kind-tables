# Token identity lives here, policy stays in the backend

The backend's chain registry computes the same `sha256(forwarder ‖ token)` labels from its own token list, and two lists deriving one label set is the divergence this repo exists to prevent: a token present in one and not the other makes the backend accept transactions no prover can prove. Token identity — chain, address, symbol, name, decimals — is owned here in `data/tokens.json`; application policy (`fee_enabled`, `coinbase_pair`, `apy`, display names, chain enablement) stays in the backend, keyed by chain and address, and the backend consumes this crate by git rev per pa-evm ADR-0002.

## Consequences

- The backend's `TokenInfo` splits into consumed identity and resident policy — a real refactor of a live service, accepted knowingly.
- Supported tokens are a standing commitment independent of deployments: `tokens.json` may list tokens on chains with no recorded deployment, and the generator emits trees only for chains that have both a protocol adapter and a forwarder recorded.
