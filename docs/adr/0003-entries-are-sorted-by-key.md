# Entries are sorted by key

Entries are sorted by `logic_ref ‖ label_ref`, the order in which arm-risc0 sorts the leaves of the kind tree. The file order does not change the root, but the sort makes the tree a function of the entry set: two branches that add tokens merge to the same kind tree, and "same entries, different files" is unrepresentable. Position in the tree does not matter to the circuit, which reads one Merkle path per resource, so nothing argues for any other order; human grouping lives in `_metadata` objects, which the generator carries through and the hash ignores.
