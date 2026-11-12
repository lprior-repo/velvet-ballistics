bead_id: vb-qi37.15.3
bead_title: cli: Add trace command
phase: State 3
updated_at: 2026-05-12T00:00:00Z

# TLA+ Temporal Model Plan

## Boundary
- Temporal behavior: None. The `trace` command is a read-only replay of already-persisted journal events. No state machine transitions, no concurrency, no retry/lease logic, no liveness conditions beyond "events are returned if they exist".
- Rust/core behavior excluded from TLA+: pure journal read and trace formatting in `commands_journal::build_trace` and `trace_one`.
- External systems abstracted: Fjall journal storage is treated as an immutable event sequence source.
- Non-applicability rationale: TLA+ is designed for temporal/state-over-time behavior. Trace is a deterministic pure function from an event sequence to a formatted output. There is no state machine, no concurrency, no fairness conditions, no deadlock potential, and no liveness property beyond "returns events if they exist". The purity of `build_trace` and the read-only nature of the command mean no temporal model is warranted.

## TLA+-Owned Clauses
- None.

## Model Shape
- N/A (no temporal model applies).

## Properties
- N/A.

## Evidence Command
- N/A.

## Waivers
- TLA+ waiver for all trace clauses: Owner=vb-qi37.15.3 agent. Reason=trace is a pure read-only journal replay with no temporal behavior. Expiry=none (permanent waiver until scope changes). Compensating evidence=INV-001 pure function tests in `commands_journal.rs` unit tests and proptest coverage.