bead_id: vb-f7k6
bead_title: Add TLA+ Timer Wheel Model
phase: 2
updated_at: 2026-05-18T17:32:11Z
attempt: 1-of-7

# Codebase Map — Timer Wheel TLA+ Model

## Explore Evidence

- Direct child task: `ses_1c3d8f5beffenKLh205bf3E609`, `[vb-f7k6] p2-explore`.
- Child could not write artifacts in its role, but returned scoped read/search findings from isolated workspace `/home/lewis/src/go-skill-vb-f7k6`.
- Orchestrator materialized this non-production artifact from that child output and marks State 2 attempt 1 as repaired for missing artifact write only.

## Primary Runtime Code

- `crates/vb_runtime/src/shard/timer_wheel.rs`
  - Owns `TimerWheel` with deadline index `BTreeMap<Instant, Vec<TimerEntry>>` and run index `HashMap<RunId, (Instant, PendingTimerKind)>`.
  - Public/observable APIs: `insert`, `cancel`, `fire_expired`, `next_deadline`, `is_empty`, `len`, `get_kind`.
- `crates/vb_runtime/src/shard/transitions.rs`
  - `await_timer` records `pending_timers` and journals `WaitScheduled` / `AskScheduled`.
- `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs`
  - `handle_timer` removes pending timer, validates fire, emits `WaitResolved` for wait timers, then drives the run.
- `crates/vb_runtime/src/runtime.rs`
  - `timer_fired(run)` enqueues `ShardCommand::TimerFired`.

## Existing Formal/Verification Context

- `specs/tla/ShardScheduler.tla`
- `verification/tla/RetryFSM.tla`
- `specs/tla/BudgetArithmetic.tla`
- `xtask/src/lanes.rs` contains TLA lane behavior expecting `verification/tla/{crate_name}.tla`, while existing evidence often uses direct `tlc -config ...` commands.
- Related concurrency model: `crates/vb_runtime/src/models/loom/timer_fired_cancel.rs`; current loom model appears to assert consistency/no panic rather than exact fire/cancel outcome lattice.

## Scope Risks

- Bounded time arithmetic must not use unbounded `Nat` to hide overflow.
- Deadline addition overflow must transition to explicit error/suspend behavior and never wrap.
- One active timer per `RunId`; replacement cancels previous deadline index entry.
- Cancel must remove from both run-index and deadline-index.
- `fire_expired(now)` returns only deadlines `<= now` and removes from both indexes.
- Stale `TimerFired` after cancel must be rejected as invalid, not resurrect a run.
- Shutdown/cancel must clear pending timers and prevent mutation of terminal/cancelled runs.

## Recommended Formal Artifact Targets

- `verification/tla/TimerWheel.tla`
- `verification/tla/TimerWheel.cfg`

## Next State

State 3 must produce a formal contract before proof writing, with bounded hardware/time limits and explicit overflow/error transitions.
