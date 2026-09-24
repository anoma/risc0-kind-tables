# Solana clusters use the same tables

The Solana protocol adapter runs the same compliance circuit and stores a kind table commitment the same way, and the AnomaPay Solana resource has the same label shape as the ERC20 one, `sha256(forwarder ‖ token)`, with a 32-byte forwarder program id and mint. So a Solana cluster gets a chain table from this repository like any EVM chain, and the fungibility domains and rules of ADR-0008 apply unchanged, instead of a second repository that would have to reproduce the hash and the generator.

A Solana cluster has no EVM chain ID, and a name such as `devnet` is ours rather than the cluster's. It is keyed by its CAIP-2 chain ID, the first 32 characters of its base58 genesis hash, which every tool derives from the cluster itself and which a test checks against the genesis hash the cluster reports.

The EVM deployment records come from the EVM bindings crates. The Solana ones come from the Solana crates, pinned by commit: the adapter and forwarder program ids from `anoma-pa-solana-client`, and the transfer logic ref from `anomapay-solana-resource`'s `transfer_library`.

## Consequences

- A Solana table has no generic call entry, because Solana has no generic call forwarder, and no V1 members, because the V2 Solana deployment starts fresh with no V1 resources to carry over.
- The Solana crates must resolve against the same `anoma-rm-risc0` release as the EVM crates. A release that moves one side without the other stops the generator until the other side follows.
- The gates are the same as on EVM: the cluster key and every mint's decimals are checked on every pull request, and the adapter's stored commitment and the forwarder's accepted logic ref on promotion.
