# Release checklist

Changes accumulate on `next` and are promoted through `staging` to `main`, matching the protocol adapter's environments: `staging` describes the staging deployments, `main` the production deployments.

## Changing a table on `next`

1. Edit `data/tokens.json` or `data/circuit-versions.json`, or bump a forwarder or protocol adapter record. Never raise a circuit pin without listing the release: the generator stops. See *Add a circuit version* in the [README](README.md#add-a-circuit-version).
2. Run `just generate` and commit the inputs together with the generated diff.
3. Review every new member in the diff as what it is: a decision that resources of two circuit versions balance against each other, one for one.
4. Merge into `next` once the pull request is green.

## Updating the staging environment

1. Read each chain's root from `data/generated/staging/roots.json`.
2. Install it with the protocol adapter repo, per chain: `just contracts-execute-staging-kind-table-update <deployer> <proxy> <root> <chain>`.
3. Update **every** chain of the environment before opening the promotion pull request — one chain left behind blocks the promotion for all of them.
4. Open the promotion pull request `next` → `staging`. It is green once every staging protocol adapter stores the generated root.

## Updating the production environment

1. Promote `staging` → `main` only after the staging cycle completed.
2. Read each chain's root from `data/generated/production/roots.json`.
3. Propose it to the owning Safe with the protocol adapter repo, per chain: `just contracts-propose-production-kind-table-update <deployer> <proxy> <proposer> <root> <chain>`; the Safe owners confirm and execute in the Safe app.
4. The promotion pull request `staging` → `main` is green once every production protocol adapter stores the generated root.

## Publishing the crate

1. Bump the workspace version on `next` and let it ride the same promotion.
2. Publish `anoma-risc0-kind-trees` from the `main` merge commit: `just crates-publish --dry-run`, then `just crates-publish`. Publishing requires every git-pinned dependency to be replaced by a registry release first.
