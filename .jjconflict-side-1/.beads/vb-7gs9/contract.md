# Contract: vb-7gs9 — runtime: Shard scheduler bounded ownership evidence

## Bead
- **ID:** vb-7gs9
- **Title:** runtime: Shard scheduler bounded ownership evidence
- **Workspace:** /home/lewis/src/Velvet-ballistics/vb-7gs9-ws
- **State:** Contract synthesis

---

## 1. Overview

This contract governs the `Shard` type in `crates/vb_runtime/src/shard/`. The shard is a single-threaded scheduler that owns bounded mutable run state directly. It processes `ShardCommand` messages from a bounded `ArrayQueue`, drives deterministic execution until suspension, and emits an evidence chain (Phase 40/44) for every step: `StepStarted` → `SlotWritten` → `StepSucceeded`.

**Design assumptions derived from code research:**
- Shard owns runs via `IndexMap<RunId, RunState>` — no global run map.
- Command queue is `ArrayQueue<ShardCommand>` with capacity set at construction.
- Frame pools are `IndexMap<(u16 step_count, u16 slot_count), FramePool>` — dimension-keyed.
- Evidence is flushed per tick via `flush_evidence(run, collector)`.
- `ShardConfig::new` validates `command_queue_capacity` and `max_active_runs`.
- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg` in first-party shard code.

---

## 2. EARS Preconditions and Postconditions

### 2.1 `Shard::new(config: ShardConfig) -> Shard`

**Preconditions:**
- `config.command_queue_capacity > 0`
- `config.command_queue_capacity <= MAX_COMMAND_QUEUE_CAPACITY` (65_536)
- `config.max_active_runs > 0`

**Postconditions:**
- Returns a `Shard` with `command_queue` capacity exactly `config.command_queue_capacity`
- Returns a `Shard` with `runs` empty
- Returns a `Shard` with `pending_timers` empty
- Returns a `Shard` with `frame_pools` empty
- Returns a `Shard` with `shutting_down == false`
- Returns a `Shard` with `trace_ring` initialized to `config.trace_capacity`
- Returns a `Shard` with `step_budget_per_tick == config.step_budget_per_tick`
- Returns a `Shard` with `max_active_runs == config.max_active_runs`
- Returns a `Shard` with `policy == config.policy`

---

### 2.2 `Shard::enqueue(cmd: ShardCommand) -> RuntimeResult<()>`

**Preconditions:**
- Shard is not in shutting-down state

**Postconditions (success):**
- Command is enqueued; `command_queue_len() == old(command_queue_len) + 1`
- `remaining_capacity() == old(remaining_capacity) - 1`

**Postconditions (QueueFull):**
- Returns `RuntimeError::QueueFull`
- Command queue is unchanged

---

### 2.3 `Shard::tick() -> RuntimeResult<bool>`

**Preconditions:**
- None (tick is idempotent on empty queue)

**Postconditions (shutting_down set):**
- Returns `Ok(false)`
- `is_shutting_down() == true`

**Postconditions (queue empty):**
- Returns `Ok(true)`
- No shard state mutated

**Postconditions (command processed):**
- Processes exactly one `ShardCommand` from the queue (FIFO order)
- Returns `Ok(true)` if shard continues (no shutdown)
- Returns `Ok(false)` if `Shutdown` command was processed

**Postconditions (Submit success):**
- `runs.get(&run)` returns `Some(RunState)` if workflow finishes synchronously (e.g., SetConst→Finish), `runs` may be empty after tick
- `counters().snapshot().runs_submitted` incremented

**Postconditions (Submit failure — duplicate run):**
- Returns `RuntimeError::RunAlreadyExists`

**Postconditions (Submit failure — max active runs):**
- Returns `RuntimeError::ActiveRunCapacityExceeded { capacity }`

**Postconditions (Submit failure — queue full during submit):**
- Returns `RuntimeError::QueueFull`

**Postconditions (Resume success):**
- Run is driven forward; returns `Ok(true)`
- `runs.get(&run)` still present if run suspends again

**Postconditions (Resume — unknown run):**
- Returns `RuntimeError::RunNotFound`

**Postconditions (ActionCompleted — unknown run):**
- Returns `RuntimeError::RunNotFound`

**Postconditions (ActionCompleted — success):**
- Frame advances; step marked Succeeded
- Trace event `ActionCompleted { run, step }` emitted

**Postconditions (ActionFailed — no error handler):**
- Run marked Failed; removed from `runs`
- `counters().snapshot().runs_failed` incremented

**Postconditions (ActionFailed — has error handler):**
- Run routes to error handler step
- `counters().snapshot().runs_failed` NOT incremented (handled)

**Postconditions (TimerFired — unknown run):**
- Returns `RuntimeError::RunNotFound`

**Postconditions (TimerFired — invalid timer fire):**
- Returns `RuntimeError::InvalidTimerFire`

**Postconditions (TimerFired — success):**
- Timer consumed; pending timer removed
- Run advances; counters updated

**Postconditions (Cancel — success):**
- Run removed from `runs`
- `counters().snapshot().runs_failed` incremented
- Trace event `RunCancelled { run }` emitted
- `pending_timers` entry removed if present
- Frame returned to pool

**Postconditions (Cancel — nonexistent run):**
- No counter mutation; returns `Ok(true)` (idempotent)

**Postconditions (Inspect):**
- `inspect_response` set to `Some(InspectResponse::Found(...))` or `NotFound`
- No mutation of run state

**Postconditions (Shutdown):**
- `shutting_down` flag set to `true`
- Returns `Ok(false)` — subsequent `tick()` calls return `Ok(false)` immediately

---

### 2.4 `Shard::drain_for_shutdown() -> RuntimeResult<()>`

**Preconditions:**
- Shard may or may not be shutting down

**Postconditions (all commands processed before capacity limit):**
- Returns `Ok(())`
- `pending_timers` cleared
- `shutting_down == true`

**Postconditions (capacity limit hit before shutdown command):**
- Returns `RuntimeError::ShutdownInProgress`
- No `pending_timers` cleared

---

### 2.5 `Shard::flush_evidence(run, collector) -> RuntimeResult<()>`

**Preconditions:**
- `collector` contains `EvidenceEvent` sequence
- Journal is available

**Postconditions (success):**
- All events drained from `collector`
- Each `StepStarted` produces `TraceEvent::StepStarted` and `RuntimeJournalEvent::StepStarted`
- Each `StepSucceeded` produces `RuntimeJournalEvent::StepSucceeded`
- Each `SlotWritten` produces `TraceEvent::SlotWritten` and `RuntimeJournalEvent::SlotWritten`

---

### 2.6 `Shard::take_frame_for(run, workflow) -> RuntimeResult<RunFrame>`

**Preconditions:**
- `workflow` has non-zero `node_count()` and `slot_count()`

**Postconditions (success):**
- Returns a `RunFrame` allocated from the dimension-keyed pool
- Pool is created if absent
- Frame has `step_count == workflow.node_count()` and `slot_count == workflow.slot_count()`

**Postconditions (FramePoolUnavailable):**
- Returns `RuntimeError::FramePoolUnavailable`

---

### 2.7 `Shard::release_frame(frame)`

**Preconditions:**
- Frame was previously acquired via `take_frame_for`

**Postconditions:**
- Frame returned to correct dimension pool
- Pool's available count incremented

---

### 2.8 `ShardConfig::new(...) -> RuntimeResult<ShardConfig>`

**Preconditions:**
- `command_queue_capacity != 0`
- `command_queue_capacity <= MAX_COMMAND_QUEUE_CAPACITY`
- `max_active_runs != 0`

**Postconditions (validation failure):**
- `command_queue_capacity == 0` → `RuntimeError::CommandQueueCapacityExceeded { capacity: 0, max: 65_536 }`
- `command_queue_capacity > MAX_COMMAND_QUEUE_CAPACITY` → `RuntimeError::CommandQueueCapacityExceeded { capacity, max: 65_536 }`
- `max_active_runs == 0` → `RuntimeError::ActiveRunCapacityZero`

**Postconditions (success):**
- Returns `Ok(ShardConfig { ... })` with all fields preserved

---

## 3. Invariants

### 3.1 Bounded Ownership Invariants

| # | Invariant | Type |
|---|-----------|------|
| I1 | `runs.len() <= max_active_runs` always | Strong |
| I2 | `command_queue.len() <= command_queue_capacity` always | Strong |
| I3 | Each `RunId` appears in `runs` map at most once | Strong |
| I4 | Each `RunId` appears in `pending_timers` map at most once | Strong |
| I5 | `frame_pools` keys are derived from `workflow.node_count()` and `workflow.slot_count()` | Strong |
| I6 | `take_frame_for` and `release_frame` are paired per `(step_count, slot_count)` dimension | Strong |
| I7 | `shutting_down == true` is permanent; `tick()` always returns `Ok(false)` thereafter | Strong |
| I8 | `pending_timers` is cleared only on `drain_for_shutdown`, `Cancel`, `TimerFired` (wait/ask completion), or `AskAnswered` | Strong |

### 3.2 Evidence Chain Invariants (Phase 40/44)

| # | Invariant | Type |
|---|-----------|------|
| E1 | For every deterministic step: `StepStarted { run, step }` journal event appears before `SlotWritten { run, slot, value }` for that step | Strong |
| E2 | For every deterministic step: `StepSucceeded { run, step }` journal event appears after `SlotWritten` for that step | Strong |
| E3 | Evidence events are flushed before `tick()` returns `Ok(true)` (i.e., before returning control to scheduler) | Strong |
| E4 | `flush_evidence` produces exactly one `SlotWritten` per slot written by a step | Strong |
| E5 | `flush_evidence` produces exactly one `StepStarted` per step executed | Strong |

### 3.3 Run Lifecycle Invariants

| # | Invariant | Type |
|---|-----------|------|
| L1 | A run's frame is in a frame pool iff the run is active (in `runs` map) | Strong |
| L2 | When a run is removed from `runs` (finish/cancel/fail), its frame is returned to the correct pool | Strong |
| L3 | `Cancel` on a run with a pending timer removes the timer entry | Strong |
| L4 | `TimerFired` for a run without a pending timer returns `InvalidTimerFire` | Strong |
| L5 | Submitting a `RunId` already in `runs` returns `RunAlreadyExists` without mutating `runs` | Strong |

### 3.4 Queue Invariants

| # | Invariant | Type |
|---|-----------|------|
| Q1 | `enqueue` never blocks; it returns `QueueFull` immediately on overflow | Strong |
| Q2 | `tick` processes commands in FIFO order | Strong |
| Q3 | `tick` processes at most one command per invocation | Strong |
| Q4 | `drain_for_shutdown` processes at most `command_queue_capacity` commands | Strong |

---

## 4. Error Taxonomy

| Error | Trigger | Classification |
|-------|---------|----------------|
| `RuntimeError::QueueFull` | `enqueue` when `command_queue.len() == command_queue_capacity` | Ownership/Admission |
| `RuntimeError::RunAlreadyExists` | `Submit` when `runs.contains_key(run)` | Ownership |
| `RuntimeError::ActiveRunCapacityExceeded { capacity }` | `Submit` when `runs.len() == max_active_runs` | Ownership/Bounded |
| `RuntimeError::RunNotFound` | `Resume`, `ActionCompletedLegacy`, `ActionCompleted`, `ActionFailed`, `TimerFired` targeting absent `RunId` | Ownership |
| `RuntimeError::InvalidTimerFire` | `TimerFired` for run with no pending timer | Ownership |
| `RuntimeError::ShutdownInProgress` | `drain_for_shutdown` cannot drain all commands before capacity limit | Lifecycle |
| `RuntimeError::FramePoolUnavailable` | `take_frame_for` cannot acquire or create pool | Bounded/Resource |
| `RuntimeError::CommandQueueCapacityExceeded { capacity, max }` | `ShardConfig::new` with invalid capacity | Validation |
| `RuntimeError::ActiveRunCapacityZero` | `ShardConfig::new` with `max_active_runs == 0` | Validation |
| `RuntimeError::EncodeFailed` | `flush_slot_written` Postcard encoding failure | Evidence |

**Error severity classification:**
- **Fatal:** `ShutdownInProgress` — scheduler cannot drain cleanly; requires operator intervention
- **Ownership violations:** `RunAlreadyExists`, `RunNotFound`, `ActiveRunCapacityExceeded`, `InvalidTimerFire` — indicate caller bug or stale command
- **Bounded/resource:** `QueueFull`, `FramePoolUnavailable` — backpressure signal; caller should retry
- **Validation:** `CommandQueueCapacityExceeded`, `ActiveRunCapacityZero` — configuration errors at construction
- **Evidence:** `EncodeFailed` — journal integrity issue; run continues but evidence may be incomplete

---

## 5. Acceptance Tests

### 5.1 Happy Path Tests

| ID | Test | Validates |
|----|------|-----------|
| H1 | `config_new_accepts_min_valid_capacity` | `ShardConfig::new(1, 1, 1, 1, Relaxed)` → `Ok` |
| H2 | `shard_new_creates_empty_shard` | New shard: `active_run_count == 0`, `pending_timer_count == 0`, `command_queue_len == 0`, `is_shutting_down == false` |
| H3 | `enqueue_and_capacity_tracking` | After enqueue Shutdown: `command_queue_len == 1`, `remaining_capacity == 3` |
| H4 | `tick_processes_shutdown_returns_false` | Enqueue Shutdown → tick → `is_shutting_down == true`, returns `false` |
| H5 | `tick_after_shutdown_always_returns_false` | After shutdown: two consecutive `tick()` calls both return `false` |
| H6 | `drain_for_shutdown_processes_pending_commands` | Submit + Shutdown → drain → `runs_completed == 1`, `shutting_down == true` |
| H7 | `finished_run_releases_frame_to_dimension_pool` | After finished workflow: pool for `(2, 1)` has `available == 1` |
| H8 | `cancelled_run_releases_frame_to_dimension_pool` | After cancel: pool `(1, 1).available == 1` |
| H9 | `cancel_cleans_pending_timer` | Submit wait workflow → cancel: `pending_timers.len() == 0` after cancel tick |
| H10 | `shard_tick_processes_commands_in_fifo_order` | Two submits → two ticks in order → both runs accounted for |
| H11 | `shard_resume_continues_suspended_run` | Submit suspended → Resume → tick succeeds |
| H12 | `action_failed_routes_to_nearqby_error_handler` | `ActionFailed` on run with error handler → routes to handler, not fail |
| H13 | `finish_cleans_pending_timer_after_timer_fire` | Wait workflow → timer fires → `pending_timers.len() == 0`, `runs_completed == 1` |
| H14 | `shard_submit_drives_run_immediately_for_finished_workflow` | Submit finished workflow → tick → `runs_completed == 1`, run not in `runs` map |
| H15 | `flush_evidence_produces_step_started_before_slot_written` | After step: journal contains `StepStarted` then `SlotWritten` then `StepSucceeded` |
| H16 | `snapshot_run_returns_not_found_for_missing_run` | `snapshot_run(RunId::new(999), 42)` → `InspectResponse::NotFound { run: 999, correlation: 42 }` |
| H17 | `status_reports_shard_health_and_capacity_without_mutation` | Enqueue Shutdown → `status()` → queue depth unchanged after call |

### 5.2 Error Path Tests

| ID | Test | Validates |
|----|------|-----------|
| E1 | `config_new_rejects_zero_capacity` | `new(0, 1, 1, 1, Relaxed)` → `Err(CommandQueueCapacityExceeded { capacity: 0, max: 65536 })` |
| E2 | `config_new_rejects_capacity_exceeding_max` | `new(65537, 1, 1, 1, Relaxed)` → `Err(CommandQueueCapacityExceeded { capacity: 65537, max: 65536 })` |
| E3 | `config_new_rejects_zero_max_active_runs` | `new(1, 1, 1, 0, Relaxed)` → `Err(ActiveRunCapacityZero)` |
| E4 | `queue_full_at_capacity_boundary` | Queue capacity 2 → 3rd enqueue → `Err(QueueFull)` |
| E5 | `enqueue_returns_queue_full_when_capacity_exceeded` | Queue capacity 2 → 3rd enqueue → `Err(QueueFull)` |
| E6 | `submit_returns_run_already_exists_for_duplicate` | Submit same `RunId` twice → second tick → `Err(RunAlreadyExists)` |
| E7 | `submit_returns_active_run_capacity_exceeded_at_limit` | max_active_runs=1 → two submits → second tick → `Err(ActiveRunCapacityExceeded { capacity: 1 })` |
| E8 | `shard_resume_returns_error_for_unknown_run` | Resume non-existent → tick → `Err(RunNotFound)` |
| E9 | `shard_action_completed_returns_error_for_unknown_run` | ActionCompletedLegacy for unknown run → tick → `Err(RunNotFound)` |
| E10 | `shard_timer_returns_error_for_unknown_run` | TimerFired for unknown run → tick → `Err(RunNotFound)` |
| E11 | `shard_timer_rejects_run_without_pending_timer` | Submit Do workflow (no wait/ask) → TimerFired → tick → `Err(InvalidTimerFire)` |
| E12 | `action_failed_without_error_handler_fails_run` | ActionFailed on suspended run with no handler → tick → `runs_failed == 1` |
| E13 | `submit_returns_active_run_capacity_exceeded_at_limit` | max_active_runs=1 → second submit at capacity → tick → `Err(ActiveRunCapacityExceeded)` |
| E14 | `drain_for_shutdown_on_empty_queue_hits_capacity_limit` | Empty queue, capacity=2 → drain → `Err(ShutdownInProgress)` |
| E15 | `shard_rejects_active_run_capacity_overflow` | max_active_runs=1 → second submit at capacity → tick → `Err(ActiveRunCapacityExceeded { capacity: 1 })` |

### 5.3 Evidence Chain Tests (Phase 40/44)

| ID | Test | Validates |
|----|------|-----------|
| EV1 | `flush_evidence_emits_step_started_before_slot_written` | Evidence collector with `StepStarted { step: 0 }` → flush → journal[0] is `StepStarted`, journal[1] is `SlotWritten` |
| EV2 | `flush_evidence_emits_step_succeeded_after_slot_written` | Evidence with `StepStarted + SlotWritten + StepSucceeded` → flush → order preserved |
| EV3 | `flush_slot_written_encodes_with_postcard` | SlotWritten → encoded payload decodes to original `SlotValue` |
| EV4 | `flush_evidence_drains_completely` | After flush → collector is empty |

### 5.4 Frame Pool Tests

| ID | Test | Validates |
|----|------|-----------|
| FP1 | `frame_pool_metrics_zero_initially` | New shard → `(0, 0)` |
| FP2 | `take_frame_for_creates_pool_if_absent` | `take_frame_for` for new dimension → pool created |
| FP3 | `release_frame_returns_to_correct_pool` | Frame with `(step_count=2, slot_count=1)` → released → pool `(2, 1).available == 1` |
| FP4 | `release_frame_ignores_unknown_key` | Release frame for unknown dimension → no panic, no mutation |

---

## 6. File Reads

The following files were read to synthesize this contract:

| File | Purpose |
|------|---------|
| `velvet-ballistics-MASTER.md` | Authoritative build contract; Phase 28 (shard scheduler), Phase 40/44 (evidence chain), Holzmann rules, error codes, resource contracts |
| `crates/vb_runtime/src/shard/impl_.rs` | Shard construction, queue operations, tick processing, evidence flushing, frame pool management, drain_for_shutdown |
| `crates/vb_runtime/src/shard/types.rs` | `Shard`, `ShardConfig`, `ShardCommand`, `RunState`, `InspectResponse`, `ShardStatus`, `ShardHealth`, `MAX_COMMAND_QUEUE_CAPACITY` |
| `crates/vb_runtime/src/shard/tests.rs` | 100+ unit tests validating happy/error/edge paths, counter semantics, timer handling, cancel behavior |

---

## 7. Constraints from MASTER.md

- `#![forbid(unsafe_code)]` applies to all shard code
- No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`
- No unchecked indexing (`[]`) on hot paths — use checked `get()` or custom `CheckedIndex`
- `ArrayQueue` is mandatory for bounded MPSC command queue
- `IndexMap` is used for `runs` and `frame_pools` (deterministic iteration order)
- Phase 40/44 evidence chain: `StepStarted` → `SlotWritten` → `StepSucceeded` per step
- `MAX_COMMAND_QUEUE_CAPACITY = 65_536`
- No task-per-step behavior; synchronous deterministic execution until suspension
