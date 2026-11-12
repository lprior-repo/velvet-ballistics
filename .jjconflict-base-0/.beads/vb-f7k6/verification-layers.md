# Verification Layers: Timer Wheel

## Boundary
- TLA+: `verification/tla/TimerWheel.tla` and `.cfg`.
- Verus: checked arithmetic, bi-index transition, due partition, stale-fire validation if proof-friendly helpers exist/are introduced.
- Kani/proptest: generated boundary/state-sequence exploration; no hardcoded shapes.
- Loom: cancel/replace racing with `TimerFired` delivery.
- Runtime tests: future behavior evidence.
- Review: independent `contract-verification-review.md` before consumption.

## Layer Assignment
- PRE-001/PRE-007/POST-008/INV-010: TLA+ lifecycle gates + Verus transition guards + tests.
- PRE-002/PRE-003/POST-009/INV-001/INV-002: TLA+ checked overflow + Verus/Kani/proptest boundary proof.
- POST-001/POST-002/INV-003/INV-004/INV-006: TLA+ insert/replace + Verus index consistency + tests.
- PRE-004/POST-003/INV-005: TLA+ cancel + Verus removal proof + tests.
- PRE-005/POST-004/POST-005/POST-006/INV-007/INV-008: TLA+ fire-expired + Verus/proptest due partition + tests.
- PRE-006/POST-007/INV-009: TLA+ stale fire + Loom + tests.
- INV-011: TLC deadlock checking.

## TLA+ Scope
Variables: `runState`, `runIndex`, `deadlineIndex`, `generation`, `lastOutcome`, `firedEvents`.
Actions: `Init`, `InsertTimer`, `ReplaceTimer`, `CancelTimer`, `FireExpired`, `DeliverTimerFired`, `ShutdownRun`, `CompleteRun`, `FailRun`, `Idle`.
Invariants: `TypeOK`, `NoDeadlineWrap`, `OneActiveTimerPerRun`, `BiIndexConsistent`, `CancelRemovesAllIndexes`, `ReplaceRemovesOldGeneration`, `DueOnlyFires`, `FireRemovesReturned`, `StaleFireNoMutation`, `TerminalNoTimerMutation`.
Temporal: `OverflowEventuallySuspended`, `DueTimerEventuallyFireable`, `NoResurrectionAlways`.
Command: `tlc -config verification/tla/TimerWheel.cfg verification/tla/TimerWheel.tla`.

## Verus Scope
Targets are discovery-blocked until State 4 confirms actual proof files/functions. Candidate proof surfaces: `spec_checked_deadline`, `proof_checked_deadline_no_wrap`, `spec_timer_transition`, `proof_timer_transition_preserves_bi_index_consistency`, `spec_validate_timer_fire`. No vacuum proofs; must bind to actual Rust implementation or required proof-friendly helper.

## Kani/Proptest Scope
Must generate structural timer wheel inputs/operation sequences. Include `0`, `MAX_TIME`, overflow, absent timer, replacement, cancellation, terminal states, stale fired metadata. Exact commands blocked until State 5 identifies harness names.

## Loom Scope
Known related model path: `crates/vb_runtime/src/models/loom/timer_fired_cancel.rs`. Future evidence must prove cancel/replace versus `TimerFired` cannot resurrect or mutate via stale fire. Exact command blocked pending repository discovery.

## Future Runtime Scenarios
- `given_timer_duration_overflows_when_inserted_then_deadline_overflow_and_no_indexes_change`
- `given_run_has_timer_when_replaced_then_old_deadline_bucket_is_removed`
- `given_run_has_timer_when_cancelled_then_run_and_deadline_indexes_are_empty_for_run`
- `given_mixed_deadlines_when_fire_expired_then_only_due_timers_return_and_are_removed`
- `given_timer_was_cancelled_when_stale_timer_fired_arrives_then_invalid_timer_fire_and_no_resurrection`
- `given_timer_was_replaced_when_old_timer_fired_arrives_then_invalid_timer_fire_and_current_timer_remains`
- `given_run_is_terminal_when_timer_mutation_requested_then_lifecycle_error_and_no_timer_created`

## Waivers
No TLA+ waiver. Lean waived unless State 4 finds Verus insufficient. Exact Verus/Kani/Loom/test commands are discovery-blocked, not waived.
