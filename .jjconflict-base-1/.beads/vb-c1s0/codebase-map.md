# vb-c1s0 Codebase Map: BDD Orchestration Runtime Acceptance Scenarios

**Bead:** vb-c1s0
**Title:** bdd: Orchestration runtime acceptance scenarios
**Source Checkout:** /home/lewis/src/velvet-ballistics
**Generated:** 2026-05-19

## 1. Executive Summary

This bead focuses on BDD (Behavior-Driven Development) orchestration runtime acceptance scenarios. The codebase implements a multi-shard runtime system for workflow execution with deterministic step processing, action scheduling, timer management, and recovery capabilities.

## 2. Crate Architecture

### 2.1 Primary Crates

| Crate | Purpose |
|-------|---------|
| `vb_runtime` | Multi-shard orchestration runtime, action scheduling, timer wheel, shard lifecycle |
| `vb_core` | Core engine: run loop, step execution, frame management, workflow compilation |
| `vb_cli` | CLI interface with BDD scenario tests |
| `vb_storage` | Journal events, recovery, replay tracking |
| `vb_validate` | Workflow validation, schema validation |
| `vb_codegen` | Code generation, IR lowering |
| `vb_compile` | YAML compilation, lowering to IR |
| `workspace_tests` | Acceptance catalog with BDD scenario definitions |

### 2.2 Key Modules

#### vb_runtime (`/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/`)

| Module | File(s) | Purpose |
|--------|---------|---------|
| Runtime | `runtime.rs` | Multi-shard routing, submit/resume/cancel/tick_all |
| Action Queue | `action_queue.rs` | Bounded action completion queue with backpressure |
| Shard | `shard/mod.rs`, `shard/impl_.rs`, `shard/lifecycle.rs` | Single-threaded shard execution |
| Shard Parts | `shard/impl_parts/chunk_001.rs` | Core tick processing |
| Lifecycle | `shard/lifecycle/chunk_001.rs`, `chunk_002.rs`, `chunk_003.rs` | Submit, resume, cancel handling |
| Timer Wheel | `shard/timer_wheel.rs` | Timer scheduling and firing |
| Primitives | `primitives/reduce.rs`, `primitives/repeat.rs`, `primitives/reentry*.rs` | Workflow primitives |

#### vb_core (`/home/lewis/src/velvet-ballistics/crates/vb_core/src/`)

| Module | File(s) | Purpose |
|--------|---------|---------|
| Engine | `engine.rs`, `engine/run_loop.rs`, `engine/step.rs` | Deterministic run loop, step execution |
| Frame | `frame.rs` | RunFrame state machine |
| Workflow | `workflow/mod.rs` | CompiledWorkflow, node kinds |

### 2.3 BDD Scenario Files

| File | Scenario Count | Focus |
|------|---------------|-------|
| `crates/vb_cli/tests/cli_vb_m214_bdd_scenarios.rs` | 17+ | CLI operator workflow BDD acceptance |
| `crates/vb_cli/tests/cli_verify_integration.rs` | 6 | Verify command BDD scenarios |
| `crates/vb_runtime/tests/recovery_bdd_tests.rs` | 20 (B-001 to B-020) | Recovery BDD tests |
| `crates/vb_runtime/src/primitives/reentry_tests.rs` | 6 (GWT-RE-1 to GWT-RE-6) | Reentry BDD Given/When/Then |
| `crates/workspace_tests/src/acceptance_catalog.rs` | 21 | Public acceptance catalog |

## 3. Execution Flow

### 3.1 Runtime Tick Flow

```
Runtime::tick_all()
  └── Shard::tick() for each shard
        └── match ShardCommand
              ├── Submit → handle_submit → drive_run
              ├── Resume → handle_resume → drive_run  
              ├── ActionCompleted → handle_action_completion → drive_run
              ├── Cancel → handle_cancel
              ├── AskAnswered → handle_ask_answer
              └── TimerFired → handle_timer
```

### 3.2 drive_run Flow

```
drive_run(run)
  └── run_until_blocked(plan, frame, budget, store)
        └── drive_deterministic()
              └── while budget.try_take():
                    └── step_once(plan, frame, store) → EngineSignal
                          ├── Continue → next step
                          ├── AwaitingAction → suspend (action pending)
                          ├── AwaitingAsk → suspend (ask pending)
                          ├── AwaitingTimer → suspend (timer pending)
                          ├── Finished → terminal
                          └── StepBudgetExhausted → suspend
```

### 3.3 Key Data Structures

| Structure | Location | Purpose |
|-----------|----------|---------|
| `Runtime` | `vb_runtime/src/runtime.rs:32` | Multi-shard container |
| `Shard` | `vb_runtime/src/shard/types.rs` | Single-threaded execution context |
| `RunState` | `vb_runtime/src/shard/types.rs` | Per-run mutable state |
| `RunFrame` | `vb_core/src/frame.rs` | Program counter, step states |
| `CompiledWorkflow` | `vb_core/src/workflow/mod.rs` | Immutable workflow plan |
| `ActionTicket` | `vb_core/src/action.rs` | Action completion handle |
| `AskTicket` | `vb_runtime/src/shard/types.rs` | Ask/resume handle |

### 3.4 Command Routing

- **RunId → Shard**: `run_id.get() % shard_count` (consistent hashing)
- **Commands**: `ShardCommand` enum with submit, resume, complete_action, fail_action, answer_ask, timer_fired, cancel, inspect, shutdown

## 4. Scenario Scheduling

### 4.1 Action Scheduling

- `ActionScheduled` event emitted when action is deferred
- `ActionCompleted` event when external completion delivered
- `ActionFailed` event when action fails
- Bounded `BoundedActionCompletionQueue` for backpressure

### 4.2 Timer Scheduling

- `TimerWheel` in `vb_runtime/src/shard/timer_wheel.rs`
- `TimerEntry` with generation, deadline, kind (Wait/Retry)
- `capture_timer_entry()` for explicit authority handoff
- `timer_entry_fired()` for advancing from captured authority

### 4.3 Ask Scheduling

- `AskScheduledEvent` journal event
- `AskAnswered` command for external answer delivery
- Resume step advancement after answer

## 5. Recovery and Replay

### 5.1 Recovery Modules

| File | Purpose |
|------|---------|
| `vb_storage/src/recovery.rs` | Full journal recovery |
| `vb_runtime/src/recovery.rs` | Runtime frame seed recovery |
| `vb_runtime/tests/recovery_bdd_tests.rs` | B-001 to B-020 tests |

### 5.2 Journal Events

| Event | Purpose |
|-------|---------|
| `RunAccepted` | Run admitted |
| `RunAdmission` | Capability grants recorded |
| `StepStarted` | Step began execution |
| `SlotWrittenEvent` | Slot value written |
| `StepSucceeded` | Step completed |
| `ActionScheduled` | Action deferred |
| `ActionCompleted` | Action completed |
| `WaitScheduledEvent` | Wait timer scheduled |
| `AskScheduledEvent` | Ask scheduled |
| `RetryScheduledEvent` | Retry scheduled |
| `RunFinished` | Run completed |

## 6. Acceptance Catalog Structure

```rust
// From crates/workspace_tests/src/acceptance_catalog.rs
pub struct Scenario {
    pub id: &'static str,           // e.g., "BDD-KYYF-001"
    pub master_behavior: &'static str,
    pub given: &'static str,
    pub when: &'static str,
    pub then: &'static str,
    pub public_surface: &'static str,  // API surface
    pub fixture: &'static str,
    pub expected_outcome: Option<&'static str>,
    pub expected_error: Option<&'static str>,
    pub durability_profile: &'static str,
    pub related_bead: &'static str,
    pub executable_evidence_target: Option<&'static str>,
    pub deferred_follow_up_bead: Option<&'static str>,
}
```

## 7. Key Public APIs

### 7.1 Runtime Public API (`vb_runtime`)

```rust
// Submission
pub fn submit_direct(&self, run: RunId, workflow: CompiledWorkflow) -> RuntimeResult<()>
pub fn submit_direct_with_grants(&self, run, workflow, caps) -> RuntimeResult<()>
pub fn submit_direct_with_inputs_grants_and_contracts(&self, run, workflow, inputs, caps, contracts) -> RuntimeResult<()>

// Lifecycle
pub fn cancel_run(&self, run: RunId) -> RuntimeResult<()>
pub fn resume_run(&self, run: RunId) -> RuntimeResult<()>

// Action Completion
pub fn complete_action_with_output(&self, ticket: ActionTicket, output: ActionOutputReady) -> RuntimeResult<()>
pub fn fail_action(&self, ticket: ActionTicket, failure: ActionFailure) -> RuntimeResult<()>

// Ask
pub fn answer_ask(&self, answer: AskAnswer) -> RuntimeResult<()>

// Timer
pub fn timer_entry_fired(&self, entry: TimerEntry) -> RuntimeResult<()>

// Inspection
pub fn snapshot_run(&self, run: RunId, correlation: u64) -> RuntimeResult<InspectResponse>
pub fn list_events(&self, run: RunId) -> RuntimeResult<Vec<TraceEvent>>

// Tick
pub fn tick_all(&mut self) -> RuntimeResult<bool>

// Metrics
pub fn collect_metrics(&self) -> RuntimeMetricsSnapshot
pub fn counters_snapshot(&self) -> CounterSnapshot
```

### 7.2 Engine Public API (`vb_core`)

```rust
// From engine/run_loop.rs
pub fn run_until_blocked(plan, run, budget, store) -> Result<EngineSignal, EngineError>
pub fn drive_deterministic(plan, run, budget, store) -> Result<EngineSignal, EngineError>

// From engine/step.rs  
pub fn step_once(plan: &CompiledWorkflow, run: &mut RunFrame, store: &mut ValueStore) -> Result<EngineSignal, EngineError>
```

## 8. Risk Tags

| Category | Risk | Location |
|----------|------|----------|
| Concurrency | Multi-shard command routing | `runtime.rs:556-573` (shard_index) |
| Temporal | Timer wheel generation mismatch | `timer_wheel.rs` |
| Persistence | Journal sequence overflow | `chunk_001.rs:130-136` |
| Concurrency | Action queue backpressure | `action_queue.rs` |
| Recovery | Replay re-scheduling idempotency | `recovery_bdd_tests.rs` |
| Budget | Step budget exhaustion | `run_loop.rs:28-34` |

## 9. Verifier Modes Required

| Lane | Evidence Type | Required |
|------|---------------|----------|
| Kani | Bounded panic-freedom for tick/shutdown paths | Yes |
| Miri | Unsafe Send+Sync on SharedRuntimeJournal | Yes |
| Loom | Command queue ordering | Yes |
| Verus | Timer wheel deadline ordering | TBD |
| Flux | Action contract refinement | TBD |

## 10. File Manifest

### Core Runtime Files
- `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/runtime.rs`
- `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/action_queue.rs`
- `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/shard/mod.rs`
- `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/shard/impl_.rs`
- `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/shard/impl_parts/chunk_001.rs`
- `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/shard/lifecycle.rs`
- `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/shard/lifecycle/chunk_001.rs`
- `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/shard/timer_wheel.rs`
- `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/shard/types.rs`

### Engine Files
- `/home/lewis/src/velvet-ballistics/crates/vb_core/src/engine.rs`
- `/home/lewis/src/velvet-ballistics/crates/vb_core/src/engine/run_loop.rs`
- `/home/lewis/src/velvet-ballistics/crates/vb_core/src/engine/step.rs`
- `/home/lewis/src/velvet-ballistics/crates/vb_core/src/frame.rs`

### BDD Test Files
- `/home/lewis/src/velvet-ballistics/crates/vb_cli/tests/cli_vb_m214_bdd_scenarios.rs`
- `/home/lewis/src/velvet-ballistics/crates/vb_cli/tests/cli_verify_integration.rs`
- `/home/lewis/src/velvet-ballistics/crates/vb_runtime/tests/recovery_bdd_tests.rs`
- `/home/lewis/src/velvet-ballistics/crates/workspace_tests/src/acceptance_catalog.rs`

### Storage/Recovery Files
- `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/journal.rs`
- `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/recovery.rs`

## 11. Open Questions

1. **DISCOVERY_BLOCKED**: The exact BDD scenario definitions for `vb-c1s0` bead itself need to be confirmed - whether this bead defines NEW scenarios or validates EXISTING catalog scenarios.
2. **DISCOVERY_BLOCKED**: The relationship between `vb-c1s0` and the `vb-kyyf` cross-run determinism scenarios needs clarification.
3. **UNKNOWN**: Whether additional scenario scheduling beyond `ActionScheduled`/`WaitScheduled`/`AskScheduled` exists for compound workflows.

## 12. Handoff Notes

- Multi-shard routing is deterministic via `RunId % shard_count`
- All shard operations are single-threaded; no locking needed within shard
- Journal events provide the canonical replay evidence chain
- Timer wheel uses generation to prevent stale timer delivery
- Action queue is bounded with 80% backpressure warning threshold
