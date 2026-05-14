# Test Plan: vb-6azo — Behavioral Property Tests for Workflow Engine Invariants

## 1. Overview

This test plan covers behavioral property tests for the Velvet-ballistics runtime engine, verifying critical invariants across `drive_deterministic_full`, `EvidenceCollector`, `FramePool`, `Shard.tick()`, and `mark_step_after_signal` under both normal and adversarial inputs.

**Bead:** vb-6azo
**Test file:** `crates/vb_runtime/src/engine/property_tests.rs`
**Strategy framework:** proptest 1.x

---

## 2. Test Categories

### 2.1 Category A: Evidence Chain Invariant Tests (INV(E1)–INV(E4))

#### A.1 `evidence_chain_ordering_preserved`
```
Signature: fn evidence_chain_ordering_preserved(workflow: CompiledWorkflow, budget: StepBudget)
Strategy: workflow in 1..50 steps, budget in 1..1000
Verifies: INV(E1), INV(E2), INV(E3)

Invariant checks:
- For every StepSucceeded(step), a StepStarted(step) appears earlier in evidence
- For every SlotWritten(slot, step, _), StepStarted(step) appears before it
- No StepSucceeded emitted for steps returning Awaiting* signals
```
**Edge cases:** Empty workflows (1 step), deeply nested branches, workflows with no branching.

#### A.2 `evidence_drain_resets_dropped_counter`
```
Signature: fn evidence_drain_resets_dropped_counter(capacity: usize, event_count: usize)
Strategy: capacity in 0..=100, event_count in 0..=500
Verifies: INV(E4)

Invariant checks:
- After any number of pushes exceeding capacity, drain() returns exactly len() events
- After drain(), len() == 0 and dropped() == 0
```
**Edge cases:** capacity=0, capacity=1, event_count=0, event_count << capacity, event_count >> capacity.

#### A.3 `evidence_collector_bounded_collection`
```
Signature: fn evidence_collector_bounded_collection(capacity: usize, pushes: usize)
Strategy: capacity in 1..=100, pushes in 0..=1000
Verifies: INV(E4), Postcondition Bounded Collection

Invariant checks:
- After n pushes where n > capacity: len() == capacity, dropped() == n - capacity (saturating)
- After n pushes where n <= capacity: len() == n, dropped() == 0
- capacity() returns the value passed to constructor
```
**Edge cases:** Saturating arithmetic at u64::MAX.

---

### 2.2 Category B: Budget Invariant Tests (INV(B1)–INV(B3))

#### B.1 `budget_exhaustion_stops_at_exact_boundary`
```
Signature: fn budget_exhaustion_stops_at_exact_boundary(workflow_step_count: u16, budget_value: u64)
Strategy: workflow_step_count in 1..=1000, budget_value in 0..=10_000
Verifies: INV(B1), INV(B2)

Invariant checks:
- When budget.try_take() returns false, drive_deterministic_full returns StepBudgetExhausted
- No node executes after budget exhaustion
- PC is exactly at budget_value after exhaustion
```
**Edge cases:** budget=0 (immediate exhaustion), budget=1, budget >> step_count, budget == step_count.

#### B.2 `zero_budget_means_no_execution`
```
Signature: fn zero_budget_means_no_execution(workflow: CompiledWorkflow)
Strategy: workflow in 1..20 steps
Verifies: INV(B2)

Invariant checks:
- StepBudget::new(0).try_take() returns Ok(false) on first call
- drive_deterministic_full with budget=0 returns StepBudgetExhausted without executing any node
```
**Error path:** No node dispatched, evidence contains no StepStarted events.

#### B.3 `budget_decrement_is_unit`
```
Signature: fn budget_decrement_is_unit(initial_budget: u64)
Strategy: initial_budget in 1..=10_000
Verifies: INV(B3)

Invariant checks:
- After N successful try_take() calls, remaining budget == initial_budget - N
- Each try_take() returns Ok(true) until budget reaches 0
```
**Edge cases:** Boundary at 1, large budgets.

---

### 2.3 Category C: Frame Pool Invariant Tests (INV(F1)–INV(F3))

#### C.1 `frame_pool_capacity_never_exceeded`
```
Signature: fn frame_pool_capacity_never_exceeded(step_count: u16, slot_count: u8, capacity: u16, releases: usize)
Strategy: step_count in 1..=16, slot_count in 0..=16, capacity in 1..=100, releases in 0..=500
Verifies: INV(F1)

Invariant checks:
- After any sequence of take() and release() calls, available() <= capacity
- Silent drops at capacity boundary do not cause available() to exceed capacity
```
**Edge cases:** Concurrent-style sequential stress (rapid take/release cycles), capacity=1.

#### C.2 `frame_pool_dimension_mismatch_silent_drop`
```
Signature: fn frame_pool_dimension_mismatch_silent_drop(pool_s1: u16, pool_c1: u8, pool_cap: u16, frame_s2: u16, frame_c2: u8)
Strategy: (pool_s1, pool_c1) != (frame_s2, frame_c2) in at least one dimension
Verifies: INV(F2)

Invariant checks:
- release(frame) to pool with mismatched (step_count, slot_count) leaves available() unchanged
- No error returned for dimension mismatch
```
**Error path:** Dimension mismatch silently drops frame without panic or error.

#### C.3 `frame_reuse_clears_all_prior_state`
```
Signature: fn frame_reuse_clears_all_prior_state(step_count: u16, slot_count: u8)
Strategy: step_count in 1..=16, slot_count in 0..=16
Verifies: INV(F3)

Invariant checks:
- After take() returning a recycled frame:
  - frame.executed() == 0
  - frame.pc() == first_step argument
  - frame.run_id() == run_id argument
  - All slot reads return SlotUninitialized
  - All taint reads return SlotUninitialized
  - All StepState entries are Pending
```
**Edge cases:** slot_count=0, step_count=1.

#### C.4 `frame_reuse_produces_clean_frame_across_pool_cycles`
```
Signature: fn frame_reuse_produces_clean_frame_across_pool_cycles(step_count: u16, slot_count: u8, cycles: u8)
Strategy: step_count in 1..=8, slot_count in 0..=8, cycles in 1..=10
Verifies: INV(F3) repeated across multiple recycle cycles

Invariant checks:
- After N take/release cycles, each freshly returned frame is clean
- SlotWritten on a prior frame does not leak into subsequent frame
```
**Error path:** State leakage between cycles.

---

### 2.4 Category D: Shard Invariant Tests (INV(S1)–INV(S4))

#### D.1 `command_queue_full_boundary`
```
Signature: fn command_queue_full_boundary(capacity: u8, enqueue_count: usize)
Strategy: capacity in 1..=64, enqueue_count in 0..=200
Verifies: INV(S1)

Invariant checks:
- is_queue_full() == (len() == capacity)
- Enqueue beyond capacity returns Err(RuntimeError::QueueFull)
- remaining_capacity() == capacity - len()
```
**Edge cases:** capacity=1, capacity=64, enqueue_count == capacity, enqueue_count >> capacity.

#### D.2 `one_command_per_tick_enforced`
```
Signature: fn one_command_per_tick_enforced(commands: Vec<ShardCommand>, tick_count: usize)
Strategy: commands in 1..=20, tick_count in 1..=50
Verifies: INV(S2)

Invariant checks:
- After N tick() calls, at most N commands have been processed
- Empty queue returns Ok(true) without mutation
```
**Edge cases:** Empty command list, tick_count >> command count.

#### D.3 `shutdown_terminates_tick_loop`
```
Signature: fn shutdown_terminates_tick_loop(pre_shutdown_commands: Vec<ShardCommand>)
Strategy: pre_shutdown_commands in 0..=10
Verifies: INV(S3)

Invariant checks:
- After Shutdown command, tick() returns Ok(false)
- Subsequent tick() calls continue returning Ok(false)
- Shard status transitions to shutting_down
```
**Error path:** Commands enqueued after Shutdown are never processed.

#### D.4 `run_lifecycle_submit_cancel_exclusivity`
```
Signature: fn run_lifecycle_submit_cancel_exclusivity(submits: usize, cancels: usize)
Strategy: submits in 0..=10, cancels in 0..=10
Verifies: INV(S4)

Invariant checks:
- A RunId appears in self.runs at most once at any time
- After Cancel, run is not in self.runs
- After Submit, run is in self.runs with Pending status
```
**Edge cases:** Cancel before Submit (no-op), double Cancel (no-op), Submit same RunId twice.

#### D.5 `shard_rejects_submit_when_at_capacity`
```
Signature: fn shard_rejects_submit_when_at_capacity(max_active_runs: u8, submit_count: usize)
Strategy: max_active_runs in 1..=8, submit_count in 0..=20
Verifies: RuntimeError::ActiveRunCapacityExceeded

Invariant checks:
- First max_active_runs Submit commands succeed
- Subsequent Submit returns Err(RuntimeError::ActiveRunCapacityExceeded)
- is_at_capacity() == (active_runs == max_active_runs)
```
**Error path:** ActiveRunCapacityExceeded on over-submit.

---

### 2.5 Category E: Step State Machine Tests (INV(M1)–INV(M2))

#### E.1 `step_state_transition_validity`
```
Signature: fn step_state_transition_validity(initial_state: StepState, signal: RuntimeSignal, expected_state: StepState)
Strategy: initial_state in {Pending, Running, Waiting, Asking, Succeeded}, signal exhaustive
Verifies: INV(M1)

Invariant checks:
| Signal            | From     | To       | Valid? |
|-------------------|----------|----------|--------|
| AwaitingWait      | Running  | Waiting  | Yes    |
| AwaitingAsk       | Running  | Asking   | Yes    |
| AwaitingAction(_) | Running  | Running  | Yes    |
| StepBudgetExhausted| Running | Running  | Yes    |
| Continue          | Running  | Succeeded| Yes    |
| Finished(_)       | Running  | Succeeded| Yes    |
| Continue          | Waiting  | —        | Err    |
| Continue          | Asking   | —        | Err    |
| Finished(_)       | Waiting  | —        | Err    |
```
**Edge cases:** All invalid signal/state combinations return EngineError::InternalInvariantViolation.

#### E.2 `mark_step_rejects_invalid_state_transitions`
```
Signature: fn mark_step_rejects_invalid_state_transitions(state: StepState, signal: RuntimeSignal)
Strategy: state in {Pending, Waiting, Asking, Succeeded}, signal in {Continue, Finished(_)}
Verifies: INV(M2)

Invariant checks:
- mark_step_after_signal with non-Running state and Continue/Finished returns Err(EngineError::InternalInvariantViolation)
- Only Running state accepts Continue or Finished
```
**Error path:** InternalInvariantViolation on invalid transition.

---

### 2.6 Category F: Drive Loop Integration Tests

#### F.1 `drive_finishes_with_correct_result`
```
Signature: fn drive_finishes_with_correct_result(slot_value: SlotValue)
Strategy: slot_value in {I64(-1000..=1000), Bool, Null, Symbol}
Verifies: Postcondition (State Transition) — Finished signal

Invariant checks:
- Given SetConst(slot=0, const=value) → Finish(result=0)
- drive_deterministic_full returns RuntimeSignal::Finished(SlotValue::I64(value))
```
**Happy path:** End-to-end workflow execution with correct result.

#### F.2 `drive_awaiting_action_signal_mapping`
```
Signature: fn drive_awaiting_action_signal_mapping(ticket: ActionTicket)
Strategy: ticket constructed with valid run_id and action_name
Verifies: Postcondition (State Transition) — AwaitingAction

Invariant checks:
- When current node is a Do node, drive returns AwaitingAction(ticket) with ticket.run == run.run_id()
```
**Happy path:** Action dispatch suspension.

#### F.3 `drive_awaiting_wait_signal_mapping`
```
Signature: fn drive_awaiting_wait_signal_mapping(deadline_ns: u64)
Strategy: deadline_ns in 0..=u64::MAX
Verifies: Postcondition (State Transition) — AwaitingWait

Invariant checks:
- When current node is WaitUntil, drive returns AwaitingWait
- Deadline comparison is correct (deadline not elapsed)
```
**Happy path:** Wait suspension.

#### F.4 `drive_awaiting_ask_signal_mapping`
```
Signature: fn drive_awaiting_ask_signal_mapping(prompt: String)
Strategy: prompt in non-empty strings up to 256 chars
Verifies: Postcondition (State Transition) — AwaitingAsk

Invariant checks:
- When current node is Ask, drive returns AwaitingAsk
- Ask node fields are preserved correctly
```
**Happy path:** Ask suspension.

#### F.5 `drive_budget_exhausted_returns_correct_signal`
```
Signature: fn drive_budget_exhausted_returns_correct_signal(workflow: CompiledWorkflow, budget: u64)
Strategy: budget < workflow.node_count()
Verifies: Postcondition (State Transition) — StepBudgetExhausted

Invariant checks:
- When budget.try_take() returns false, drive returns StepBudgetExhausted
- budget.try_take() was called at least once
```
**Error path:** Budget exhaustion propagation.

#### F.6 `drive_pc_always_in_bounds`
```
Signature: fn drive_pc_always_in_bounds(workflow: CompiledWorkflow, budget: u64)
Strategy: workflow in 1..=100 steps, budget in 0..=200
Verifies: INV(B1) / PC Invariant

Invariant checks:
- At every point in drive, run.pc() is a valid StepIdx within [0, plan.node_count())
- No unchecked indexing on plan.nodes
```
**Error path:** PC out-of-bounds detection.

---

### 2.7 Category G: Error Taxonomy Tests

#### G.1 `runtime_engine_errors_return_correct_variants`
```
Signature: fn runtime_engine_errors_return_correct_variants(error_input: ErrorScenario)
Strategy: ErrorScenario in {StackOverflow, Underflow, SlotOutOfBounds, DivByZero, NonFinite}
Verifies: Error Taxonomy mapping (Section 4.1)

Invariant checks:
| Condition                          | Error Variant                              | Signal |
|------------------------------------|--------------------------------------------|--------|
| plan.node(pc) returns None         | EngineError::InvalidProgramCounter        | Err    |
| Stack exceeds max_stack            | EngineError::ExpressionStackOverflow      | Err    |
| Stack pop on empty                 | EngineError::ExpressionStackUnderflow      | Err    |
| Slot index >= slot_count           | EngineError::SlotOutOfBounds              | Err    |
| Division by zero                   | EngineError::DivisionByZero               | Err    |
| NaN/Inf in finite context          | EngineError::NonFiniteNumber              | Err    |
```
**Error path:** All error variants map to correct RuntimeEngineError.

#### G.2 `branch_limit_exceeded_error`
```
Signature: fn branch_limit_exceeded_error(branch_count: u32)
Strategy: branch_count in (u16::MAX as u32 + 1)..=(u16::MAX as u32 * 2)
Verifies: RuntimeEngineError::BranchLimitExceeded

Invariant checks:
- compute_max_parallel_in_flight with > u16::MAX branches returns Err(BranchLimitExceeded { max: u16::MAX, requested: actual })
```
**Error path:** BranchLimitExceeded on overflow.

#### G.3 `frame_pool_error_variants`
```
Signature: fn frame_pool_error_variants(scenario: FramePoolErrorScenario)
Strategy: scenario in {CapacityZero, Capacity4097, StepCountZero}
Verifies: Frame Pool Error Taxonomy (Section 4.3)

Invariant checks:
| Scenario           | Error Variant                                  |
|--------------------|------------------------------------------------|
| capacity == 0      | CoreError::ResourceLimitExceeded(frame_pool)  |
| capacity > 4096    | CoreError::ResourceLimitExceeded(frame_pool)  |
| step_count == 0    | CoreError::InvalidCompiledWorkflow(step_count) |
```
**Error path:** Proper error construction.

---

### 2.8 Category H: Adversarial / Invariant Falsification Tests

#### H.1 `adversarial_workflow_stress`
```
Signature: fn adversarial_workflow_stress(workflow: CompiledWorkflow, budget: StepBudget)
Strategy: workflow in 1..=1000 steps, budget in 0..=10_000
Verifies: All invariants under extreme inputs

Invariant checks:
- Evidence chain remains ordered
- Budget enforcement holds
- PC remains in bounds
- Frame pool invariants hold
```
**Adversarial:** Large step counts, extreme budget ratios, all edge cases.

#### H.2 `adversarial_evidence_flood`
```
Signature: fn adversarial_evidence_flood(capacity: usize, flood_count: usize)
Strategy: capacity in 1..=10, flood_count in 10_000..=100_000
Verifies: INV(F1), EvidenceCollector boundedness under extreme load

Invariant checks:
- dropped() counter saturates correctly
- len() never exceeds capacity
- drain performance is acceptable
```
**Adversarial:** Massive event floods, tiny capacity.

#### H.3 `adversarial_frame_take_release_cycles`
```
Signature: fn adversarial_frame_take_release_cycles(step_count: u16, slot_count: u8, capacity: u16, cycles: usize)
Strategy: cycles in 10_000..=100_000
Verifies: INV(F1), INV(F3) under high iteration count

Invariant checks:
- Capacity never exceeded across all cycles
- Each recycled frame remains clean
```
**Adversarial:** High-frequency take/release cycles.

---

## 3. BDD Given-When-Then Scenarios

### 3.1 Evidence Chain Scenarios

**Scenario E1: Evidence ordering is preserved across workflow execution**
- GIVEN a valid `CompiledWorkflow` with multiple steps
- WHEN `drive_deterministic_full` executes to completion
- THEN every `StepSucceeded(step)` has a preceding `StepStarted(step)`
- AND every `SlotWritten(slot, step, _)` has a preceding `StepStarted(step)`
- AND no `StepSucceeded` appears for steps returning `Awaiting*` signals

**Scenario E2: Evidence collector drains cleanly**
- GIVEN an `EvidenceCollector` with capacity 5 containing 10 events
- WHEN `drain()` is called
- THEN `drain()` returns exactly 5 events
- AND `len() == 0` after drain
- AND `dropped() == 0` after drain

### 3.2 Budget Scenarios

**Scenario B1: Budget exhaustion halts execution immediately**
- GIVEN a workflow with 100 steps and `StepBudget::new(50)`
- WHEN `drive_deterministic_full` runs until budget exhaustion
- THEN exactly 50 steps execute (50 `StepStarted` events)
- AND `RuntimeSignal::StepBudgetExhausted` is returned
- AND PC is at step index 50

**Scenario B2: Zero budget executes no steps**
- GIVEN any `CompiledWorkflow` with at least 1 step
- WHEN `drive_deterministic_full` is called with `StepBudget::new(0)`
- THEN `StepBudget::new(0).try_take()` returns `Ok(false)`
- AND `drive_deterministic_full` returns `StepBudgetExhausted` immediately
- AND evidence contains zero `StepStarted` events

### 3.3 Frame Pool Scenarios

**Scenario F1: Capacity is never exceeded**
- GIVEN a `FramePool` with `capacity = 3`
- WHEN 3 frames are taken
- AND 3 frames are released
- AND 3 more frames are taken
- THEN `available()` never exceeds 3 at any point

**Scenario F2: Dimension mismatch silently drops frame**
- GIVEN a `FramePool` with `step_count=4, slot_count=2, capacity=3`
- WHEN a frame with `step_count=2, slot_count=1` is released
- THEN `available()` remains unchanged
- AND no error is returned

**Scenario F3: Reused frame has clean state**
- GIVEN a `FramePool` with `step_count=4, slot_count=2`
- WHEN a frame is taken, used to write to slots, and released
- AND a second frame is taken from the same pool
- THEN the second frame has `executed() == 0`
- AND all slot reads return `SlotUninitialized`
- AND all taint reads return `SlotUninitialized`

### 3.4 Shard Scenarios

**Scenario S1: Command queue honors capacity**
- GIVEN a `Shard` with `command_queue_capacity = 4`
- WHEN 5 `Submit` commands are enqueued
- THEN the first 4 succeed
- AND the 5th returns `Err(RuntimeError::QueueFull)`
- AND `is_queue_full()` returns `true` after 4 enqueues

**Scenario S2: Shutdown terminates tick processing**
- GIVEN a `Shard` with pending commands in queue
- WHEN a `Shutdown` command is enqueued and processed
- THEN subsequent `tick()` calls return `Ok(false)`
- AND no further commands are processed

**Scenario S3: Run exclusivity is enforced**
- GIVEN a `Shard` with `max_active_runs = 2`
- WHEN 2 `Submit` commands are processed
- AND a 3rd `Submit` is attempted
- THEN the 3rd returns `Err(RuntimeError::ActiveRunCapacityExceeded)`
- AND after `Cancel` of one run, a new `Submit` succeeds

### 3.5 Step State Machine Scenarios

**Scenario M1: Valid transitions are accepted**
- GIVEN a `RunFrame` with a step in `Running` state
- WHEN `mark_step_after_signal` is called with `AwaitingWait`
- THEN the step transitions to `Waiting` state

**Scenario M2: Invalid transitions are rejected**
- GIVEN a `RunFrame` with a step in `Waiting` state
- WHEN `mark_step_after_signal` is called with `Continue`
- THEN `Err(EngineError::InternalInvariantViolation)` is returned

---

## 4. Test Execution Order

### Phase 1: Unit Property Tests (Categories A, B, C, E)
Execute independently; no cross-dependencies.

### Phase 2: Component Integration Tests (Category D, F)
Shard tests require FramePool and RunFrame; drive tests require EvidenceCollector and StepBudget.

### Phase 3: Error Taxonomy Tests (Category G)
Independent; verify error variant mapping.

### Phase 4: Adversarial Falsification (Category H)
Run last with highest iteration counts (10,000+).

---

## 5. Acceptance Criteria Mapping

| Criterion | Test Coverage |
|-----------|---------------|
| All 14 property tests compile | A.1–E.2 (14 tests) |
| clippy passes with no warnings | All tests |
| proptest ≥1000 iterations | All proptest functions |
| Miri clean on pure engine tests | A.1, A.2, B.1–B.3, C.1–C.4, E.1, E.2 |
| Invariant falsification confirms hold | H.1, H.2, H.3 |
| No unsafe/unwrap/expect/panic/todo/dbg | All tests |
| snake_case test names | All tests |

---

## 6. Test Function Summary

| ID | Function Name | Category | Invariants | proptest? |
|----|-------------|----------|------------|-----------|
| 1 | `evidence_chain_ordering_preserved` | A | E1,E2,E3 | Yes |
| 2 | `evidence_drain_resets_dropped_counter` | A | E4 | Yes |
| 3 | `evidence_collector_bounded_collection` | A | E4 | Yes |
| 4 | `budget_exhaustion_stops_at_exact_boundary` | B | B1,B2 | Yes |
| 5 | `zero_budget_means_no_execution` | B | B2 | Yes |
| 6 | `budget_decrement_is_unit` | B | B3 | Yes |
| 7 | `frame_pool_capacity_never_exceeded` | C | F1 | Yes |
| 8 | `frame_pool_dimension_mismatch_silent_drop` | C | F2 | Yes |
| 9 | `frame_reuse_clears_all_prior_state` | C | F3 | Yes |
| 10 | `frame_reuse_produces_clean_frame_across_pool_cycles` | C | F3 | Yes |
| 11 | `command_queue_full_boundary` | D | S1 | Yes |
| 12 | `one_command_per_tick_enforced` | D | S2 | Yes |
| 13 | `shutdown_terminates_tick_loop` | D | S3 | Yes |
| 14 | `run_lifecycle_submit_cancel_exclusivity` | D | S4 | Yes |
| 15 | `shard_rejects_submit_when_at_capacity` | D | S4 | Yes |
| 16 | `step_state_transition_validity` | E | M1 | Yes |
| 17 | `mark_step_rejects_invalid_state_transitions` | E | M2 | Yes |
| 18 | `drive_finishes_with_correct_result` | F | Postcondition | Yes |
| 19 | `drive_awaiting_action_signal_mapping` | F | Postcondition | Yes |
| 20 | `drive_awaiting_wait_signal_mapping` | F | Postcondition | Yes |
| 21 | `drive_awaiting_ask_signal_mapping` | F | Postcondition | Yes |
| 22 | `drive_budget_exhausted_returns_correct_signal` | F | Postcondition | Yes |
| 23 | `drive_pc_always_in_bounds` | F | PC Invariant | Yes |
| 24 | `runtime_engine_errors_return_correct_variants` | G | Error Taxonomy | Yes |
| 25 | `branch_limit_exceeded_error` | G | Error Taxonomy | Yes |
| 26 | `frame_pool_error_variants` | G | Error Taxonomy | Yes |
| 27 | `adversarial_workflow_stress` | H | All | Yes |
| 28 | `adversarial_evidence_flood` | H | E4,F1 | Yes |
| 29 | `adversarial_frame_take_release_cycles` | H | F1,F3 | Yes |

**Total: 29 test functions across 8 categories.**

---

*Test plan synthesized from vb-6azo contract. All invariants from Section 3 covered. All EARS scenarios from Section 5 covered. Error taxonomy from Section 4 covered.*
