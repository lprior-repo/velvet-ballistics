bead_id: vb-qi37.13
bead_title: cli: Reconcile structured output contract
phase: 3
updated_at: 2026-05-14T22:16:30Z
attempt: 1-of-7

# TLA+ Temporal Model Plan

## Boundary

- Temporal/workflow behavior: none in scope for this reconciliation bead.
- Rust/core behavior excluded from TLA+: CLI enum mapping, diagnostic envelope fields, postcard header validation.
- External systems abstracted: shell process status and filesystem output are treated as observation boundaries.
- Non-applicability rationale: the scoped defects are pure local value mapping and codec validation, not state-over-time lifecycle, concurrency, retry, lease, queue, or distributed behavior.

## TLA+-Owned Clauses

- None.

## Waivers

- TLA-WAIVE-001: No TLA+ model required for `CliExitCode` range or postcard header bounds; Verus/Kani/proptest/fuzz are the appropriate layers. Owner: go-skill/rust-contract. Expiry: bead closure. Compensating evidence: VERUS-EXIT-001, TEST-EXIT-001, FUZZ-POSTCARD-001.
