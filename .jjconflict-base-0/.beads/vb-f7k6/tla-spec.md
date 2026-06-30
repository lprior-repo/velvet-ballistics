# TLA+ Temporal Model Plan: Timer Wheel

## Boundary
- Temporal behavior: bounded deadline scheduling, checked overflow, insert/replace/cancel/fire transitions, stale `TimerFired` rejection, lifecycle immutability.
- Excluded to Rust proof/tests: concrete `BTreeMap`/`HashMap`, allocation, ownership, exact queue plumbing, wall-clock source.
- Module/config: `verification/tla/TimerWheel.tla`, `verification/tla/TimerWheel.cfg`.

## Model Shape
Constants: `RUNS`, `KINDS`, `MAX_TIME`, `MAX_DURATION`, `TIMES=0..MAX_TIME`, `DURATIONS=0..MAX_DURATION`, bounded `GENERATIONS`.

Variables: `runState`, `runIndex`, `deadlineIndex`, `generation`, `lastOutcome`, `firedEvents`.

Actions: `Init`, `InsertTimer`, `ReplaceTimer`, `CancelTimer`, `FireExpired`, `DeliverTimerFired`, `ShutdownRun`, `CompleteRun`, `FailRun`, optional `Idle`.

## Required Arithmetic Semantics
`CheckedAdd(now,duration)` must branch as:

```text
IF now \in TIMES /\ duration \in DURATIONS /\ duration <= MAX_TIME - now
THEN Ok(now + duration)
ELSE Err(DeadlineOverflow)
```

Forbidden: scheduling by modular wrap. Overflow must set `lastOutcome=DeadlineOverflow`, move run to `SuspendedOverflow` or exact error state, leave both indexes unchanged for that run, and create no fired event.

## State Constraints and Bounds
- `runState in [RUNS -> RunStates]`; `DOMAIN runIndex subset RUNS`; deadline-index keys are bounded `TIMES`.
- Timer entries contain only declared runs/kinds/generations/deadlines.
- Include TLC boundary values `0`, `MAX_TIME`, and at least one `now,duration` pair with `duration > MAX_TIME - now`.
- `RUNS` and `KINDS` may be symmetric if no identity/order-specific logic exists.

## Safety Invariants
- `TypeOK`
- `NoDeadlineWrap`
- `OneActiveTimerPerRun`
- `BiIndexConsistent`
- `CancelRemovesAllIndexes`
- `ReplaceRemovesOldGeneration`
- `DueOnlyFires`
- `FireRemovesReturned`
- `StaleFireNoMutation`
- `TerminalNoTimerMutation`

## Temporal Properties
- `OverflowEventuallySuspended`
- `DueTimerEventuallyFireable` unless cancel/shutdown/terminal preempts it
- `NoResurrectionAlways`

## Fairness and Deadlock
- Weak fairness on `FireExpired` for due timers when enabled and not preempted.
- Weak fairness on `DeliverTimerFired` for pending fired events.
- No fairness assumed for caller-driven insert/cancel/replace unless a liveness property explicitly models the environment.
- TLC deadlock checking required; if all-terminal states need stutter, model explicit `Idle` and document it.

## Refinement Boundary
- `TimerWheel::insert` refines `InsertTimer`/`ReplaceTimer`.
- `TimerWheel::cancel` refines `CancelTimer`.
- `TimerWheel::fire_expired` refines destructive due partition.
- `Runtime::timer_fired` / `ShardCommand::TimerFired` refine `DeliverTimerFired` with freshness/lifecycle validation.
- `await_timer` and lifecycle handlers refine scheduling/cancel/shutdown/terminal transitions.

## Evidence Command
`tlc -config verification/tla/TimerWheel.cfg verification/tla/TimerWheel.tla`

## Waivers
None for TLA+. This bead is explicitly TLA+ model work.
