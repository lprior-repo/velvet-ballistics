# Domain Model Review: Timer Wheel

## Decision
Model the timer wheel as a finite state machine over bounded hardware-sized time, not as unbounded math. The aggregate is `(run_index, deadline_index, runState, generation, firedEvents)`. Valid state means both indexes are exact projections of the same active timer set.

## Entities and Values
- Entity: `Run` by `RunId`.
- Values: `Time`, `Duration`, `Deadline`, `TimerKind`, `TimerEntry=(run, deadline, kind, generation)`.
- Lifecycle: `Active`, `SuspendedOverflow`, `Cancelled`, `Shutdown`, `Completed`, `Failed`.
- Outcomes: `Scheduled`, `Replaced`, `CancelledNoTimer`, `Fired`, `OverflowSuspended`, `InvalidFireRejected`, `LifecycleRejected`.

## Aggregate Invariants
1. A run has zero or one active timer.
2. `run_index` and `deadline_index` are two indexes over one abstract active-timer set.
3. Replacement is one semantic transition: remove old, then expose new.
4. Cancel is complete only after both indexes have no run entry.
5. `fire_expired(now)` is destructive: returned due timers are removed.
6. Stale `TimerFired` is past information, not present authority.
7. Terminal lifecycle state dominates timer events.

## Bounded Arithmetic Model
Valid checked-add shape:

```text
if now in 0..MAX_TIME and duration in 0..MAX_DURATION and duration <= MAX_TIME - now
then Ok(now + duration)
else Err(DeadlineOverflow)
```

Invalid shape: `(now + duration) % (MAX_TIME + 1)` for normal scheduling.

## Freshness Review
The abstract model requires generation/freshness metadata. If runtime events only carry `RunId`, State 4/5 must require a refinement mechanism before accepting stale-fire correctness.

## Rejected Designs
- Unbounded `Nat` deadlines: hides overflow.
- Deadline-index-only truth: cannot prove cancel/replace by run.
- Run-index-only truth: cannot prove due ordering.
- Idempotent stale-fire success: masks resurrection/lost-cancel bugs.
