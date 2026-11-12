# Test Plan: vb-7gs9 — runtime: Shard scheduler bounded ownership evidence

## 1. Overview

This test plan covers the `Shard` type in `crates/vb_runtime/src/shard/`, specifically
validating bounded ownership, evidence chain integrity (Phase 40/44), run lifecycle,
frame pool management, and error taxonomy. All tests are organized under the Testing
Trophy distribution: unit tests (base), integration tests (middle), and property-based
tests (apex).

**Distribution budget (Testing Trophy):**
- Unit tests: 70%
- Integration tests: 20%
- Property-based tests (proptest): 10%

---

## 2. Unit Tests

Unit tests isolate individual `Shard` methods. Each test is a pure function of
input preconditions → expected postconditions, no I/O, no concurrency.

### 2.1 `ShardConfig::new` — Validation

| Test ID | Test Function | Precondition | Expected Postcondition |
|---------|---------------|--------------|------------------------|
| UT-001 | `fn config_new_accepts_min_valid_capacity()` | `command_queue_capacity=1`, `max_active_runs=1` | `Ok(ShardConfig)` with all fields preserved |
| UT-002 | `fn config_new_rejects_zero_capacity()` | `command_queue_capacity=0` | `Err(RuntimeError::CommandQueueCapacityExceeded { capacity: 0, max: 65536 })` |
| UT-003 | `fn config_new_rejects_capacity_exceeding_max()` | `command_queue_capacity=65537` | `Err(RuntimeError::CommandQueueCapacityExceeded { capacity: 65537, max: 65536 })` |
| UT-004 | `fn config_new_rejects_zero_max_active_runs()` | `max_active_runs=0` | `Err(RuntimeError::ActiveRunCapacityZero)` |
| UT-005 | `fn config_new_accepts_max_boundary_capacity()` | `command_queue_capacity=65536`, `max_active_runs=1` | `Ok(ShardConfig)` |
| UT-006 | `fn config_new_rejects_arbitrary_capacity_over_max()` | `command_queue_capacity=100_000` | `Err(RuntimeError::CommandQueueCapacityExceeded { capacity: 100_000, max: 65536 })` |

### 2.2 `Shard::new` — Construction

| Test ID | Test Function | Precondition | Expected Postcondition |
|---------|---------------|--------------|------------------------|
| UT-010 | `fn shard_new_creates_empty_shard()` | `ShardConfig { capacity: 4, max_active_runs: 2, ... }` | `runs.len()==0`, `pending_timers.len()==0`, `command_queue_len()==0`, `!is_shutting_down()` |
| UT-011 | `fn shard_new_sets_step_budget_per_tick()` | `ShardConfig { step_budget_per_tick: 7, ... }` | `step_budget_per_tick() == 7` |
| UT-012 | `fn shard_new_sets_max_active_runs()` | `ShardConfig { max_active_runs: 5, ... }` | `max_active_runs() == 5` |
| UT-013 | `fn shard_new_sets_policy()` | `ShardConfig { policy: Strict, ... }` | `policy() == Strict` |
| UT-014 | `fn shard_new_initializes_trace_ring()` | `ShardConfig { trace_capacity: 128, ... }` | `trace_ring.len() == 128` |

### 2.3 `Shard::enqueue` — Bounded Admission

| Test ID | Test Function | Precondition | Expected Postcondition |
|---------|---------------|--------------|------------------------|
| UT-020 | `fn enqueue_increments_queue_len()` | Empty queue, capacity 4, enqueue `Shutdown` | `command_queue_len() == 1` |
| UT-021 | `fn enqueue_decrements_remaining_capacity()` | Queue has 2 items, capacity 4, enqueue `Shutdown` | `remaining_capacity() == 1` |
| UT-022 | `fn enqueue_returns_ok_on_space_available()` | Queue with 3/4 items | `Ok(())` |
| UT-023 | `fn enqueue_returns_queue_full_at_capacity()` | Queue at capacity (2/2) | `Err(RuntimeError::QueueFull)`, queue unchanged |
| UT-024 | `fn enqueue_returns_queue_full_when_totally_full()` | Queue at capacity (4/4) | `Err(RuntimeError::QueueFull)`, queue unchanged |
| UT-025 | `fn enqueue_is_idempotent_on_full_queue()` | Queue full, two concurrent `enqueue` calls | Both return `Err(QueueFull)`, no double-count |
| UT-026 | `fn enqueue_rejects_shutdown_command_when_not_shutting_down()` | Fresh shard | `Ok(())` (allowed; shutdown flag set on tick) |
| UT-027 | `fn enqueue_allows_shutdown_command_always()` | Any shard state | `Ok(())` — only `tick` processes the command |

### 2.4 `Shard::tick` — Command Processing (FIFO)

| Test ID | Test Function | Precondition | Expected Postcondition |
|---------|---------------|--------------|------------------------|
| UT-030 | `fn tick_returns_true_on_empty_queue()` | Empty queue | `Ok(true)`, no state mutated |
| UT-031 | `fn tick_processes_shutdown_returns_false()` | Queue has `Shutdown` | `Ok(false)`, `is_shutting_down() == true` |
| UT-032 | `fn tick_after_shutdown_always_returns_false()` | Shard already shutting down | `Ok(false)` — permanent |
| UT-033 | `fn tick_processes_commands_in_fifo_order()` | Enqueue `Submit(R1)`, `Submit(R2)`, tick twice | R1 processed first, then R2 |
| UT-034 | `fn tick_processes_at_most_one_command()` | Queue has 2 commands, single tick | Only first command processed |
| UT-035 | `fn tick_idempotent_on_empty_queue()` | Empty queue, tick twice | Both return `Ok(true)`, no state change |

### 2.5 `Shard::tick` — Submit

| Test ID | Test Function | Precondition | Expected Postcondition |
|---------|---------------|--------------|------------------------|
| UT-040 | `fn tick_submit_increments_runs_submitted()` | `max_active_runs=2`, enqueue Submit workflow, tick | `counters().snapshot().runs_submitted == 1` |
| UT-041 | `fn tick_submit_finishes_synchronous_workflow()` | Enqueue Submit for zero-step workflow | `runs.len() == 0` after tick, `runs_completed == 1` |
| UT-042 | `fn tick_submit_returns_run_already_exists()` | Enqueue Submit twice for same `RunId`, tick both | First succeeds, second returns `Err(RunAlreadyExists)` |
| UT-043 | `fn tick_submit_returns_active_run_capacity_exceeded()` | `max_active_runs=1`, two Submit, tick both | First succeeds, second tick returns `Err(ActiveRunCapacityExceeded { capacity: 1 })` |
| UT-044 | `fn tick_submit_inserts_run_into_runs_map()` | Valid Submit, tick | `runs.get(&run) == Some(RunState)` |

### 2.6 `Shard::tick` — Resume

| Test ID | Test Function | Precondition | Expected Postcondition |
|---------|---------------|--------------|------------------------|
| UT-050 | `fn tick_resume_continues_suspended_run()` | Suspended run in `runs`, enqueue Resume, tick | `Ok(true)`, run still in `runs` |
| UT-051 | `fn tick_resume_returns_run_not_found()` | Enqueue Resume for unknown `RunId`, tick | `Err(RuntimeError::RunNotFound)` |

### 2.7 `Shard::tick` — ActionCompleted

| Test ID | Test Function | Precondition | Expected Postcondition |
|---------|---------------|--------------|------------------------|
| UT-060 | `fn tick_action_completed_returns_run_not_found()` | Enqueue ActionCompleted for unknown run, tick | `Err(RuntimeError::RunNotFound)` |
| UT-061 | `fn tick_action_completed_advances_frame()` | Suspended run, ActionCompleted advances step, tick | Frame step incremented |
| UT-062 | `fn tick_action_completed_emits_trace_event()` | Suspended run, ActionCompleted, tick | `TraceEvent::ActionCompleted { run, step }` in ring |

### 2.8 `Shard::tick` — ActionFailed

| Test ID | Test Function | Precondition | Expected Postcondition |
|---------|---------------|--------------|------------------------|
| UT-070 | `fn tick_action_failed_fails_run_without_handler()` | Suspended run (no error handler), ActionFailed, tick | `runs_failed == 1`, run removed from `runs` |
| UT-071 | `fn tick_action_failed_routes_to_error_handler()` | Suspended run with error handler, ActionFailed, tick | Run routes to handler step, `runs_failed` not incremented |
| UT-072 | `fn tick_action_failed_increments_failed_counter()` | Suspended run without handler, ActionFailed, tick | `counters().snapshot().runs_failed == 1` |

### 2.9 `Shard::tick` — TimerFired

| Test ID | Test Function | Precondition | Expected Postcondition |
|---------|---------------|--------------|------------------------|
| UT-080 | `fn tick_timer_fired_returns_run_not_found()` | Enqueue TimerFired for unknown run, tick | `Err(RuntimeError::RunNotFound)` |
| UT-081 | `fn tick_timer_fired_returns_invalid_timer_fire()` | Run with no pending timer, TimerFired, tick | `Err(RuntimeError::InvalidTimerFire)` |
| UT-082 | `fn tick_timer_fired_consumes_pending_timer()` | Wait workflow, pending timer, TimerFired, tick | `pending_timers.len() == 0` |
| UT-083 | `fn tick_timer_fired_advances_run()` | Wait workflow, timer fires, tick | Run advances, `runs_completed == 1` if done |

### 2.10 `Shard::tick` — Cancel

| Test ID | Test Function | Precondition | Expected Postcondition |
|---------|---------------|--------------|------------------------|
| UT-090 | `fn tick_cancel_removes_run_from_runs()` | Active run, Cancel, tick | `runs.len() == 0` |
| UT-091 | `fn tick_cancel_increments_runs_failed()` | Active run, Cancel, tick | `counters().snapshot().runs_failed == 1` |
| UT-092 | `fn tick_cancel_emits_run_cancelled_event()` | Active run, Cancel, tick | `TraceEvent::RunCancelled { run }` in ring |
| UT-093 | `fn tick_cancel_removes_pending_timer()` | Run with pending timer, Cancel, tick | `pending_timers.len() == 0` |
| UT-094 | `fn tick_cancel_returns_frame_to_pool()` | Run using frame `(step_count, slot_count)`, Cancel, tick | Frame pool `(step_count, slot_count).available` incremented |
| UT-095 | `fn tick_cancel_is_idempotent_for_unknown_run()` | Cancel for unknown run, tick | `Ok(true)`, no counter mutation |

### 2.11 `Shard::tick` — Inspect

| Test ID | Test Function | Precondition | Expected Postcondition |
|---------|---------------|--------------|------------------------|
| UT-100 | `fn tick_inspect_returns_found_for_active_run()` | Active run, Inspect, tick | `InspectResponse::Found(...)` |
| UT-101 | `fn tick_inspect_returns_not_found_for_missing_run()` | Inspect for unknown `RunId` | `InspectResponse::NotFound { run, correlation }` |
| UT-102 | `fn tick_inspect_does_not_mutate_run_state()` | Active run, Inspect, tick | Run state unchanged after tick |

### 2.12 `Shard::drain_for_shutdown`

| Test ID | Test Function | Precondition | Expected Postcondition |
|---------|---------------|--------------|------------------------|
| UT-110 | `fn drain_clears_pending_timers()` | Runs with pending timers, drain | `pending_timers.len() == 0` |
| UT-111 | `fn drain_sets_shutting_down_flag()` | Any state, drain | `is_shutting_down() == true` |
| UT-112 | `fn drain_returns_shutdown_in_progress_at_capacity_limit()` | Queue at capacity (2/2), no Shutdown command, drain | `Err(RuntimeError::ShutdownInProgress)` |
| UT-113 | `fn drain_returns_ok_when_shutdown_command_processed()` | Submit + Shutdown, drain | `Ok(())`, `shutting_down == true` |

### 2.13 `Shard::flush_evidence` — Evidence Chain (Phase 40/44)

| Test ID | Test Function | Precondition | Expected Postcondition |
|---------|---------------|--------------|------------------------|
| UT-120 | `fn flush_evidence_emits_step_started_before_slot_written()` | Collector with `StepStarted { step: 0 }` | Journal[0] is `StepStarted` |
| UT-121 | `fn flush_evidence_emits_step_succeeded_after_slot_written()` | Collector with full chain | Order: `StepStarted → SlotWritten → StepSucceeded` |
| UT-122 | `fn flush_evidence_drains_collector_completely()` | Collector with events | After flush, collector is empty |
| UT-123 | `fn flush_slot_written_encodes_with_postcard()` | `SlotWritten { slot, value }` | Encoded bytes decode back to original `SlotValue` |
| UT-124 | `fn flush_evidence_produces_one_slot_written_per_slot()` | Collector with one step's worth of slots | Exactly N `SlotWritten` events for N slots |
| UT-125 | `fn flush_evidence_produces_one_step_started_per_step()` | Collector with multiple steps | Exactly one `StepStarted` per step |

### 2.14 Frame Pool Management

| Test ID | Test Function | Precondition | Expected Postcondition |
|---------|---------------|--------------|------------------------|
| UT-130 | `fn take_frame_for_creates_pool_if_absent()` | `take_frame_for` for new dimension `(2, 1)` | Pool `(2, 1)` created |
| UT-131 | `fn take_frame_for_returns_frame_with_correct_dimensions()` | `take_frame_for` for workflow with `(step_count=3, slot_count=2)` | Frame has `step_count==3`, `slot_count==2` |
| UT-132 | `fn take_frame_for_reuses_existing_pool()` | Two `take_frame_for` calls for same dimension | Both return frames from same pool |
| UT-133 | `fn release_frame_returns_to_correct_pool()` | Acquired frame `(2, 1)`, released | Pool `(2, 1).available == 1` |
| UT-134 | `fn release_frame_increments_available_count()` | Pool has 0 available, frame released | Pool `available == 1` |
| UT-135 | `fn frame_pool_metrics_zero_initially()` | Fresh shard | All pools have `available == 0` |
| UT-136 | `fn release_frame_ignores_unknown_dimension()` | Release frame for dimension never `take_frame_for`'d | No panic, no state mutation |

### 2.15 Invariant Assertion Tests

| Test ID | Test Function | Invariant Verified |
|---------|---------------|-------------------|
| UT-140 | `fn invariant_i1_runs_len_never_exceeds_max_active_runs()` | `runs.len() <= max_active_runs` after all operations |
| UT-141 | `fn invariant_i2_queue_len_never_exceeds_capacity()` | `command_queue.len() <= command_queue_capacity` always |
| UT-142 | `fn invariant_i3_run_id_unique_in_runs()` | No duplicate `RunId` keys in `runs` map |
| UT-143 | `fn invariant_i4_run_id_unique_in_pending_timers()` | No duplicate `RunId` keys in `pending_timers` map |
| UT-144 | `fn invariant_i7_shutting_down_is_permanent()` | After `shutting_down==true`, all subsequent `tick()` return `Ok(false)` |

---

## 3. Integration Tests

Integration tests validate interactions between `Shard` and external dependencies:
bounded queue, frame pools, trace ring, and evidence collector. These tests use
real (not mocked) `ArrayQueue`, real `IndexMap` storage, and real evidence wiring.

### 3.1 Shard + Bounded Queue Interaction

| Test ID | Test Function | Scenario | Validates |
|---------|---------------|---------|-----------|
| IT-010 | `fn enqueue_dequeue_cycle_exhausts_capacity_then_refuses()` | Fill queue to capacity (3/3), attempt 4th enqueue, drain one, enqueue 4th | FIFO order preserved, no capacity violation |
| IT-011 | `fn tick_resumes_command_processing_after_drain()` | Fill queue, drain all, verify empty, enqueue new command, tick | Queue drains and refills correctly |
| IT-012 | `fn concurrent_enqueue_and_tick_stress()` | Rapidly alternate enqueue and tick in sequence | No capacity leak, no double-processing |
| IT-013 | `fn drain_for_shutdown_respects_capacity_limit()` | Queue at capacity, no Shutdown command present | `ShutdownInProgress` returned, no partial drain |

### 3.2 Shard + Frame Pool Interaction

| Test ID | Test Function | Scenario | Validates |
|---------|---------------|---------|-----------|
| IT-020 | `fn submit_and_cancel_cycles_frame_through_pool()` | Submit workflow, Cancel, verify pool available incremented | Frame returned to pool on cancel |
| IT-021 | `fn multiple_runs_same_dimension_share_pool()` | 3 concurrent runs with same `(step_count, slot_count)`, cancel all | Pool refills correctly for all 3 |
| IT-022 | `fn different_dimensions_create_separate_pools()` | Runs with `(1,1)` and `(2,1)` dimensions | Two separate pools exist |
| IT-023 | `fn submit_finishes_workflow_releases_frame()` | Submit sync workflow (finishes in one tick), verify frame returned | Frame pool available incremented |
| IT-024 | `fn submit_suspended_workflow_retains_frame_until_suspend`() | Submit suspended workflow, verify frame NOT released until suspend | Frame held while run active |

### 3.3 Shard + Evidence Chain Integration

| Test ID | Test Function | Scenario | Validates |
|---------|---------------|---------|-----------|
| IT-030 | `fn evidence_chain_step_started_before_slot_written_in_journal()` | Execute one step of a multi-step workflow | Journal order: StepStarted → SlotWritten → StepSucceeded |
| IT-031 | `fn evidence_flushed_before_tick_returns()` | Workflow with evidence events, tick | Evidence flushed to journal before `tick` returns |
| IT-032 | `fn multiple_steps_produce_ordered_evidence()` | 3-step workflow | Evidence for step 0 completes before step 1 begins |
| IT-033 | `fn cancelled_run_flushes_final_evidence()` | Submit workflow, cancel, flush evidence | Final `RunCancelled` evidence present |
| IT-034 | `fn failed_run_flushes_final_evidence()` | Submit workflow, trigger failure, flush | Final `ActionFailed` evidence present |

### 3.4 Shard + Timer Interaction

| Test ID | Test Function | Scenario | Validates |
|---------|---------------|---------|-----------|
| IT-040 | `fn wait_workflow_timer_fires_and_continues()` | Wait workflow, TimerFired, tick | Run advances, timer consumed |
| IT-041 | `fn cancel_clears_wait_timer()` | Wait workflow, Cancel | Timer cleared, frame returned |
| IT-042 | `fn multiple_pending_timers_are_tracked_separately()` | Two runs with pending timers, independent cancel | Each run's timer tracked independently |
| IT-043 | `fn ask_workflow_answer_clears_pending_timer()` | Ask workflow, answer, tick | Timer consumed, run continues |

### 3.5 Shard + Run Lifecycle Integration

| Test ID | Test Function | Scenario | Validates |
|---------|---------------|---------|-----------|
| IT-050 | `fn submit_resume_cancel_full_lifecycle()` | Submit suspended, Resume, Cancel | All state transitions correct |
| IT-051 | `fn submit_action_completed_action_failed_full_lifecycle()` | Submit suspended, ActionCompleted, ActionFailed | Run fails correctly |
| IT-052 | `fn duplicate_submit_rejected_at_tick_not_enqueue()` | Enqueue duplicate Submit, tick | `RunAlreadyExists` at tick, not enqueue |
| IT-053 | `fn capacity_limit_hit_at_tick_not_enqueue()` | max_active_runs=2, 3 submits, enqueue all, tick sequentially | `ActiveRunCapacityExceeded` at 3rd tick |
| IT-054 | `fn shutdown_flag_prevents_further_enqueue_processing()` | Shutdown, then Submit | Submit not processed, `shutting_down` remains true |

### 3.6 Shard + Counters Integration

| Test ID | Test Function | Scenario | Validates |
|---------|---------------|---------|-----------|
| IT-060 | `fn runs_submitted_incremented_on_submit()` | Submit 3 workflows, tick each | `runs_submitted == 3` |
| IT-061 | `fn runs_failed_incremented_on_cancel()` | Cancel 2 runs | `runs_failed == 2` |
| IT-062 | `fn runs_failed_incremented_on_action_failed_no_handler()` | ActionFailed on run without handler | `runs_failed == 1` |
| IT-063 | `fn runs_failed_not_incremented_on_handled_failure()` | ActionFailed on run with handler | `runs_failed == 0`, `runs_handled == 1` |
| IT-064 | `fn runs_completed_incremented_on_sync_finish()` | Submit 2 sync workflows | `runs_completed == 2` |

### 3.7 Shard + Inspect Integration

| Test ID | Test Function | Scenario | Validates |
|---------|---------------|---------|-----------|
| IT-070 | `fn inspect_returns_fresh_data_after_tick()` | Submit, tick, Inspect | Returns current run state, not stale |
| IT-071 | `fn status_reports_health_without_mutation()` | Multiple ticks, call `status()` | Queue depth unchanged after call |
| IT-072 | `fn inspect_after_cancel_returns_not_found()` | Submit, Cancel, Inspect | `NotFound` response |

---

## 4. Property-Based Tests (Proptest)

Proptest validates invariants across hundreds of randomly generated scenarios.
All strategies respect the bounded ownership constraints from the contract.

### 4.1 ShardConfig Strategy

```rust
fn shard_config_strategy() -> impl Strategy<Value = ShardConfig> {
    (1..=65536u32, 1..=1024u32, 1..=256u32, any::<ShardPolicy>())
        .prop_map(|(capacity, max_runs, budget, policy)| {
            ShardConfig::new(capacity, max_runs, budget, budget, policy)
        })
        .prop_filter("valid config", |config| config.is_ok())
        .prop_map(|config| config.unwrap())
}
```

### 4.2 ShardCommand Strategy

```rust
fn shard_command_strategy(run_ids: Vec<RunId>) -> impl Strategy<Value = ShardCommand> {
    prop_oneof![
        just(ShardCommand::Shutdown),
        run_ids.clone().prop_map(ShardCommand::Submit),
        run_ids.clone().prop_map(ShardCommand::Resume),
        run_ids.clone().prop_map(ShardCommand::Cancel),
        run_ids.clone().prop_map(ShardCommand::TimerFired),
        run_ids.clone().prop_map(ShardCommand::Inspect),
    ]
}
```

### 4.3 Invariant Property Tests

| Test ID | Property | Validates |
|---------|----------|-----------|
| PT-010 | `fn prop_runs_len_bounded_by_max_active_runs(ShardConfig, Vec<SubmitCommand>)` | After any sequence of enqueue+tick, `runs.len() <= max_active_runs` |
| PT-011 | `fn prop_queue_len_bounded_by_capacity(ShardConfig, Vec<ShardCommand>)` | After any sequence, `command_queue.len() <= command_queue_capacity` |
| PT-012 | `fn prop_shutdown_is_permanent(ShardConfig)` | After Shutdown command processed, all subsequent `tick()` return `Ok(false)` |
| PT-013 | `fn prop_pending_timers_unique_per_run(ShardConfig, Vec<TimerCommand>)` | No duplicate `RunId` keys in `pending_timers` |
| PT-014 | `fn prop_frame_pool_key_matches_workflow_dimensions(ShardConfig, Vec<SubmitCommand>)` | All frame pool keys derived from actual `node_count()` and `slot_count()` |
| PT-015 | `fn prop_take_frame_releases_preserve_pool_count(ShardConfig, Vec<SubmitCommand>)` | For every `take_frame_for`, a corresponding `release_frame` exists |
| PT-016 | `fn prop_duplicate_run_rejected_without_mutation(ShardConfig, RunId)` | Duplicate submit does not modify `runs` map |
| PT-017 | `fn prop_cancel_unknown_run_is_idempotent(ShardConfig, RunId)` | Cancel on unknown run produces no side effects |
| PT-018 | `fn prop_timer_fired_without_pending_returns_invalid_timer_fire(ShardConfig, RunId)` | TimerFired on run without timer returns `InvalidTimerFire` |

### 4.4 Evidence Chain Property Tests

| Test ID | Property | Validates |
|---------|----------|-----------|
| PT-020 | `fn prop_step_started_before_slot_written(EvidenceChain)` | E1: For every step, `StepStarted` appears before `SlotWritten` |
| PT-021 | `fn prop_step_succeeded_after_slot_written(EvidenceChain)` | E2: For every step, `StepSucceeded` appears after `SlotWritten` |
| PT-022 | `fn prop_exactly_one_slot_written_per_slot(EvidenceChain)` | E4: Exactly one `SlotWritten` per slot per step |
| PT-023 | `fn prop_exactly_one_step_started_per_step(EvidenceChain)` | E5: Exactly one `StepStarted` per step executed |
| PT-024 | `fn prop_evidence_flushed_before_tick_returns(ShardConfig, Vec<SubmitCommand>)` | E3: Evidence flushed before `tick()` returns |

### 4.5 Run Lifecycle Property Tests

| Test ID | Property | Validates |
|---------|----------|-----------|
| PT-030 | `fn prop_frame_in_pool_only_when_run_active(ShardConfig, Vec<RunLifecycleCommand>)` | L1: Frame in pool iff run is active |
| PT-031 | `fn prop_run_removal_returns_frame_to_pool(ShardConfig, Vec<SubmitCommand>)` | L2: Frame returned to pool on run removal |
| PT-032 | `fn prop_cancel_removes_pending_timer(ShardConfig, RunId)` | L3: Cancel removes timer entry |
| PT-033 | `fn prop_submit_duplicate_returns_run_already_exists(ShardConfig, RunId)` | L5: Duplicate submit returns error without mutation |
| PT-034 | `fn prop_queue_full_returns_immediately(ShardConfig, Vec<ShardCommand>)` | Q1: `enqueue` never blocks; returns `QueueFull` or `Ok` |

### 4.6 Error Path Property Tests

| Test ID | Property | Validates |
|---------|----------|-----------|
| PT-040 | `fn prop_enqueue_queue_full_does_not_mutate_queue(ShardConfig, Vec<ShardCommand>)` | When `enqueue` returns `QueueFull`, queue state is unchanged |
| PT-041 | `fn prop_run_not_found_does_not_mutate_runs(ShardConfig, RunId)` | When `Resume/ActionCompleted/ActionFailed/TimerFired` returns `RunNotFound`, `runs` unchanged |
| PT-042 | `fn prop_capacity_exceeded_does_not_fill_runs(ShardConfig, Vec<SubmitCommand>)` | When `ActiveRunCapacityExceeded` returned, `runs` map has exactly `max_active_runs` entries |

---

## 5. BDD Scenarios

Given-When-Then format. Each scenario covers one complete behavior slice.

### 5.1 Shard Initialization

```
Scenario: Shard initializes with empty state and correct configuration
  Given a valid ShardConfig with command_queue_capacity=4, max_active_runs=2, step_budget=8, policy=Relaxed
  When I construct a new Shard with that config
  Then runs is empty
  And pending_timers is empty
  And command_queue has capacity 4
  And is_shutting_down() is false
  And max_active_runs() returns 2
  And step_budget_per_tick() returns 8
```

```
Scenario: ShardConfig rejects invalid capacity at construction
  Given command_queue_capacity=0
  When I call ShardConfig::new(0, 1, 1, 1, Relaxed)
  Then the result is Err(CommandQueueCapacityExceeded { capacity: 0, max: 65536 })
  And no Shard can be constructed with this config
```

### 5.2 Command Queue Admission Control

```
Scenario: Enqueue adds command and decrements remaining capacity
  Given a fresh Shard with queue capacity 4
  When I enqueue Shutdown
  Then command_queue_len() equals 1
  And remaining_capacity() equals 3
```

```
Scenario: Enqueue rejects when queue is at capacity
  Given a Shard with queue capacity 2, already holding 2 commands
  When I attempt to enqueue a third command
  Then the result is Err(QueueFull)
  And command_queue_len() remains 2
```

### 5.3 Shutdown Lifecycle

```
Scenario: Shutdown command sets permanent shutting_down flag
  Given a fresh Shard
  When I enqueue Shutdown and call tick()
  Then tick returns Ok(false)
  And is_shutting_down() is true
```

```
Scenario: Tick returns false permanently after shutdown
  Given a Shard that has processed Shutdown
  When I call tick() a second time
  Then the result is Ok(false)
  And subsequent tick() calls continue to return Ok(false)
```

```
Scenario: drain_for_shutdown processes all pending commands
  Given a Shard with Submit(R1) and Shutdown queued
  When I call drain_for_shutdown()
  Then the result is Ok(())
  And pending_timers is cleared
  And is_shutting_down() is true
```

```
Scenario: drain_for_shutdown fails when capacity limit is hit
  Given a Shard with queue at capacity (2/2) and no Shutdown command
  When I call drain_for_shutdown()
  Then the result is Err(ShutdownInProgress)
  And pending_timers is NOT cleared
```

### 5.4 Run Submission and Bounded Active Runs

```
Scenario: Submit adds run to runs map under capacity
  Given a Shard with max_active_runs=2 and empty runs
  When I enqueue Submit(R1, workflow) and call tick()
  Then runs.get(R1) returns Some(RunState)
  And counters().runs_submitted equals 1
```

```
Scenario: Submit returns RunAlreadyExists for duplicate RunId
  Given a Shard with R1 already in runs
  When I enqueue Submit(R1, workflow) and call tick()
  Then the result is Err(RunAlreadyExists)
  And runs.len() remains 1
```

```
Scenario: Submit returns ActiveRunCapacityExceeded when at limit
  Given a Shard with max_active_runs=1 and R1 in runs
  When I enqueue Submit(R2, workflow) and call tick()
  Then the result is Err(ActiveRunCapacityExceeded { capacity: 1 })
  And runs still contains only R1
```

### 5.5 Run Cancellation

```
Scenario: Cancel removes run and increments failed counter
  Given a Shard with an active run R1
  When I enqueue Cancel(R1) and call tick()
  Then runs does not contain R1
  And counters().runs_failed equals 1
```

```
Scenario: Cancel emits RunCancelled trace event
  Given a Shard with an active run R1
  When I enqueue Cancel(R1) and call tick()
  Then TraceEvent::RunCancelled { run: R1 } is in the trace ring
```

```
Scenario: Cancel returns frame to dimension pool
  Given a Shard with an active run using frame (step_count=2, slot_count=1)
  When I enqueue Cancel(R1) and call tick()
  Then frame_pool(2,1).available equals 1
```

```
Scenario: Cancel clears pending timer if present
  Given a Shard with an active run R1 that has a pending timer
  When I enqueue Cancel(R1) and call tick()
  Then pending_timers does not contain R1
```

```
Scenario: Cancel is idempotent for unknown run
  Given a Shard (with or without active runs)
  When I enqueue Cancel(R_unknown) and call tick()
  Then the result is Ok(true)
  And counters are unchanged
```

### 5.6 TimerFired

```
Scenario: TimerFired advances waiting run
  Given a Shard with a run R1 in wait state and a pending timer
  When I enqueue TimerFired(R1) and call tick()
  Then pending_timers does not contain R1
  And the run advances or completes
```

```
Scenario: TimerFired returns RunNotFound for unknown run
  Given a Shard (no specific R1 in runs)
  When I enqueue TimerFired(R_unknown) and call tick()
  Then the result is Err(RunNotFound)
```

```
Scenario: TimerFired returns InvalidTimerFire when no timer pending
  Given a Shard with an active run R1 that has no pending timer
  When I enqueue TimerFired(R1) and call tick()
  Then the result is Err(InvalidTimerFire)
```

### 5.7 Frame Pool

```
Scenario: take_frame_for creates dimension pool on first use
  Given a fresh Shard
  When I call take_frame_for with workflow (step_count=2, slot_count=1)
  Then frame_pool(2,1) exists
  And the returned frame has step_count=2 and slot_count=1
```

```
Scenario: release_frame returns frame to correct pool
  Given a Shard where take_frame_for was called with (step_count=3, slot_count=2)
  When I call release_frame with that frame
  Then frame_pool(3,2).available equals 1
```

```
Scenario: Multiple runs share dimension pool
  Given a Shard with max_active_runs=3
  When I submit 3 workflows all with dimension (1,1)
  Then frame_pool(1,1) exists
  And all 3 frames are allocated from the same pool
```

### 5.8 Evidence Chain (Phase 40/44)

```
Scenario: Single step produces correct evidence order
  Given a Shard with a workflow that has 1 step
  When I execute that step via tick
  And I call flush_evidence
  Then the journal contains StepStarted before SlotWritten before StepSucceeded
```

```
Scenario: Multiple steps produce sequential evidence chains
  Given a Shard with a 3-step workflow
  When I execute all 3 steps via tick calls
  And I call flush_evidence after each tick
  Then step 0 evidence appears before step 1 evidence
  And step 1 evidence appears before step 2 evidence
```

```
Scenario: Cancelled run flushes final RunCancelled evidence
  Given a Shard with an active run
  When I cancel the run and call flush_evidence
  Then the journal contains RunCancelled event
```

### 5.9 Inspect

```
Scenario: Inspect returns Found for active run
  Given a Shard with an active run R1
  When I enqueue Inspect(R1, correlation=42) and call tick()
  Then the result is InspectResponse::Found { run: R1, ... }
```

```
Scenario: Inspect returns NotFound for missing run
  Given a Shard (with or without runs)
  When I enqueue Inspect(R_unknown, 99) and call tick()
  Then the result is InspectResponse::NotFound { run: R_unknown, correlation: 99 }
```

### 5.10 Error Handler Routing

```
Scenario: ActionFailed without error handler fails the run
  Given a Shard with an active run R1 (no error handler)
  When I enqueue ActionFailed(R1) and call tick()
  Then runs does not contain R1
  And counters().runs_failed equals 1
```

```
Scenario: ActionFailed with error handler routes to handler
  Given a Shard with an active run R1 that has an error handler
  When I enqueue ActionFailed(R1) and call tick()
  Then R1 is still in runs (at handler step)
  And counters().runs_failed equals 0
```

---

## 6. Test Execution Order

Within a test binary, tests run in this priority order:

1. **Unit tests** (`#[test]`) — run in random order, fully isolated
2. **Integration tests** (`#[test]`, `tests/integration/` or `mod integration`) — run after all unit tests pass
3. **Property tests** (`#[proptest]`) — run last, with 10_000 iterations each

The test binary must pass all three tiers before the bead is considered verified.

---

## 7. Test Statistics Summary

| Category | Count | Trophy Layer |
|----------|-------|-------------|
| Unit tests (UT-*) | 55 | Base |
| Integration tests (IT-*) | 30 | Middle |
| Property tests (PT-*) | 18 | Apex |
| BDD scenarios | 16 | All layers |
| **Total** | **119** | |

---

## 8. Test Naming Convention

All test functions follow the pattern:

```
<layer>_<invariant_or_behavior>_<expected_outcome>

layer:  ut_ (unit), it_ (integration), pt_ (property)
```

Examples:
- `ut_shard_new_creates_empty_shard`
- `it_submit_returns_active_run_capacity_exceeded`
- `pt_runs_len_bounded_by_max_active_runs`
- `fn scenario_shutdown_is_permanent`
