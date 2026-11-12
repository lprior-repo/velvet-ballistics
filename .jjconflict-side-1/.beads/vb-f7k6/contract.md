# Contract Specification: Timer Wheel TLA+ Model

## Context
- Bead: `vb-f7k6` / Add TLA+ Timer Wheel Model.
- Formal-spec-first output only; no production code, tests, proof code, or config edits.
- Future TLA+ targets: `verification/tla/TimerWheel.tla`, `verification/tla/TimerWheel.cfg`.
- Runtime surfaces to refine: `TimerWheel::insert`, `cancel`, `fire_expired`, `next_deadline`, `len`, `get_kind`, `Runtime::timer_fired`, `ShardCommand::TimerFired`.
- GOD rule: TLA+ must model exact bounded hardware limits and explicit Err/suspend transitions; no unbounded `Nat` cheating.

## Domain Terms
- `RunId`: finite run identity.
- `Time`: bounded integer domain `0..MAX_TIME` representing the hardware/runtime deadline encoding boundary.
- `Duration`: bounded integer domain `0..MAX_DURATION`.
- `Deadline`: created only by checked addition of `now + duration`.
- `TimerKind`: finite timer kind set, at least wait/ask if implementation exposes both.
- `run_index`: `RunId -> (deadline, kind, generation)` partial map.
- `deadline_index`: `Deadline -> set(TimerEntry)`.
- `generation`: freshness token per run for stale-fire rejection.
- `RunState`: `Active | SuspendedOverflow | Cancelled | Shutdown | Completed | Failed`.

## Assumptions / Open Questions
- If runtime uses `Instant`, TLA+ models the encoded bounded deadline abstraction, not wall-clock precision.
- TLC may use small finite sets for runs/kinds/generations, but arithmetic must preserve the same overflow branch structure as target hardware.
- State 4 must confirm the exact numeric runtime boundary (`u64`, `usize`, or encoded `Instant` horizon).
- State 4/5 must confirm whether `TimerFired` carries freshness metadata; if not, plan refinement before implementation.

## Preconditions
- PRE-001: Insert/replace requires a known timer-mutable run.
- PRE-002: `now in 0..MAX_TIME` and `duration in 0..MAX_DURATION`.
- PRE-003: Deadline construction uses checked addition; overflow returns `TimerError::DeadlineOverflow`, enters suspended/error state, and mutates no timer index.
- PRE-004: Cancel accepts any known run; absent timer cancel is idempotent success.
- PRE-005: `fire_expired(now)` accepts bounded `now` and observes coherent indexes.
- PRE-006: `TimerFired` may mutate only if metadata matches the current active timer.
- PRE-007: Shutdown/cancel/terminal state gates all later timer mutation.

## Postconditions
- POST-001: Successful insert creates exactly one active timer in both indexes.
- POST-002: Replacement removes every old index entry before the new timer becomes observable.
- POST-003: Cancel removes all entries for the run from both indexes.
- POST-004: `fire_expired(now)` returns only timers with `deadline <= now`.
- POST-005: Returned timers are removed from both indexes.
- POST-006: Timers with `deadline > now` remain indexed and unreturned.
- POST-007: Stale `TimerFired` returns `InvalidTimerFire` and leaves run/timer state unchanged.
- POST-008: Post-shutdown/cancel/terminal timer operations return lifecycle error and cannot resurrect or mutate the run.
- POST-009: Overflow is explicit error/suspend, never wrap.

## Invariants
- INV-001: All time/duration/deadline/generation values remain in finite bounded domains.
- INV-002: No transition computes a wrapped deadline after overflow.
- INV-003: At most one active timer per `RunId`.
- INV-004: `run_index` and `deadline_index` are exact mirrors of one active-timer set.
- INV-005: Cancel completeness: no deadline bucket contains cancelled run entries.
- INV-006: Replacement freshness: old generation/deadline cannot fire.
- INV-007: Fire due-only: no emitted timer has `deadline > now`.
- INV-008: Fire removal: emitted timers are absent from both indexes next state.
- INV-009: Stale fire rejection: stale/absent/wrong metadata cannot mutate or recreate a timer.
- INV-010: Terminal immutability: terminal/cancelled/shutdown runs cannot gain timers or mutate by timer fire.
- INV-011: Deadlock freedom under declared fairness/idle semantics.

## Error Taxonomy
- `TimerError::DeadlineOverflow`: checked deadline addition exceeds `MAX_TIME`; no index mutation; run suspended/errored.
- `TimerError::InvalidTimerFire`: stale, absent, wrong generation/deadline/kind, or terminal target fire.
- `TimerError::RunNotTimerMutable`: mutation after cancellation, shutdown, completion, failure, or suspended terminal-equivalent state.
- `TimerError::InvalidRunId`: run outside known run set/store.
- `TimerError::IndexInvariantViolation`: unreachable in TLA+; runtime fail-closed corruption if observed.

## Abstract Signatures
- `fn insert_timer(run: RunId, now: Time, duration: Duration, kind: TimerKind) -> Result<TimerSnapshot, TimerError>`
- `fn cancel_timer(run: RunId) -> Result<TimerSnapshot, TimerError>`
- `fn fire_expired(now: Time) -> Result<(TimerSnapshot, Vec<TimerEntry>), TimerError>`
- `fn handle_timer_fired(run: RunId, fire: TimerFireMetadata) -> Result<RunTransition, TimerError>`
- `fn shutdown_run(run: RunId) -> Result<TimerSnapshot, TimerError>`

## Verification Ownership
- TLA+: all temporal/lifecycle/scheduler clauses PRE-001..007, POST-001..009, INV-001..011, and all error transitions.
- Verus candidates: checked arithmetic, pure bi-index transitions, due partition, stale-fire validation; must bind to actual Rust implementation or record a blocker.
- Lean: no mandatory theorem; optional tiny algebraic bi-index projection only if Verus is insufficient.

## Non-goals
- No wall-clock precision, latency, or performance claim.
- No implementation/test/proof/model code in State 3.
