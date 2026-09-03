# Successions anchor to the original kind

A resource's kind is bound into its commitment when it is created, so a key resolving to one point today and another tomorrow strands every resource created under it — the delta it was created with no longer cancels the delta it is consumed with. Kinds therefore only accumulate. A circuit release is recorded as a succession in `data/successions.json`, whose generated aliases resolve the successor's keys to the *predecessor's* canonical points, never the reverse, and the version generating the canonical rows — the anchor — stays pinned where it is for as long as resources under it can still be consumed. Bumping the primary pin to a new release instead of recording a succession is the failure this rule exists to prevent: it is the obvious move, it produces a table that looks correct, and it re-points every key at once. Canonical entries are exempt in both directions, because the circuit derives the same point when one is missing — adding or dropping one changes nothing a transaction may do, so only aliases carry meaning and only aliases must survive.

A compromised circuit version, recorded in `data/vulnerabilities.json`, is the exception. Fungibility must not reach a version whose resources may be forged, so no succession may name one on either side, and its keys are the one place a deliberate re-point is allowed — which freezes those resources, honest holders included. That is the trade the marking exists to make explicit and reviewable.

## Consequences

- The append-only test compares each table against its published form and fails when an existing key's point changes; the keys of a marked version are the only exemption.
- A circuit release adds a renamed pin and a succession, and never replaces the pinned anchor.
- Deprecation needs no authoring: a version a succession has moved past is deprecated and stays fungible, and the generated `_metadata` says so. An unmarked, unsuperseded version is active.
