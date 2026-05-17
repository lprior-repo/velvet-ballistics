# Theorem Kernel Projection - vb-qi37.12.2

STATUS: WAIVED

## Boundary

- TLA+-owned temporal model: resume workflow and append-failure transitions.
- Verus-owned Rust core: none currently required unless downstream extracts a pure conversion/state kernel.
- Theorem-owned kernel: none.
- Rust/runtime shell: error enum layout, journal I/O, state restoration, and semver-compatible error conversion.
- External systems excluded from theorem proof: durable journal, storage failures, async/runtime scheduling.

## Theorem-Owned Clauses

- None. The blocking issue is representational: a unit public enum variant cannot carry per-error source identity. This does not need Lean/Aeneas/Hax.

## Waiver

- Owner: State 3 rust-contract.
- Reason: no tiny algebraic theorem kernel adds value beyond the explicit contract narrowing and TLA+/test obligations.
- Expiry: revisit only if owner chooses a semver-breaking source-carrying error shape or a new explicit source-detail API.
- Compensating evidence: API compatibility, focused integration tests, static scan forbidding ambient source side channels, and optional TLA+ workflow model.
