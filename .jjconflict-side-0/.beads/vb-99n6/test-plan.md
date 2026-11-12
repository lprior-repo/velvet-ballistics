# Test Plan: vb-99n6 — Timer Wheel Driven Resume and Cancellation Hardening

## 1. Overview

This plan covers testing for the timer wheel, resume, cancellation, and timer-fire hardening paths in `vb_runtime`. The testing strategy follows the Testing Trophy distribution with emphasis on unit tests for isolated logic and integration tests for cross-component interactions.

**Files under test:**
- `crates/vb_runtime/src/shard/timer_wheel.rs`
- `crates/vb_runtime/src/shard/lifecycle.rs`
- `crates/vb_runtime/src/shard/transitions.rs`
- `crates/vb_runtime/src/shard/helpers.rs`

**Files test doubles/benchmarks:**
- `crates/vb_runtime/src/shard/impl_.rs`
- `crates/vb_runtime/src/shard/types.rs`

---

## 2. Testing Trophy Distribution

| Layer | Count | Description |
|---|---|---|
| Unit tests | ~42 | TimerWheel ops, invariants, state transitions |
| Integration tests | ~18 | handle_*_command × scenario combos |
| Property-based tests | ~12 | Invariant checkers via proptest |
| E2E/BDD scenarios | ~8 | Full run lifecycle scenarios |

---

## 3. Unit Tests

### 3.1 TimerWheel — `timer_wheel.rs`

#### TW-UT-001: `insert` stores entry in both `by_deadline` and `by_run`
```
fn insert_stores_in_both_indexes()
insert(timer_entry) -> ()
assert: by_deadline.contains_key(deadline)
assert: by_run.contains_key(run_id)
assert: by_deadline.get(deadline).contains(run_id)
```

#### TW-UT-002: `insert` returns previous timer when replacing
```
fn insert_returns_previous_on_replacement()
pre: insert(RunId(1), deadline=D1, kind=Wait)
pre: insert(RunId(1), deadline=D2, kind=Ask)
assert: returns Some((D1, Wait))
assert: len() == 1
assert: get_kind(RunId(1)) == Ask
```

#### TW-UT-003: `cancel` returns `true` and removes from both indexes when present
```
fn cancel_removes_from_both_indexes()
pre: insert(RunId(1), deadline, kind)
assert: cancel(RunId(1)) == true
assert: by_deadline.is_empty()
assert: by_run.is_empty()
assert: get_kind(RunId(1)).is_none()
```

#### TW-UT-004: `cancel` returns `false` when run has no timer
```
fn cancel_returns_false_when_not_present()
assert: cancel(RunId(nonexistent)) == false
```

#### TW-UT-005: `fire_expired` returns only expired entries
```
fn fire_expired_returns_only_expired()
pre: insert(RunId(1), deadline=past, kind=Wait)
pre: insert(RunId(2), deadline=future, kind=Ask)
assert: fire_expired(now) == [TimerEntry(RunId(1), past, Wait)]
assert: len() == 1  // future entry remains
```

#### TW-UT-006: `fire_expired` removes from both indexes
```
fn fire_expired_removes_from_both_indexes()
pre: insert(RunId(1), deadline=past, kind)
pre: fire_expired(now) returns [entry]
assert: by_deadline.is_empty()
assert: by_run.is_empty()
```

#### TW-UT-007: `fire_expired` returns empty when no entries expired
```
fn fire_expired_returns_empty_when_no_expired()
pre: insert(RunId(1), deadline=future)
assert: fire_expired(now) == []
assert: len() == 1
```

#### TW-UT-008: `next_deadline` returns earliest deadline
```
fn next_deadline_returns_earliest()
pre: insert(RunId(1), deadline=D1)
pre: insert(RunId(2), deadline=D2) where D2 > D1
assert: next_deadline == Some(D1)
```

#### TW-UT-009: `next_deadline` returns `None` when empty
```
fn next_deadline_none_when_empty()
assert: next_deadline == None
```

#### TW-UT-010: `len` equals count of unique runs
```
fn len_equals_run_count()
pre: insert(RunId(1), deadline, kind)
pre: insert(RunId(2), deadline, kind)
assert: len() == 2
pre: insert(RunId(1), deadline, kind)  // replacement
assert: len() == 2  // still 2, not 3
```

#### TW-UT-011: `get_kind` returns correct kind
```
fn get_kind_returns_registered_kind()
pre: insert(RunId(1), deadline, kind=Wait)
assert: get_kind(RunId(1)) == Some(Wait)
```

#### TW-UT-012: `get_kind` returns `None` for cancelled run
```
fn get_kind_none_after_cancel()
pre: insert(RunId(1), deadline, Wait)
pre: cancel(RunId(1))
assert: get_kind(RunId(1)) == None
```

#### TW-UT-013: `fire_expired` is idempotent (double-fire returns empty)
```
fn fire_expired_idempotent()
pre: insert(RunId(1), deadline=past, kind)
pre: fire_expired(now) returns [entry]
assert: fire_expired(now) == []  // second call empty
assert: len() == 0
```

### 3.2 Transitions — `transitions.rs`

#### TR-UT-001: `await_timer` inserts pending timer for `AwaitingWait`
```
fn await_timer_inserts_for_awaiting_wait()
pre: RunState with frame.pc() at WaitUntil node
pre: deadline slot set
assert: await_timer(run, state, step) calls pending_timers.insert(run, PendingTimer { step, kind: Wait })
assert: WaitScheduled journal event appended
```

#### TR-UT-002: `await_timer` inserts pending timer for `AwaitingAsk`
```
fn await_timer_inserts_for_awaiting_ask()
pre: RunState with frame.pc() at Ask node
assert: await_timer(run, state, step) inserts PendingTimer { step, kind: Ask }
assert: AskScheduled journal event appended
```

#### TR-UT-003: `await_timer` is no-op when `timer_registration_required` is false
```
fn await_timer_noop_for_non_timed_step()
pre: RunState with frame.pc() at non-timer node (e.g., SetConst)
assert: pending_timers unchanged after await_timer call
assert: no journal event appended
```

#### TR-UT-004: `finish_run` removes pending timer before appending `RunFinished`
```
fn finish_run_removes_pending_timer()
pre: pending_timers.insert(run, PendingTimer { step, kind: Wait })
pre: finish_run(run) is called
assert: pending_timers.get(&run).is_none()
assert: RunFinished journal event appended
```

#### TR-UT-005: `fail_run_state` removes pending timer before appending `RunFailed`
```
fn fail_run_state_removes_pending_timer()
pre: pending_timers.insert(run, PendingTimer { step, kind: Ask })
pre: fail_run_state(run, error) is called
assert: pending_timers.get(&run).is_none()
assert: RunFailed journal event appended
```

#### TR-UT-006: `keep_run` re-inserts run into runs map after suspension
```
fn keep_run_reinserts_into_runs_map()
pre: take_run_state(run) extracted state
pre: keep_run(run, state) is called
assert: runs.get(&run) == Some(state)
```

### 3.3 Helpers — `helpers.rs`

#### HP-UT-001: `timer_registration_required` returns `true` for `WaitUntil` step
```
fn timer_reg_required_true_for_wait_until()
pre: frame.pc() points to WaitUntil
assert: timer_registration_required(state, step) == true
```

#### HP-UT-002: `timer_registration_required` returns `true` for `Ask(timeout)` step
```
fn timer_reg_required_true_for_ask()
pre: frame.pc() points to Ask with timeout
assert: timer_registration_required(state, step) == true
```

#### HP-UT-003: `timer_registration_required` returns `false` for `Finish` step
```
fn timer_reg_required_false_for_finish()
pre: frame.pc() points to Finish
assert: timer_registration_required(state, step) == false
```

#### HP-UT-004: `advance_after_timer_fire` updates frame for `PendingTimerKind::Wait`
```
fn advance_after_timer_fire_for_wait()
pre: PendingTimer { step, kind: Wait }
pre: advance_after_timer_fire(state, timer) called
assert: state.frame.pc() advanced past the WaitUntil step
```

#### HP-UT-005: `advance_after_timer_fire` signals failure for `PendingTimerKind::Ask`
```
fn advance_after_timer_fire_for_ask()
pre: PendingTimer { step, kind: Ask }
pre: advance_after_timer_fire(state, timer) called
assert: returned RuntimeSignal indicates failure
```

---

## 4. Integration Tests

### 4.1 handle_timer — Happy Paths

#### IT-TIMER-001: Timer fire advances `WaitUntil` to completion
```
Given: Shard with submitted workflow SetConst → WaitUntil → Finish
  And: deadline set to past time
  And: TimerFired { run } enqueued
When:  tick() processes command
Then:  pending_timers.len() == 0
  And: runs_completed == 1
  And: WaitResolved journal event appended
```

#### IT-TIMER-002: Timer fire fails `Ask` timeout
```
Given: Shard with submitted workflow SetConst → Ask → Finish
  And: ask timer registered (PendingTimerKind::Ask)
  And: TimerFired { run } enqueued
When:  tick() processes command
Then:  pending_timers.len() == 0
  And: runs_failed == 1
  And: RunFailed journal event appended
```

#### IT-TIMER-003: Ask answer cleans timer — subsequent TimerFired is stale
```
Given: Shard with submitted workflow SetConst → Ask → Finish
  And: ask timer registered
  And: AskAnswer received (removes timer from pending_timers)
  And: TimerFired { run } enqueued
When:  tick() processes command
Then:  returns RuntimeError::InvalidTimerFire
  And: run still in runs map (awaiting ask result)
```

### 4.2 handle_timer — Error Paths

#### IT-TIMER-004: TimerFired on unknown run returns `RunNotFound`
```
Given: Shard (empty, no runs)
When:  TimerFired { run: nonexistent } enqueued
Then:  tick() returns RuntimeError::RunNotFound
```

#### IT-TIMER-005: TimerFired on run with no pending timer returns `InvalidTimerFire`
```
Given: Shard with submitted action-suspended workflow (Do → Finish)
  And: no timer registered for the run
  And: TimerFired { run } enqueued
When:  tick() processes command
Then:  returns RuntimeError::InvalidTimerFire
  And: run still in runs map
```

#### IT-TIMER-006: TimerFired after cancel returns `RunNotFound`
```
Given: Shard with submitted timed workflow
  And: Cancel { run } processed (removes run and timer)
When:  TimerFired { run } enqueued
Then:  tick() returns RuntimeError::RunNotFound (NOT InvalidTimerFire)
```

#### IT-TIMER-007: TimerFired after finish returns `RunNotFound`
```
Given: Shard with workflow that completes (TimerFired advances to Finish)
When:  TimerFired { run } enqueued (for already-finished run)
Then:  tick() returns RuntimeError::RunNotFound
```

### 4.3 handle_cancel — Happy and Error Paths

#### IT-CANCEL-001: Cancel removes run and timer atomically
```
Given: Shard with submitted timed wait workflow
  And: pending_timers.len() == 1
When:  Cancel { run } enqueued and processed
Then:  runs.get(&run).is_none()
  And: pending_timers.get(&run).is_none()
  And: RunCancelled journal event appended
  And: TraceEvent::RunCancelled emitted
  And: counters.runs_failed == 1
```

#### IT-CANCEL-002: Cancel on non-existent run succeeds silently
```
Given: Shard (empty)
When:  Cancel { run: nonexistent } enqueued
Then:  tick() returns Ok(())
  And: runs_failed counter unchanged
  And: no journal event appended
```

#### IT-CANCEL-003: Duplicate cancel is idempotent
```
Given: Shard with submitted action-suspended workflow
  And: Cancel { run } processed (first time, run removed)
When:  Cancel { run } enqueued again
Then:  tick() returns Ok(())
  And: runs_failed counter still == 1 (not incremented again)
```

### 4.4 handle_resume — Happy and Error Paths

#### IT-RESUME-001: Resume re-drives action-suspended run
```
Given: Shard with run suspended on action (AwaitingAction)
When:  Resume { run } enqueued and processed
Then:  drive_run called
  And: run remains in runs map
  And: pending_timers unchanged
```

#### IT-RESUME-002: Resume re-drives wait-suspended run without consuming timer
```
Given: Shard with run suspended on WaitUntil with timer registered
  And: deadline has NOT passed
When:  Resume { run } enqueued and processed
Then:  drive_run called
  And: pending_timers.len() == 1 (timer still present)
  And: timer for this run still valid for subsequent TimerFired
```

#### IT-RESUME-003: Resume on run past deadline does NOT auto-fire timer
```
Given: Shard with run suspended on WaitUntil
  And: deadline has already passed
When:  Resume { run } enqueued and processed
Then:  drive_deterministic_full called
  And: timer remains registered in pending_timers
  And: caller is responsible for enqueueing TimerFired
```

#### IT-RESUME-004: Resume on unknown run returns `RunNotFound`
```
Given: Shard (empty)
When:  Resume { run: nonexistent } enqueued
Then:  tick() returns RuntimeError::RunNotFound
```

### 4.5 handle_ask_answer

#### IT-ASK-001: Ask answer removes timer and completes ask
```
Given: Shard with run suspended on Ask with timer registered
When:  AskAnswer { run, step, ticket } enqueued and processed
Then:  pending_timers.get(&run).is_none()
  And: timer removed before run is driven
```

---

## 5. Property-Based Tests (Proptest)

### 5.1 TimerWheel Invariants

#### PB-TW-001: Dual-index consistency after insert
```
proptest! {
  fn dual_index_consistency_after_insert(entries: Vec<(RunId, Instant, PendingTimerKind)>) {
    // For each insert, by_deadline and by_run must contain same set of entries
    // indexed by deadline and run_id respectively
  }
}
```

#### PB-TW-002: Dual-index consistency after cancel
```
proptest! {
  fn dual_index_consistency_after_cancel(entries: Vec<TimerEntry>, to_cancel: RunId) {
    // After cancel, both maps must be empty for that run
  }
}
```

#### PB-TW-003: Dual-index consistency after fire_expired
```
proptest! {
  fn dual_index_consistency_after_fire_expired(entries: Vec<TimerEntry>, now: Instant) {
    // After fire_expired, removed entries absent from both maps
  }
}
```

#### PB-TW-004: Replacement cancels previous timer
```
proptest! {
  fn replacement_cancels_previous(d1: Instant, d2: Instant) {
    // Insert for same RunId at different deadline replaces, not double-counts
    // len() stays == 1
  }
}
```

#### PB-TW-005: fire_expired never returns non-expired entries
```
proptest! {
  fn fire_expired_only_returns_expired(entries: Vec<TimerEntry>, now: Instant) {
    // All returned entries have deadline <= now
  }
}
```

### 5.2 Per-Run Invariants (State Machine)

#### PB-SM-001: At most one pending timer per run (I-1)
```
proptest! {
  fn at_most_one_timer_per_run(ops: Vec<TimerWheelOp>) {
    // Sequence of inserts/replaces/cancels
    // After each op, for each run, pending_timers has <= 1 entry
  }
}
```

#### PB-SM-002: Timer implies run exists (I-2)
```
proptest! {
  fn timer_implies_run_exists(ops: Vec<ShardOp>) {
    // If pending_timers.get(&run) is Some, then runs.get(&run) is Some
  }
}
```

#### PB-SM-003: Timer kind matches suspension point (I-3)
```
proptest! {
  fn timer_kind_matches_suspension(ops: Vec<ShardOp>) {
    // For runs in AwaitingWait, timer.kind == Wait
    // For runs in AwaitingAsk, timer.kind == Ask
  }
}
```

### 5.3 Global Invariants

#### PB-GLOBAL-001: pending_timers.len() <= runs.len() (I-5)
```
proptest! {
  fn timer_count_leq_run_count(ops: Vec<ShardOp>) {
    // Never more timers than runs
  }
}
```

#### PB-GLOBAL-002: handle_cancel idempotent (I-6)
```
proptest! {
  fn cancel_idempotent(run_id: RunId, state: ShardState) {
    // Calling cancel twice returns Ok(()) both times
    // Counter incremented only once
  }
}
```

#### PB-GLOBAL-003: RunNotFound vs InvalidTimerFire distinction (I-7)
```
proptest! {
  fn run_not_found_vs_invalid_timer_distinction(ops: Vec<ShardOp>) {
    // TimerFired on removed run -> RunNotFound
    // TimerFired on existing run with no timer -> InvalidTimerFire
  }
}
```

---

## 6. BDD Scenarios

### BDD-001: Timer fire race — timer consumed by AskAnswer
```
Feature: Timer fire atomicity
  Scenario: Timer fires after AskAnswer already consumed it
    Given a run suspended on Ask with timer registered
    When an AskAnswer arrives and consumes the timer
    And then a TimerFired command is enqueued for the same run
    Then handle_timer returns InvalidTimerFire
    And the run remains in the runs map
    And the run is NOT removed
```

### BDD-002: Cancel then timer fire
```
Feature: Cancel timer interaction
  Scenario: Timer fire after cancel returns RunNotFound
    Given a run suspended on WaitUntil with timer registered
    When Cancel is enqueued and processed
    Then the run is removed from runs
    And the timer is removed from pending_timers
    When TimerFired is enqueued for the same run
    Then handle_timer returns RunNotFound
```

### BDD-003: Resume while timer pending
```
Feature: Resume with pending timer
  Scenario: Resume does not consume timer
    Given a run suspended on WaitUntil with timer registered
    And the deadline has not yet passed
    When Resume is enqueued and processed
    Then the timer remains registered
    And the run remains in pending_timers
    When TimerFired is enqueued
    Then the timer fire succeeds
    And the run advances or completes
```

### BDD-004: Resume after timer already fired
```
Feature: Resume after timer advance
  Scenario: Resume re-drives run from current PC
    Given a run suspended on WaitUntil
    When TimerFired is processed (advancing PC past WaitUntil)
    And the run is now suspended on a new step (or finished)
    When Resume is enqueued for the same run
    Then handle_resume calls drive_run from current PC
    And the run is not in pending_timers for the old timer
```

### BDD-005: Last-wins timer replacement
```
Feature: PendingTimer replacement
  Scenario: Second timer registration replaces first
    Given a run suspended on Ask with timer registered (step=3, kind=Ask)
    When Resume re-drives and hits Ask again
    And await_timer is called a second time
    Then pending_timers.len() == 1
    And the timer entry is updated to the new step
    And the old step is not recoverable
```

### BDD-006: Shutdown blocks resume
```
Feature: Shard shutdown and resume
  Scenario: Resume rejected when shard is shutting down
    Given a shard with shutting_down == true
    And a run suspended on WaitUntil
    When Resume is enqueued
    Then handle_resume returns ShutdownInProgress
    And the run is not driven
```

### BDD-007: Action completion removes timer
```
Feature: Action-timer interaction
  Scenario: handle_action_completion cleans timer before drive
    Given a run suspended on AwaitingAction with timer also registered
    When ActionCompletion arrives
    Then the timer is removed before drive_run
    And the run is driven to its next state
```

### BDD-008: Finish removes timer atomically
```
Feature: Run finish cleanup
  Scenario: finish_run removes timer before RunFinished journal entry
    Given a run with a pending timer
    When the run's frame reaches Finish
    And finish_run is called
    Then the timer is removed from pending_timers
    And then RunFinished is appended to the journal
    And the run is removed from the runs map
```

---

## 7. Error Path Matrix

| Scenario | Command | Expected Error | Condition |
|---|---|---|---|
| TimerFired on empty shard | TimerFired { run } | `RunNotFound` | run not in runs |
| TimerFired after cancel | TimerFired { run } | `RunNotFound` | run removed |
| TimerFired after finish | TimerFired { run } | `RunNotFound` | run removed |
| TimerFired, timer gone, run exists | TimerFired { run } | `InvalidTimerFire` | run in runs, timer absent |
| TimerFired, no timer, action-suspended | TimerFired { run } | `InvalidTimerFire` | AwaitingAction has no timer |
| Resume on empty shard | Resume { run } | `RunNotFound` | run not in runs |
| Resume during shutdown | Resume { run } | `ShutdownInProgress` | shutting_down == true |
| Submit when runs full | Submit { run } | `ActiveRunCapacityExceeded` | runs.len() >= max |
| Submit duplicate run | Submit { run } | `RunAlreadyExists` | run already in runs |

---

## 8. Test Execution Order

1. **Unit tests** — run in parallel via `cargo test -p vb_runtime -- timer_wheel --test-threads=8`
2. **Integration tests** — run with single-threaded shard simulation
3. **Proptest** — run with `--nocapture` to seeshrinking steps
4. **BDD scenarios** — run as doc-tests or separate binary with `--test-threads=1`

---

*Plan synthesized from contract vb-99n6. Unit tests cover TimerWheel dual-index consistency, integration tests cover handle_timer/resume/cancel command paths, proptest covers invariant preservation, BDD covers cross-component scenarios.*
