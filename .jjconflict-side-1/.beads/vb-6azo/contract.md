# Contract: vb-6azo — Behavioral Property Tests for Workflow Engine Invariants

## Bead Metadata

- **Bead ID:** vb-6azo
- **Title:** quality: Behavioral property tests for workflow engine invariants
- **Workspace:** ../vb-6azo-ws
- **Contractor:** rust-contract specialist
- **Status:** CONTRACT phase

---

## 1. Governing Context

### 1.1 Source Documents (read before writing this contract)

| File | Relevance |
|------|-----------|
| `velvet-ballistics-MASTER.md` | Sections 13 (Resource Contracts), 14 (Core Rust Types), 15 (Final IR Contract), 18 (Fjall Persistence Behavior), 20 (Runtime and Shard Design) |
| `crates/vb_runtime/src/shard/impl_.rs` | Shard construction, command queue, tick processing, frame pool lifecycle |
| `crates/vb_runtime/src/engine/drive.rs` | Deterministic drive loop, evidence collection, budget exhaustion |
| `crates/vb_runtime/src/engine/types.rs` | `EvidenceCollector`, `RuntimeEngineError`, `RuntimeSignal`, `RetryPolicy` |
| `crates/vb_runtime/src/engine/helpers.rs` | `mark_step_after_signal` step-state machine transitions |
| `crates/vb_runtime/src/frame_pool.rs` | Bounded frame pool, capacity limits, dimension mismatches |
| `crates/vb_runtime/src/shard/types.rs` | `ShardConfig`, `ShardCommand`, `RunState`, `ShardStatus` |
| `crates/vb_storage/src/proptests.rs` | Existing property tests for storage encoding, key ordering, admission |

### 1.2 What This Contract Covers

Behavioral property tests (proptest/strategies) that verify the **runtime engine**, **shard**, **frame pool**, and **evidence collection** subsystems maintain critical invariants under adversarial inputs.

What is **in scope**:
- `drive_deterministic_full` invariant preservation (evidence chain, budget, PC bounds)
- `EvidenceCollector` capacity bounding and event ordering
- `FramePool` capacity limits and dimension-gated reuse
- `Shard.tick()` command processing and queue boundedness
- `mark_step_after_signal` step-state machine transitions
- `RuntimeEngineError` variant coverage and runtime code mapping
- `RuntimeSignal` exhaustive coverage of suspension/termination paths
- Shutdown and cancellation lifecycle invariants

What is **out of scope** (covered by other beads):
- YAML parsing, validation, compile-time IR construction
- Generated Rust workflow equivalence (codegen bead)
- Fjall storage durability/replay (storage bead)
- Action ABI and capability enforcement (action bead)
- Network/IPC protocol fuzzing (IPC bead)

---

## 2. EARS Preconditions and Postconditions

### 2.1 `drive_deterministic_full`

**Precondition (Ubiquitous):**
- `plan` is a valid `CompiledWorkflow` produced by `try_from_parts`
- `run` is a `RunFrame` initialized with `RunFrame::new(run_id, entry, step_count, slot_count)` where `step_count == plan.node_count()`
- `budget` is constructed via `StepBudget::new(n)` where `n >= 0`
- `evidence` is constructed via `EvidenceCollector::new()` or `EvidenceCollector::with_capacity(k)`
- `store` is a `ValueStore::new()`
- `collect_states` is a `CollectStates::new()`
- `granted` is a `CapabilitySet`

**Postcondition (State Transition):**
- When returned `RuntimeSignal::Finished(v)`, `v` equals the value in slot `result` of the `Finish` node
- When returned `RuntimeSignal::StepBudgetExhausted`, `budget.try_take()` returned `Ok(false)` at least once
- When returned `RuntimeSignal::AwaitingAction(ticket)`, the current node is a `Do` node and `ticket.run == run.run_id()`
- When returned `RuntimeSignal::AwaitingWait`, the current node is a `WaitUntil` node and deadline has not elapsed
- When returned `RuntimeSignal::AwaitingAsk`, the current node is an `Ask` node

**Postcondition (Evidence Chain Invariant):**
- For every step that emits `StepSucceeded`, exactly one `StepStarted { step }` with matching `step` appears **earlier** in the evidence stream
- `StepSucceeded` with `output: Some(slot)` is always accompanied by a preceding `SlotWritten { slot, value }` for the same step
- No `StepSucceeded` appears for steps that return `Awaiting*` signals (Ask, Wait, Action)
- Evidence drain resets `dropped` counter to zero

**Postcondition (Budget Invariant):**
- `budget.try_take()` is called exactly once per loop iteration before node dispatch
- When `budget` reaches 0, the loop exits with `StepBudgetExhausted` immediately; no node executes with exhausted budget
- A budget of 0 always returns `StepBudgetExhausted` without executing any node

**Postcondition (PC Invariant):**
- `run.pc()` is always a valid `StepIdx` within `[0, plan.node_count())`
- PC advances only via explicit `next` field of the current node or `mark_step_after_signal`
- No unchecked indexing on `plan.nodes`

### 2.2 `EvidenceCollector`

**Precondition:** Capacity `k` passed to `with_capacity(k)` satisfies `k > 0`

**Postcondition (Bounded Collection):**
- After `push_*` calls totalling `n > k` events, `dropped() == n - k` (saturating) and `len() == k`
- After `drain()`, `len() == 0` and `dropped() == 0`
- `capacity()` always returns the value passed to constructor

**Postcondition (Event Ordering):**
- `push_step_started(s)` prepends `StepStarted { step: s }` to the internal event buffer
- Events are emitted in the order they were pushed (FIFO)

### 2.3 `FramePool`

**Precondition:** `new(step_count, slot_count, capacity)` requires `capacity > 0 && capacity <= 4096`

**Postcondition (Capacity Bound):**
- After any number of `release()` calls, `available() <= capacity`
- `release()` silently drops frames when `available() == capacity` (no panic, no error)

**Postcondition (Dimension Gate):**
- A frame with `(step_count, slot_count)` dimensions can only be returned to a pool with matching dimensions
- A frame from pool A with dimensions `(s1, c1)` released to pool B with `(s2, c2)` where `s1 != s2 || c1 != c2` is silently dropped

**Postcondition (Reuse Clears State):**
- A reused frame from `take()` after a prior `release()` has:
  - `run_id` set to the new `run_id` argument
  - `pc` set to the new `first_step` argument
  - `executed() == 0`
  - All `StepState` entries reset to `Pending`
  - All slot values return `SlotUninitialized` error on read
  - All taint entries return `SlotUninitialized` error on read

### 2.4 `Shard.tick()` and Command Queue

**Precondition:** Shard is not `shutting_down`

**Postcondition (Command Processing):**
- Each call to `tick()` processes **at most one** `ShardCommand` from the queue
- If queue is empty, `tick()` returns `Ok(true)` without mutating shard state
- `Shutdown` command sets `shutting_down = true` and `tick()` returns `Ok(false)`

**Postcondition (Queue Bounds):**
- `enqueue(cmd)` returns `Err(RuntimeError::QueueFull)` when `command_queue.len() == command_queue.capacity()`
- `remaining_capacity()` returns `capacity - len()` (saturating)
- `is_queue_full()` returns `len() == capacity`

**Postcondition (Run Lifecycle):**
- `Submit` command inserts a `RunState` entry into `self.runs` keyed by `run`
- `Cancel` command removes the `RunState` entry and releases the frame to the pool
- After `Cancel`, the run is no longer in `self.runs` and cannot be resumed

### 2.5 `mark_step_after_signal`

**Precondition:** `run` is a valid `RunFrame`, `step` is a valid `StepIdx`

**Postcondition (State Machine Transitions):**

| Signal | Step State Before | Step State After |
|--------|------------------|------------------|
| `AwaitingWait` | `Running` | `Waiting` |
| `AwaitingAsk` | `Running` | `Asking` |
| `AwaitingAction(_)` | `Running` | `Running` (no change) |
| `StepBudgetExhausted` | `Running` | `Running` (no change) |
| `Continue` | `Running` | `Succeeded` |
| `Finished(_)` | `Running` | `Succeeded` |

**Postcondition (Invalid Transition Rejection):**
- If step state is not `Running` and signal is `Continue` or `Finished`, returns `Err(EngineError::InternalInvariantViolation)`

---

## 3. Invariants to Be Tested

### 3.1 Evidence Chain Invariants (Phase 40/44 requirement)

```
INV(E1): event_ordering
  For all runs of drive_deterministic_full:
    Let S = [i | events[i] is StepStarted]
    Let P = [i | events[i] is StepSucceeded]
    For all (s_idx, s_step) in S and (p_idx, p_step) in P:
      If s_step == p_step then s_idx < p_idx

INV(E2): started_before_slot_written
  For all step executions that produce SlotWritten:
    StepStarted(step) appears in evidence before SlotWritten(_, step, _)

INV(E3): no_spurious_succeeded
  For all AwaitingAction, AwaitingWait, AwaitingAsk signals:
    No StepSucceeded event is emitted for that step

INV(E4): evidence_drain_resets_dropped
  After evidence.drain():
    len() == 0
    dropped() == 0
```

### 3.2 Budget Invariants

```
INV(B1): budget_exhaustion_stops_execution
  When budget.try_take() returns false:
    No node is dispatched in that iteration
    Loop exits with StepBudgetExhausted

INV(B2): zero_budget_means_no_execution
  StepBudget::new(0).try_take() returns Ok(false) on first call

INV(B3): budget_decrement_is_unit
  Each successful try_take() decrements remaining by exactly 1
```

### 3.3 Frame Pool Invariants

```
INV(F1): capacity_never_exceeded
  For all FramePool instances and all operations:
    pool.available() <= pool.capacity()

INV(F2): dimension_mismatch_drops
  release(frame) to a pool with mismatched (step_count, slot_count):
    pool.available() is unchanged (frame silently dropped)

INV(F3): reuse_produces_clean_frame
  After take() returning a recycled frame:
    frame.executed() == 0
    All slot reads return SlotUninitialized
    All taint reads return SlotUninitialized
    frame.pc() == first_step argument
    frame.run_id() == run_id argument
```

### 3.4 Shard Invariants

```
INV(S1): command_queue_bounded
  enqueue always succeeds unless queue is full
  QueueFull is returned exactly when is_queue_full()

INV(S2): one_command_per_tick
  Each tick() call processes at most one command

INV(S3): shutdown_termination
  After Shutdown command: tick() returns Ok(false) on all subsequent calls
  pending_timers are cleared on drain_for_shutdown

INV(S4): run_exclusivity
  A RunId appears in self.runs at most once at any time
```

### 3.5 Step State Machine Invariants

```
INV(M1): valid_state_transitions
  mark_step_after_signal only permits:
    Running -> Waiting (AwaitingWait)
    Running -> Asking (AwaitingAsk)
    Running -> Succeeded (Continue, Finished)
    Running -> Running (AwaitingAction, StepBudgetExhausted)

INV(M2): no_invalid_backward_transitions
  No transition from Waiting/Succeeding/Asking back to Running without a Resume command
```

---

## 4. Error Taxonomy

### 4.1 Runtime Engine Errors (covered by this bead)

| Error Variant | Condition | Expected Signal Mapping |
|--------------|-----------|------------------------|
| `RuntimeEngineError::Core(EngineError::InvalidProgramCounter)` | `plan.node(pc)` returns `None` | Returns as Err |
| `RuntimeEngineError::Core(EngineError::ExpressionStackOverflow)` | Expression stack exceeds `max_stack` | Returns as Err |
| `RuntimeEngineError::Core(EngineError::ExpressionStackUnderflow)` | Stack pop on empty stack | Returns as Err |
| `RuntimeEngineError::Core(EngineError::SlotOutOfBounds)` | Slot index >= slot_count | Returns as Err |
| `RuntimeEngineError::Core(EngineError::DivisionByZero)` | `/` or `%` with zero divisor | Returns as Err |
| `RuntimeEngineError::Core(EngineError::NonFiniteNumber)` | NaN or Inf in finite-only context | Returns as Err |
| `RuntimeEngineError::BranchLimitExceeded` | TogetherStart branches > u16::MAX | Returns as Err |
| `RuntimeEngineError::RetryExhausted` | Action attempts exceed policy max | Returns as Err |
| `RuntimeEngineError::TaintViolation` | Clean result from tainted input | Returns as Err |

### 4.2 Shard Errors (covered by this bead)

| Error Variant | Condition |
|--------------|-----------|
| `RuntimeError::QueueFull` | Enqueue when command queue is at capacity |
| `RuntimeError::CommandQueueCapacityExceeded` | ShardConfig::new with capacity == 0 or > MAX_COMMAND_QUEUE_CAPACITY |
| `RuntimeError::ActiveRunCapacityZero` | ShardConfig::new with max_active_runs == 0 |
| `RuntimeError::ShutdownInProgress` | drain_for_shutdown called with pending commands |
| `RuntimeError::FramePoolUnavailable` | Frame pool cannot allocate or return a frame |

### 4.3 Frame Pool Errors

| Error Variant | Condition |
|--------------|-----------|
| `CoreError::ResourceLimitExceeded { resource: "frame_pool_capacity" }` | capacity == 0 or > 4096 |
| `CoreError::AllocationFailed` | Pool empty and fresh allocation fails |
| `CoreError::InvalidCompiledWorkflow { reason: "step_count_zero" }` | `RunFrame::new` with step_count == 0 |

---

## 5. Test Scenarios (Given-When-Then)

### 5.1 Happy Path Tests

**GIVEN** a valid `CompiledWorkflow` with 2 steps: `SetConst(slot=0, const=42)` → `Finish(result=0)`
**WHEN** `drive_deterministic_full` is called with `budget = StepBudget::new(10)`
**THEN** the result is `RuntimeSignal::Finished(SlotValue::I64(42))`
**AND** evidence contains exactly: `StepStarted(0)`, `SlotWritten(0, I64(42))`, `StepSucceeded(0, Some(0))`, `StepStarted(1)`, `StepSucceeded(1, None)`

---

**GIVEN** a `FramePool` created with `new(4, 2, 3)` (step_count=4, slot_count=2, capacity=3)
**WHEN** 3 frames are taken and released in a cycle
**THEN** `available()` remains at 3 throughout

---

**GIVEN** a `Shard` created with `command_queue_capacity = 4`
**WHEN** 4 `Shutdown` commands are enqueued
**THEN** `is_queue_full()` returns `true`
**AND** the 5th enqueue returns `Err(RuntimeError::QueueFull)`

---

**GIVEN** a `CompiledWorkflow` with `TogetherStart { branches: [step1, step2], join: step3 }`
**WHEN** `compute_max_parallel_in_flight` is called
**THEN** the result is `Ok(2)`

---

### 5.2 Error Path Tests

**GIVEN** `StepBudget::new(0)`
**WHEN** `try_take()` is called
**THEN** it returns `Ok(false)`
**AND** a subsequent `drive_deterministic_full` returns `RuntimeSignal::StepBudgetExhausted` immediately

---

**GIVEN** a `FramePool` with `capacity = 1` and no recycled frames
**WHEN** `take()` is called twice without releasing
**THEN** both calls return `Ok` (pool always allocates fresh when empty)

---

**GIVEN** a `FramePool` with `capacity = 2`
**WHEN** 3 frames are taken and then all 3 are released
**THEN** `available()` equals `2` (3rd frame is silently dropped)

---

**GIVEN** a `CompiledWorkflow` with `ChooseSlot` where no branch condition matches and `otherwise` is `None`
**WHEN** the workflow is driven to the ChooseSlot node with all boolean slots = `false`
**THEN** the result is `Err(EngineError::MissingNextStep { step })`

---

**GIVEN** an `EvidenceCollector` with `capacity = 2`
**WHEN** 5 events are pushed
**THEN** `len() == 2` and `dropped() == 3`
**AND** `drain()` returns 2 events and resets `dropped()` to 0

---

**GIVEN** a `Shard` with `max_active_runs = 2`
**WHEN** 3 `Submit` commands are enqueued and processed
**THEN** the first 2 succeed, the 3rd returns `Err(RuntimeError::ActiveRunCapacityExceeded)`

---

### 5.3 Adversarial/Invariant Falsification Tests

**GIVEN** `drive_deterministic_full` with a workflow that has 1000 steps and `budget = StepBudget::new(500)`
**WHEN** driven to completion
**THEN** exactly 500 steps execute
**AND** `StepBudgetExhausted` is returned
**AND** PC is at step index 500

---

**GIVEN** a `FramePool` with `step_count=2, slot_count=1, capacity=4`
**WHEN** a frame from a pool with `step_count=4, slot_count=2` is released into it
**THEN** `available()` remains unchanged (dimension mismatch silent drop)

---

**GIVEN** `EvidenceCollector` with `capacity = 0`
**WHEN** any `push_*` method is called
**THEN** `len() == 0` and `dropped() == 1`

---

**GIVEN** a `CompiledWorkflow` with `TogetherStart` having `u16::MAX + 1` branches (created via `from_parts_unchecked`)
**WHEN** `compute_max_parallel_in_flight` is called
**THEN** it returns `Err(RuntimeEngineError::BranchLimitExceeded { max: u16::MAX, requested: u16::MAX + 1 })`

---

**GIVEN** a `Shard` in `shutting_down` state
**WHEN** `tick()` is called
**THEN** it returns `Ok(false)` immediately without processing any command

---

**GIVEN** `mark_step_after_signal` with a step in `Waiting` state
**WHEN** called with `RuntimeSignal::Continue`
**THEN** it returns `Err(EngineError::InternalInvariantViolation)`

---

## 6. Implementation Notes for the Test Writer

### 6.1 Proptest Strategy Hints

- **Workflows:** Use `CompiledWorkflow::try_from_parts` with `WorkflowParts` built from `vec![CompiledNode]` of bounded size (1..20 steps). Use `from_parts_unchecked` only for adversarial tests targeting validation bypass.
- **StepBudget:** Strategy: `u64::NON_ZERO..=10_000` for normal tests; `just(0u64)` for exhaustion tests.
- **RunFrame:** Use `RunFrame::new(run_id, entry, step_count, slot_count)` with `step_count > 0`.
- **EvidenceCollector:** Strategy: `EvidenceCollector::with_capacity(0..=100)`.
- **FramePool:** Strategy: `step_count: 1..=16`, `slot_count: 0..=16`, `capacity: 1..=100`.
- **SlotValue:** Use `proptest::sample::subrange` of `vec![I64(-1000..=1000), Bool, Null, Symbol]` to avoid corpus explosion.
- **SlotIdx/StepIdx:** Use `new(u16::MAX)` only in adversarial tests; normal tests use `new(0..100)`.

### 6.2 Required Test Functions

At minimum, the following invariant-preserving property tests must be written:

1. `evidence_chain_ordering_preserved` — proptest over random workflows with 1..50 steps, verifies INV(E1), INV(E2), INV(E3)
2. `budget_exhaustion_stops_at_exact_boundary` — verifies INV(B1), INV(B2)
3. `frame_pool_capacity_never_exceeded` — verifies INV(F1) under concurrent release cycles
4. `frame_pool_dimension_mismatch_silent_drop` — verifies INV(F2)
5. `frame_reuse_clears_all_prior_state` — verifies INV(F3)
6. `command_queue_full_boundary` — verifies INV(S1)
7. `one_command_per_tick_enforced` — verifies INV(S2)
8. `shutdown_terminates_tick_loop` — verifies INV(S3)
9. `step_state_transition_validity` — verifies INV(M1), INV(M2)
10. `evidence_drain_resets_dropped_counter` — verifies INV(E4)
11. `compute_max_parallel_rejects_overflow` — verifies BranchLimitExceeded invariant
12. `zero_capacity_collector_drops_all` — verifies EvidenceCollector edge case
13. `run_lifecycle_submit_cancel_exclusivity` — verifies INV(S4)
14. `mark_step_rejects_invalid_state_transitions` — verifies INV(M2)

### 6.3 Test File Location

All tests go in: `crates/vb_runtime/src/engine/property_tests.rs`

### 6.4 Existing Test Imports

```rust
use vb_core::engine::StepBudget;
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts};
use crate::engine::drive::{drive_deterministic_full, compute_max_parallel_in_flight};
use crate::engine::types::{EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeEngineError, RuntimeSignal};
use crate::primitives::collect::CollectStates;
use crate::frame_pool::FramePool;
```

---

## 7. Acceptance Criteria

The test suite is accepted when:

1. All 14 property tests compile under `cargo test -p vb_runtime`
2. `cargo clippy -p vb_runtime -- --deny warnings` passes with no new warnings
3. `proptest` generates at least 1000 iterations for each parameterized test without discovering a counterexample
4. Running `cargo miri test -p vb_runtime` on pure engine tests (`evidence_`, `budget_`, `frame_pool_`, `mark_step_`) completes without undefined behavior
5. Invariant falsification tests (adversarial) confirm that the invariants hold for all generated inputs
6. No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg` in the test file
7. Test names follow `snake_case` and include the invariant being tested

---

*Contract synthesized from runtime engine source analysis. EARS format. Invariants derived from phase 40/44 evidence chain requirements, frame pool capacity contracts, shard command queue bounds, and step state machine transition rules.*
