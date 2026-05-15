# TLA+ Temporal Model Plan — vb-core-replay-divergence-recovery

## Boundary

- Temporal/workflow behavior: None. This bead covers single-writer sequential recovery replay. No concurrent workflow transitions, no distributed consensus, no scheduler, no protocol with temporal liveness requirements beyond sequential event replay.
- Rust/core behavior excluded from TLA+: Postcard codec invariants, frame hydration, action replay tracking, digest verification — all handled by miri on existing tests.
- External systems abstracted: Fjall journal (treated as append-only ordered store), CompiledWorkflow (treated as immutable artifact).
- Non-applicability rationale: Recovery is deterministic single-writer sequential replay. Every property of interest (seq ordering, no double-scheduling, digest match, fail-closed on corruption) is a data invariant over the event stream, not a temporal/liveness property requiring model checking.

## TLA+-Owned Clauses

None.

Rationale:
- Recovery replay is not a concurrent or distributed system. There is one writer (the runtime) and one recovery path (the replay).
- State-over-time behavior is limited to: "events are replayed in order, each step is applied exactly once."
- This is a data integrity property, not a temporal protocol property. It is covered by miri on integration tests and by property-based tests.
- If future work introduces concurrent recovery workers or multi-version snapshot interleaving, a TLA+ model will be required at that time.

## Evidence Command

N/A — no TLA+ model for this bead.

## Waivers

| Clause ID | Reason | Compensating Evidence |
|---|---|---|
| TLA+ model for recovery replay | Single-writer deterministic sequential replay; no temporal/liveness properties requiring model checking | miri on recovery_integration.rs, replay_resume.rs; proptest contract tests in vb_qi37_1_1_red_recovery_contract_test.rs |
