# Domain Model: vb-fzgdn — deterministic delayed-action timer seam

## Scope
Model a fresh `vb_runtime` delayed-action/timer seam that replaces behavior-affecting `std::time::Instant` registration and authority with deterministic numeric time. No production Rust, tests, or verifier artifacts are authored in this state.

## Ubiquitous language
| Term | Meaning | Forbidden interpretation |
|---|---|---|
| `TimerTick` | Replayable logical time value owned by runtime clock domain. | Host `Instant`, system time, sleep duration. |
| `TimerDuration` | Bounded logical delay in ticks; zero has explicit policy. | Signed/floating/unchecked duration. |
| `TimerDeadline` | Absolute logical tick when a timer becomes fireable. | Captured `Instant::now()`. |
| `TimerAuthority` | Exact capability to fire one pending timer: run, step, generation, deadline, kind. | Run-only or partial token. |
| `TimerGeneration` | Per-run non-wrapping generation that invalidates stale authorities. | Wrapping/global counter. |
| `DelayedActionKey` | Stable fixed-width deterministic idempotency key. | Random UUID/string key in hot runtime. |
| `PendingDelayedAction` | Shard-owned admitted delayed-action entry. | Partially validated request DTO. |
| `ClockAdvance` | Explicit command to move logical time forward. | Background wall-clock task. |

## Entities and value objects
- Aggregate: `ShardTimerRegistry` owns pending timers, delayed-action idempotency index, and current logical tick for one shard.
- Entity: `RunTimerState` tracks timer lifecycle for a run/step.
- Value objects: `TimerTick(u64)`, `TimerDuration(u64)`, `TimerDeadline(TimerTick)`, `TimerGeneration(u64)`, `TimerCapacity`, `DelayedActionKey`, `TimerKind`, `TimerAuthority`.

## Invariants
1. Replay-visible timer state, journal payloads, and fire authorities use numeric ticks/deadlines, never behavior-affecting `Instant`.
2. Admission validates run existence, step/kind, slot-derived time value, duplicate key, capacity, deadline arithmetic, and generation successor before mutation.
3. Fire validates full authority equality before pending removal, journal success, delayed-action enqueue, or run-frame advancement.
4. Stale, duplicate, wrong-generation, wrong-kind, wrong-step, wrong-run, wrong-deadline, and missing authorities leave registry and run frame unchanged.
5. Generation never wraps; exhaustion is typed failure.
6. Duplicate delayed-action key with identical payload returns the existing authority and preserves original deadline; divergent duplicate fails before mutation.
7. Equal-deadline ordering is deterministic via stable tuple or journaled sequence.

## Commands and events
- Commands: `ScheduleTimer`, `ScheduleDelayedAction`, `AdvanceClockTo`, `FireTimer`, `CancelTimer`.
- Events: `TimerScheduled`, `DelayedActionAdmitted`, `TimerFired`, numeric `WaitScheduled`/`AskScheduled` evidence, `TimerCancelled`.

## Policies
- `AdvanceClockTo(new_tick)` rejects `new_tick < current_tick` before mutation.
- Relative deadlines use checked `current_tick + duration`.
- `WaitUntil` consumes validated absolute `TimerDeadline`; `WaitEvent`/`Ask` consume validated `TimerDuration` unless later compiler contract normalizes them.
- Host wall clock may exist only as an adapter producing explicit tick commands; replay uses recorded ticks.

## Open decisions
- Zero delay: immediate-fireable at current tick vs typed rejection.
- Final public API names.
- Migrate existing `TimerWheel` in place vs replace/fence it.
