# Contract: vb-99n6 — Timer Wheel Driven Resume and Cancellation Hardening

## Bead Identification

- **Bead ID:** vb-99n6
- **Workspace:** /home/lewis/src/Velvet-ballistics/vb-99n6-ws
- **Phase:** runtime
- **Category:** timer-wheel, resume, cancellation, hardening
- **Governing document:** velvet-ballistics-MASTER.md

---

## 1. Context and Motivation

The runtime shard (`Shard`) manages run lifecycles including suspension on `WaitUntil`, `WaitEvent`, and `Ask` primitives. When a run suspends awaiting a timer deadline or ask timeout, the shard registers a `PendingTimer` in `pending_timers: IndexMap<RunId, PendingTimer>`. A separate `TimerWheel` data structure (`timer_wheel.rs`) provides O(log n) timer management via dual-index BTreeMap+HashMap.

This bead HARDENS three critical paths:

1. **Resume correctness**: `handle_resume` must correctly re-drive a run that is suspended on a pending timer WITHOUT consuming or invalidating the timer. The run may have been pre-empted by a concurrent timer fire, cancelled, or awaiting external action completion.

2. **Cancellation cleanup**: `handle_cancel` must atomically remove the pending timer and run state, preventing stale timer fires from referencing deallocated state.

3. **Timer fire atomicity**: `handle_timer` must correctly handle the case where the timer has already been consumed (e.g., by a prior timer fire, cancellation, or ask answer), returning `InvalidTimerFire` without corrupting shard state.

The `TimerWheel` provides `fire_expired(now)` which returns all expired `TimerEntry` records. Integration with the shard requires that `ShardCommand::TimerFired` carries only a `RunId` — the shard must validate the timer is still live before advancing state.

---

## 2. EARS Preconditions and Postconditions

### 2.1 `handle_resume`

**Precondition (Ubiquitous):**
- The shard is not shutting down (`shard.shutting_down == false`).

**Precondition (Resumable run):**
- `self.runs.contains_key(&run)` is `true` — the run exists and is owned by this shard.
- The run's frame is in a valid state with `pc` pointing to a step that can be driven.

**Postcondition (Successful resume):**
- If the run was suspended on an action (`AwaitingAction`): the run remains in `self.runs` with the same `pc`, and `drive_run` re-enters the action-suspension path producing the same or newer `ActionTicket`.
- If the run was suspended on a wait timer (`AwaitingWait`): the run remains in `self.runs` with the same `pc` and `pending_timers` entry unchanged. Subsequent `TimerFired` command for this `run` must still find a valid timer and succeed.
- If the run was suspended on an ask timer (`AwaitingAsk`): the run remains in `self.runs` with the same `pc` and `pending_timers` entry unchanged. Subsequent `TimerFired` command for this `run` must still find a valid timer.
- If the run was suspended on `AwaitingWait` but the deadline has already passed: `handle_resume` MUST NOT auto-fire the timer. The timer remains registered. The caller is responsible for enqueueing `TimerFired`.
- `self.pending_timers` is not mutated by `handle_resume` unless `drive_deterministic_full` transitions to a new suspension point that has a different timer kind.

**Postcondition (Run not found):**
- If `self.runs.get(&run)` is `None`, `handle_resume` returns `RuntimeError::RunNotFound`.

---

### 2.2 `handle_cancel`

**Precondition (Ubiquitous):**
- The shard is not shutting down.

**Precondition (None required on RunId):**
- `handle_cancel` accepts any `RunId`, including non-existent ones. Cancellation of a non-existent run is a no-op that returns `Ok(())`.

**Postcondition (Run exists):**
- After `handle_cancel(run)` returns `Ok(())`:
  - `self.runs.get(&run)` is `None` (run removed from map).
  - `self.pending_timers.get(&run)` is `None` (timer removed if present).
  - `RunCancelled` journal event is appended.
  - `TraceEvent::RunCancelled { run }` is pushed to trace ring.
  - `counters.runs_failed` is incremented.
  - The run's frame is released to the appropriate `FramePool`.

**Postcondition (Run did not exist):**
- If the run did not exist: `Ok(())` is returned, `runs_failed` counter is NOT incremented, no journal or trace events are emitted.

**Postcondition (Atomicity):**
- If `self.pending_timers.swap_remove(&run)` succeeds (timer existed), the timer MUST NOT be re-inserted or recoverable via subsequent `TimerFired` commands for the same `run`.

---

### 2.3 `handle_timer`

**Precondition (Ubiquitous):**
- The shard is not shutting down.

**Precondition (Timer must be registered):**
- `self.pending_timers.get(&run)` must return `Some(PendingTimer { step, kind })`.

**Postcondition (Valid timer fire):**
- The pending timer is removed from `self.pending_timers` via `swap_remove` BEFORE driving the run.
- If `timer.kind == PendingTimerKind::Wait`: `WaitScheduled` journal event was previously emitted; upon fire, `WaitResolved { run, step }` is appended.
- If `timer.kind == PendingTimerKind::Ask`: upon fire (timeout), the run is failed via `RuntimeSignal::AwaitingAsk` matching failure path in `apply_drive_result`.
- `advance_after_timer_fire` is called with the removed timer.
- The run is driven via `drive_state`.
- `apply_drive_result` is called with the result:
  - `Continue | StepBudgetExhausted` → run stays active in `self.runs`.
  - `Finished` → run is finished and removed.
  - `AwaitingAction | AwaitingWait | AwaitingAsk` → run stays active with new suspension.
  - `Err` → run is failed and removed.

**Postcondition (Stale timer fire — timer already consumed):**
- If `self.pending_timers.swap_remove(&run)` returns `None` (timer already removed):
  - The run state is re-inserted into `self.runs` via `self.runs.insert(run, state)`.
  - `RuntimeError::InvalidTimerFire` is returned.
  - No timer-kind-specific state is advanced.
  - The run is NOT removed from `self.runs`.

**Postcondition (Run not found):**
- If `self.take_run_state(run)` returns `RunNotFound` (run already removed), `RuntimeError::RunNotFound` is returned.

---

### 2.4 `await_timer`

**Precondition (Ubiquitous):**
- Called from `apply_drive_result` when `RuntimeSignal` is `AwaitingWait` or `AwaitingAsk`.

**Precondition (Timer registration needed):**
- `timer_registration_required(state, step)` returns `true` for the current `pc`.
- `PendingTimerKind` must be `Wait` for `AwaitingWait`, `Ask` for `AwaitingAsk`.

**Postcondition (Timer registered):**
- `self.pending_timers.insert(run, PendingTimer { step, kind })` is called.
- `WaitScheduled` or `AskScheduled` journal event is appended.
- The run state is inserted into `self.runs`.

**Postcondition (No-op for non-timed steps):**
- If `timer_registration_required` returns `false`, `pending_timers` is NOT modified, and the run is inserted into `self.runs` without any timer registration.

---

### 2.5 TimerWheel Integration (separate from shard command path)

**Invariant (Dual-index consistency):**
- After every `TimerWheel::insert`, `TimerWheel::cancel`, or `TimerWheel::fire_expired`, `by_deadline` and `by_run` must contain the same set of entries (just indexed differently).

**Invariant (Timer replacement):**
- `insert` for an existing run cancels the previous timer before inserting the new one.

**Invariant (fire_expired cleanup):**
- `fire_expired` must remove all expired entries from both `by_deadline` and `by_run`.

**Invariant (No timer for cancelled run):**
- After `cancel(run)` returns `true`, `get_kind(run)` returns `None` and `by_run` has no entry for `run`.

---

## 3. Invariants

### 3.1 Per-Run Invariants

- **I-1:** A run has AT MOST ONE pending timer at any time. If a second timer is registered (e.g., `Resume` re-schedules a wait after an ask), the first timer is replaced.
- **I-2:** If `pending_timers.get(&run)` is `Some(timer)`, then `self.runs.get(&run)` is also `Some(state)` — a timer cannot exist for a run that is not in the runs map.
- **I-3:** If `self.runs.get(&run)` is `Some(state)` and `state.frame.pc()` points to a `WaitUntil`, `WaitEvent(timeout)`, or `Ask(timeout)` node, then `pending_timers.get(&run)` MUST be `Some` unless the deadline has passed and the timer already fired.
- **I-4:** A run that is in the `runs` map with a `PendingTimerKind::Ask` timer MUST eventually either: (a) receive an `AskAnswer` that removes the timer, or (b) receive a `TimerFired` that fails the run on timeout. It MUST NOT silently leak.

### 3.2 Global Invariants

- **I-5:** `pending_timers.len() <= self.runs.len()` — not more timers than runs.
- **I-6:** `handle_cancel` is idempotent: calling it twice with the same `run` returns `Ok(())` on the second call and does not double-increment counters.
- **I-7:** `handle_timer` for a run that is NOT in `self.runs` (already finished or cancelled) returns `RuntimeError::RunNotFound` — NOT `InvalidTimerFire`.
- **I-8:** After `handle_timer` returns `Ok(())`, the timer for that run is NOT present in `pending_timers`.
- **I-9:** After `handle_cancel` returns `Ok(())`, the run is NOT present in `self.runs` AND the timer (if any) is NOT present in `pending_timers`.
- **I-10:** `finish_run` removes the pending timer for the run before appending `RunFinished` to the journal.
- **I-11:** `fail_run_state` removes the pending timer for the run before appending `RunFailed` to the journal.

---

## 4. Error Taxonomy

| Error Variant | Condition | Terminal State |
|---|---|---|
| `RuntimeError::RunNotFound` | `self.runs.get(&run)` is `None` | Run never existed or already removed |
| `RuntimeError::InvalidTimerFire` | `pending_timers.get(&run)` is `None` but run is still active | Timer already consumed by prior fire, cancel, or ask-answer |
| `RuntimeError::QueueFull` | Command queue at `MAX_COMMAND_QUEUE_CAPACITY` | Command rejected, caller retries |
| `RuntimeError::ActiveRunCapacityExceeded` | `self.runs.len() >= max_active_runs` | Submit rejected |
| `RuntimeError::RunAlreadyExists` | `self.runs.contains_key(&run)` on submit | Duplicate submit rejected |
| `RuntimeError::StaleAttempt` | Action completion attempt < current attempt counter | Completion rejected |
| `RuntimeError::AttemptBeyondMax` | Action completion attempt > ticket.capacity | Completion rejected |
| `RuntimeError::InvalidActionCompletion` | Ticket step not in `Running` state, or wrong action | Completion rejected |
| `RuntimeError::ShutdownInProgress` | `drain_for_shutdown` called on non-empty queue | Shutdown aborted |

---

## 5. Behavioral Edge Cases

### 5.1 Resume While Timer Pending

**Scenario:** Run A is suspended on `WaitUntil`. `pending_timers` contains `{RunId(A) → PendingTimer { step: 5, kind: Wait }}`. External caller enqueues `Resume { run: A }`.

**Expected:** `handle_resume` calls `drive_run(A)`. The run is driven. If the deadline has NOT passed, `drive_deterministic_full` returns `AwaitingWait` and `await_timer` re-registers the SAME timer (or finds it already there). The timer for Run A remains valid. A subsequent `TimerFired` for Run A still succeeds.

### 5.2 Timer Fire Race (Timer Already Consumed by AskAnswer)

**Scenario:** Run A is suspended on `Ask` with `PendingTimerKind::Ask`. Before the timer fires, an `AskAnswer` arrives. `handle_ask_answer` calls `swap_remove` on `pending_timers` and removes the timer. Then a `TimerFired` for Run A is enqueued.

**Expected:** `handle_timer` calls `take_run_state`. Since the timer is already removed, `pending_timers.swap_remove(&run)` returns `None`. The run state is re-inserted into `self.runs`. `RuntimeError::InvalidTimerFire` is returned. The run remains active in the `runs` map.

### 5.3 Cancel Then Timer Fire

**Scenario:** Run A is suspended on `WaitUntil`. `Cancel { run: A }` is enqueued and processed. `handle_cancel` removes the run from `self.runs` and removes the timer from `pending_timers`. Then `TimerFired { run: A }` is enqueued.

**Expected:** `handle_timer` calls `take_run_state` which returns `RuntimeError::RunNotFound`. `handle_timer` returns `RuntimeError::RunNotFound` (NOT `InvalidTimerFire`). The `InvalidTimerFire` path is only taken when the run exists but the timer has been cleared/consumed.

### 5.4 Resume After Timer Fire (Already Advanced)

**Scenario:** Run A is suspended on `WaitUntil`. Deadline passes. `TimerFired { run: A }` is processed. `advance_after_timer_fire` marks the wait step as succeeded and advances PC to the next step. The run finishes or suspends on a new step. Then `Resume { run: A }` is enqueued.

**Expected:** `handle_resume` calls `drive_run`. The run is driven from its current PC. If the run is already finished, `drive_run` returns and the run is not in `self.runs`. `handle_resume` returns `Ok(())` and the subsequent `self.runs.get(&run)` lookup in `take_run_state` returns `RunNotFound` on a subsequent call.

### 5.5 Timer Fire After Finish

**Scenario:** Run A reaches `Finish` node. `finish_run` is called. It removes the pending timer (via `swap_remove`) and releases the frame. Then `TimerFired { run: A }` is enqueued.

**Expected:** `handle_timer` calls `take_run_state`. Since the run was removed, returns `RuntimeError::RunNotFound`. This is distinct from `InvalidTimerFire` which requires the run to exist but the timer to be absent.

### 5.6 PendingTimer Last-Wins Semantics

**Scenario:** Run A is suspended on `Ask`. `pending_timers` has `{RunId(A) → PendingTimer { step: 3, kind: Ask }}`. A `Resume` triggers re-drive which again hits the `Ask` node and calls `await_timer`. The second `insert` replaces the existing entry.

**Expected:** `pending_timers.len()` remains 1. The timer entry for Run A is updated with the new step and same kind. Only the most recent timer registration matters.

---

## 6. Acceptance Tests

### 6.1 Happy Path Tests

**AT-1: Timer fire advances wait to completion**
- Submit workflow with `SetConst → WaitUntil → Finish` where deadline slot is set to a past or soon deadline.
- After submit tick, `pending_timers.len() == 1`.
- Enqueue `TimerFired { run }`.
- After tick, `pending_timers.len() == 0`, `runs_completed == 1`.

**AT-2: Ask answer cleans timer**
- Submit workflow with `SetConst → SetConst → Ask → AskResume → Finish`.
- After submit tick, `pending_timers.len() == 1`.
- Enqueue `AskAnswer` with correct `AskTicket`.
- After tick, `pending_timers.len() == 0`, run completes.

**AT-3: Cancel cleans timer**
- Submit timed wait workflow.
- After submit, `pending_timers.len() == 1`.
- Enqueue `Cancel { run }`.
- After tick, `pending_timers.len() == 0`, `runs_failed == 1`.

**AT-4: Resume re-drives action-suspended run**
- Submit workflow with `Do` action at step 0.
- Run suspends on action at step 0.
- Enqueue `Resume { run }`.
- After tick, run is still active (suspended again or completed).

**AT-5: Resume re-drives timer-suspended run without consuming timer**
- Submit timed wait workflow with a future deadline.
- Run suspends on wait with timer registered.
- Enqueue `Resume { run }`.
- After tick, `pending_timers.len() == 1` (timer still present).
- Subsequent `TimerFired { run }` succeeds and completes the run.

### 6.2 Error Path Tests

**AT-6: TimerFired on unknown run returns RunNotFound**
- Enqueue `TimerFired { run: nonexistent }`.
- After tick, returns `RuntimeError::RunNotFound`.

**AT-7: TimerFired on run with no pending timer returns InvalidTimerFire**
- Submit action-suspended workflow (no timer registered).
- Enqueue `TimerFired { run }`.
- After tick, returns `RuntimeError::InvalidTimerFire`.

**AT-8: TimerFired after cancel returns RunNotFound**
- Submit timed wait workflow.
- Enqueue `Cancel { run }`.
- After tick, run removed and timer removed.
- Enqueue `TimerFired { run }`.
- After tick, returns `RuntimeError::RunNotFound`.

**AT-9: TimerFired after ask answer (stale) returns InvalidTimerFire**
- Submit ask workflow.
- Run suspends with ask timer registered.
- Enqueue `AskAnswer` — removes timer from `pending_timers`.
- Enqueue `TimerFired { run }` for the same run.
- After tick, returns `RuntimeError::InvalidTimerFire` (run still in `runs` map, timer gone).

**AT-10: Resume on unknown run returns RunNotFound**
- Enqueue `Resume { run: nonexistent }`.
- After tick, returns `RuntimeError::RunNotFound`.

**AT-11: Cancel on non-existent run succeeds silently**
- Enqueue `Cancel { run: nonexistent }`.
- After tick, returns `Ok(())`, `runs_failed` counter unchanged.

**AT-12: Duplicate cancel is idempotent**
- Submit action-suspended workflow.
- Enqueue `Cancel { run }`.
- Tick — run removed.
- Enqueue `Cancel { run }` again.
- Tick — returns `Ok(())`, counter unchanged (still `runs_failed == 1`).

**AT-13: TimerFired after finish returns RunNotFound**
- Submit `SetConst → WaitUntil → Finish` workflow.
- Wait deadline expires; `TimerFired` advances to Finish which completes.
- Run removed from `runs`.
- Enqueue `TimerFired { run }`.
- After tick, returns `RuntimeError::RunNotFound`.

### 6.3 Timer Wheel Unit Tests

**AT-14: Timer wheel insert and fire_expired**
- Insert two timers: one expired, one future.
- `fire_expired(now)` returns only the expired entry.
- Future entry remains.

**AT-15: Timer wheel cancel removes from both indexes**
- Insert timer for RunId(1).
- `cancel(RunId(1))` returns `true`.
- `by_deadline` and `by_run` both empty.
- `fire_expired` for any time returns empty.

**AT-16: Timer wheel replacement updates both indexes**
- Insert timer for RunId(1) at deadline D1.
- Insert timer for RunId(1) at deadline D2 (D2 > D1).
- `len() == 1`, `get_kind` returns the second kind.
- `next_deadline == D2`.
- `fire_expired(D1)` returns empty.

---

## 7. Implementation Constraints

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`.
- No unchecked indexing, slicing, or casts.
- No `unsafe` in timer_wheel.rs or lifecycle.rs or transitions.rs.
- `pending_timers` is `IndexMap<RunId, PendingTimer>` — a BTreeMap-backed index.
- `TimerWheel` uses `BTreeMap<Instant, Vec<TimerEntry>>` + `HashMap<RunId, (Instant, PendingTimerKind)>`.
- All timer operations (`insert`, `cancel`, `fire_expired`) must maintain dual-index consistency.
- `RuntimeSignal::AwaitingAsk` timer fire must fail the run, not advance it.
- `handle_timer` must remove the timer BEFORE driving the run to prevent double-fire races.

---

## 8. Files In Scope

| File | Role |
|---|---|
| `crates/vb_runtime/src/shard/impl_.rs` | `tick()` dispatch, `handle_resume`, `enqueue`, queue ops |
| `crates/vb_runtime/src/shard/lifecycle.rs` | `handle_submit`, `handle_resume`, `handle_timer`, `handle_cancel`, `handle_ask_answer`, `handle_action_completion`, `handle_action_failure` |
| `crates/vb_runtime/src/shard/transitions.rs` | `keep_run`, `finish_run`, `await_action`, `await_timer`, `fail_run_state` |
| `crates/vb_runtime/src/shard/types.rs` | `Shard`, `RunState`, `PendingTimer`, `PendingTimerKind`, `ShardCommand` |
| `crates/vb_runtime/src/shard/timer_wheel.rs` | `TimerWheel`, `TimerEntry`, dual-index timer management |
| `crates/vb_runtime/src/shard/helpers.rs` | `advance_after_timer_fire`, `timer_registration_required`, `validate_action_completion` |
| `crates/vb_storage/src/events.rs` | `JournalEvent` variants: `WaitScheduled`, `AskScheduled`, `WaitResolved`, `RunCancelled`, `RunFinished` |

---

*Contract synthesized from bead vb-99n6 research. EARS format. Implementation is OUT OF SCOPE for this contract document.*
