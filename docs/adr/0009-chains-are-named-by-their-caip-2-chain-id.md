# Chains are named by their CAIP-2 chain ID

A kind tree belongs to one chain, and every chain-keyed file names that chain. An EIP-155 chain ID names EVM chains only, and `alloy_chains::NamedChain` knows EVM chains only, so neither can name a Solana chain. CAIP-2 names a chain in any chain family: a namespace and a reference joined by a colon, such as `eip155:11155111` for Sepolia and `solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp` for Solana. `tokens.json` and `roots.json` are keyed by it, and the crate looks chains up by it.

A generated tree is named for the same ID with the colon replaced by `_`, such as `eip155_11155111.json`. `cargo package` refuses a colon in a file name, and the crate embeds its trees, so a colon would make the crate unpublishable. No namespace contains `_`, so the first `_` separates the namespace from the reference, and the file name maps back to the ID without loss.

## Consequences

- No root changes. The root covers the entries only, and a chain ID is never part of an entry.
- The lookups take `impl Into<Caip2ChainId>`, and a `NamedChain` converts, so an EVM consumer compiles without a change.
- An `eip155` reference must be the decimal chain ID without a leading zero, so that one chain has one key.
- The loaders reject a chain this crate cannot name, so a mistyped key fails loudly. Today only EVM chains have names. A Solana tree also needs a Solana token address, a Solana label derivation and Solana deployment records; the chain ID is the first of these, not all of them.
