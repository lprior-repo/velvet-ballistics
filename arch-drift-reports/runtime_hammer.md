# Architectural Drift Report: vb_runtime/src/runtime.rs

## Line Count Violation
**Current: 2718 lines | Limit: 300 lines | Ratio: 9.1x OVER**

---

## Executive Summary

This file is a **GODZILLA-CLASS architectural violation**. It crams **six distinct DDD bounded contexts** into a single 2718-line file: runtime construction, command routing, scheduling orchestration, migration, metrics, and a 1600-line test module. The `Runtime` struct violates every Scott Wlaschin principle: it's a God Object, it leaks primitive obsession everywhere, and its "state machine" is a match statement wearing a trench coat.

---

## Function Responsibilities Requiring Separation

### Responsibility 1: Runtime Construction (lines 38-69)
**Current:** `Runtime::new`, `Runtime::new_with_journal`
**Problem:** Mixing journal configuration with shard bootstrapping
**Should be:** `RuntimeBuilder` or constructor injection

---

### Responsibility 2: Command Submission Gateway (lines 72-171)
**Current:** 9 public submit methods with combinatorial explosion of variants:
- `submit_direct` / `submit_direct_with_grants` / `submit_direct_with_grants_and_contracts` / `submit_direct_with_inputs_grants_and_contracts`
- `submit_compiled` / `submit_compiled_with_grants` / `submit_compiled_with_inputs` / `submit_compiled_with_inputs_and_grants`

**Problem:** This is a classic **Feature Envy** smell — all these methods delegate to `shard_for(run)?` then `shard.enqueue(...)`. The runtime is just a router, yet it exposes 9 different submission surfaces.

**Should be:** A single `submit` method with a typed `SubmitRequest` struct containing optional fields. Or at minimum, collapse into 2-3 methods.

---

### Responsibility 3: Run Lifecycle Operations (lines 174-189)
**Current:** `cancel_run`, `resume_run`, `inspect_run`, `snapshot_run`
**Problem:** These are 4 different **workflow transitions** jammed into one struct
**Should be:** A `RunController` or `RunLifecycleManager` trait/object

---

### Responsibility 4: Tick / Scheduling Orchestration (lines 198-315)
**Current:** `tick_all`, `tick_shard`, `migrate_shard`
**Problem:** This is the **SCHEDULING CONTEXT** — it coordinates work across shards. 118 lines of directive processing, migration logic, and tick loops.
**Should be:** `Scheduler` or `TickOrchestrator` in its own module

---

### Responsibility 5: Action Completion Handlers (lines 318-346)
**Current:** `complete_action`, `complete_action_with_output`, `fail_action`
**Problem:** These handle **asynchronous action resolution** — a distinct workflow state transition
**Should be:** `ActionCompletionHandler` or merged into lifecycle controller

---

### Responsibility 6: Timer Management (lines 370-390)
**Current:** `timer_fired`, `capture_timer_entry`, `timer_entry_fired`
**Problem:** Timer authority capture and firing is a **temporal coordination** concern
**Should be:** `TimerAuthority` or `TimerCoordinator` trait

---

### Responsibility 7: Observation / Metrics (lines 393-550)
**Current:** `take_inspect_response`, `drain_trace`, `collect_metrics`, `counters_snapshot`, `list_active_runs`
**Problem:** 158 lines of **observability** code mixed with orchestration
**Should be:** `RuntimeObserver` or separate `MetricsCollector` module

---

### Responsibility 8: Shard Routing (lines 561-583)
**Current:** `shard_index`, `shard_for`, `shard_for_mut`
**Problem:** Internal **routing infrastructure** exposed as private methods
**Should be:** `ShardRouter` or `ShardRegistry`

---

### Responsibility 9: Test Module (lines 586-2718)
**Current:** 2132 lines of tests with inline fixture helpers
**Problem:** The test module is **larger than most production crates**. Fixture functions like `suspended_workflow()`, `action_then_finish_workflow()`, `wait_then_finish_workflow()`, `ticket()`, `runtime_config()` are duplicated across tests.
**Should be:** Move to `tests/` directory at crate root, use shared test fixtures module

---

## Primitive Obsession Map

| Raw Type | Location | Should Be Domain Type |
|----------|----------|----------------------|
| `u64` | `correlation: u64` (line 186) | `CorrelationId(u64)` — request/response matching |
| `u32` | `shard_index: u32` (line 223) | `ShardIndex(u32)` — shard identification |
| `u32` | `target: u32` in `ShardDirective::Migrate` | `ShardIndex(u32)` — same type as above |
| `u32` | `limit: u32` (line 492) | `QueryLimit(u32)` — pagination boundary |
| `u64` | `runs_active`, `runs_waiting` counters (lines 415-416) | `RunCount(u64)` — domain quantity |
| `u64` | `runs_failed_total`, `runs_finished_total` (lines 417-418) | `RunCount(u64)` |
| `u64` | `steps_total` (line 419) | `StepCount(u64)` |
| `u32` | `shard_id` in metrics (line 444) | `ShardIndex(u32)` |
| `u32` | `queue_depth`, `queue_remaining` (lines 424-425) | `QueueDepth(u32)` |
| `u32` | `frame_pool_free`, `frame_pool_total` (lines 428-429) | `FramePoolMetrics` tuple |
| `u16` | `trace_capacity`, `trace_len` (lines 430-431) | `TraceRingMetrics` |
| `u16` | `step_count`, `steps_completed` in `ActiveRunSummary` | `StepCount(u16)`, `CompletedSteps(u16)` |
| `usize` | `count`, `index` in shard iteration (lines 57-62) | `ShardCount(usize)` |
| `f32` | `trace_ring_fill_pct` (line 433) | `Percentage(f32)` — type-safe percentage |

---

## State Machine Violations

### Violation 1: ShardDirective is a Schizophrenic State Machine

```rust
pub enum ShardDirective {
    Continue,    // Alive, processing
    Suspend,      // Alive, NOT processing
    Migrate { target: u32 },  // Transitioning to another shard
    Shutdown,    // Terminal: draining
    Cancel,      // REJECTED — but encoded as a variant!
    Barrier,     // REJECTED — but encoded as a variant!
}
```

**Problems:**
- `Cancel` and `Barrier` return `UnsupportedOperation` — they should NOT be variants
- `Migrate` carries `target: u32` primitive obsession
- No explicit transition guards or preconditions modeled
- No terminal/non-terminal distinction in types

**Should be:**
```rust
// Two separate enums for orthogonal concerns
pub enum ShardLifecycle {
    Continue,
    Suspend,
    Shutdown,
}

pub enum ShardMigration {
    MigrateTo(ShardIndex),
    // Self-migration caught at construction time, not at tick time
}
```

---

### Violation 2: Run Lifecycle States Are Implicit

The `Runtime` manages runs through commands (`Submit`, `Cancel`, `Resume`, `Inspect`) but the **run state machine** (`Submitted → Active → Suspended → Completed/Failed/Cancelled`) is hidden inside `Shard`, not expressed in `Runtime`.

**Problem:** `Runtime` is orchestrating a state machine it doesn't model.

---

### Violation 3: Tick Return Value is a Hack

```rust
pub fn tick_all(&mut self) -> RuntimeResult<bool>  // false = shutting down
```

The `bool` return tells callers if the runtime is alive, but this is **encoding state in a primitive**. The `bool` should be part of a `RuntimeStatus` enum:

```rust
pub enum RuntimeStatus {
    Alive,
    ShuttingDown,
    Dead,
}
```

---

## Recommended Module Split

```
vb_runtime/src/
├── runtime/
│   ├── mod.rs           (50-80 lines: Runtime struct only, delegate everything else)
│   ├── builder.rs       (80-100 lines: Runtime construction)
│   ├── router.rs        (50-70 lines: shard_index, shard_for, shard_for_mut)
│   ├── submit.rs        (60-80 lines: submit variants collapsed to typed SubmitRequest)
│   ├── lifecycle.rs     (80-100 lines: cancel, resume, inspect, snapshot)
│   ├── scheduler.rs      (100-120 lines: tick_all, tick_shard, migrate_shard)
│   ├── timer.rs          (40-60 lines: timer authority capture/fire)
│   └── observation.rs    (80-100 lines: metrics, trace, counters)
├── runtime/tests/        (tests moved here as integration tests)
│   ├── submit_tests.rs
│   ├── lifecycle_tests.rs
│   ├── scheduler_tests.rs
│   └── fixtures.rs       (shared test fixtures)
└── lib.rs
```

**Target sizes:**
- `runtime/mod.rs`: ~80 lines (public API surface, constructor delegation)
- Each submodule: 40-120 lines MAX
- Total runtime crate: Should be ~600 lines, not 2718

---

## Immediate Refactoring Mandate

1. **Extract a `RuntimeSubmitter`**: Collapse 9 submit methods into 1 with `SubmitRequest { run, workflow, inputs, caps, contracts }` — all optional
2. **Extract `ShardRouter`**: The `shard_index`, `shard_for`, `shard_for_mut` trio is a pure routing concern
3. **Extract `RuntimeScheduler`**: `tick_all`, `tick_shard`, `migrate_shard` are scheduling
4. **Extract test fixtures**: Move all `fn workflow() -> Option<CompiledWorkflow>` helpers to a test fixtures module
5. **Type aliases for ALL primitives**: Create `CorrelationId`, `ShardIndex`, `QueryLimit`, etc.
6. **Fix `ShardDirective`**: Remove `Cancel` and `Barrier` variants, add `RuntimeStatus` enum

---

## DDD Bounded Context Boundaries Detected

| Context | Currently In | Should Be |
|---------|--------------|-----------|
| Runtime Construction | `runtime.rs` | `runtime/builder.rs` |
| Command Routing | `runtime.rs` | `runtime/router.rs` |
| Scheduling | `runtime.rs` | `runtime/scheduler.rs` |
| Run Lifecycle | `runtime.rs` | `runtime/lifecycle.rs` |
| Metrics/Observability | `runtime.rs` | `runtime/observation.rs` |
| Timer Coordination | `runtime.rs` | `runtime/timer.rs` |

---

## Verdict

**REFACTOR OR PERISH.** This file is a monolith that will breed bugs, resist testing, and block parallelism. The 9.1x line count overrun is a symptom of context leakage. Each DDD bounded context identified above must become its own module before any new feature work is permitted on this crate.
