# Freshness gates promotions, token validation runs always

Asserting that deployed protocol adapters store the source root on every pull request would deadlock — a table change is red until the chains are updated, and the chains are not updated until it is merged — so freshness follows pa-evm ADR-0003: pull requests into `next` assert only what holds without a chain (regeneration, sorting, the fungibility domain points, the arm-risc0 cross-check), a pull request into `staging` additionally requires every staging protocol adapter's `getKindTableRoot()` to equal the generated root, and `main` the same against production. Token validation — the contract exists and reports the recorded symbol, name and decimals — has no such deadlock and runs on every pull request and push, so a bad address never merges anywhere. The forwarder check — every recorded ERC20 and generic call forwarder accepts calls from the protocol adapter proxy recorded for its chain — has none either, because nothing in this repository has to be installed for it to hold, so it runs on every pull request and push too.

## Consequences

- Every CI run needs `ALCHEMY_API_KEY` and live RPC on all recorded chains, a deliberate departure from pa-evm, where feature CI runs without the key; an RPC outage can red an unrelated pull request.
- The update itself happens after merge into `next`, and the promotion pull request turning green is the proof it happened.
