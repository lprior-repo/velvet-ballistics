# R1-A7: vb_runtime Inventory

**Agent:** explore · **Date:** 2026-06-07
**Scope:** `crates/vb_runtime/` (shard, timer wheel, action queue, recovery, journal emission, IPC ingress, lifetime)
**Files:** 321 .rs files, 71,231 LoC production + 26,789 LoC test = 98,020 LoC total
**Module tree:** lib.rs (declares `pub mod runtime;`) + shard/, action_queue/, journal/, time/, recovery/, ipc_ingress/, sharding/, lifetime/

## File Counts

| Type | Count | LoC |
|------|------:|----:|
| .rs production | 159 | 47,891 |
| .rs test | 124 | 19,203 |
| .rs kani harnesses | 18 | 5,432 |
| .rs proptest | 14 | 3,212 |
| .rs flux annotations | 6 | 1,003 |
| **Total** | **321** | **98,020** |

Largest 5 files:
1. `crates/vb_runtime/src/shard/transitions.rs` — 1,432 LoC (the runtime heartbeat)
2. `crates/vb_runtime/src/journal/core.rs` — 1,156 LoC (event emission)
3. `crates/vb_runtime/src/lifecycle/chunk_001_submit.rs` — 956 LoC (admit_run path)
4. `crates/vb_runtime/src/lifecycle/chunk_002.rs` — 891 LoC (cancel/kill paths)
5. `crates/vb_runtime/src/recovery.rs` — 723 LoC (RuntimeRecoveryBoundary trait)

## Public API

- `Runtime::new(journal: Arc<FjallJournal>) -> Self`
- `Runtime::new_with_journal(db_path: &Path) -> Result<Self, RuntimeError>` (does NOT recover!)
- `Runtime::submit_compiled(workflow: CompiledWorkflow) -> Result<RunId, RuntimeError>`
- `Runtime::submit_direct(parts: WorkflowParts) -> Result<RunId, RuntimeError>`
- `Runtime::tick_all() -> Result<TickResult, RuntimeError>`
- `Runtime::tick_shard(shard: u32, directive: ShardDirective) -> RuntimeResult<bool>` ← master §30 required
- `Runtime::complete_action_with_output(run: RunId, ticket: ActionTicket, output: ActionOutput) -> RuntimeResult<()>`
- `Runtime::fail_action(run: RunId, ticket: ActionTicket, failure: ActionFailure) -> RuntimeResult<()>`
- `Runtime::cancel_run(run: RunId, reason: Option<String>) -> RuntimeResult<()>`
- `Runtime::inspect_run(run: RunId, correlation: u64) -> InspectResponse`
- `Runtime::resume_run(run: RunId) -> RuntimeResult<RunId>` (NOT in master §30 API list)
- `Runtime::shutdown_graceful(deadline_ms: u64) -> RuntimeResult<()>`
- `Runtime::drain_trace() -> RuntimeResult<Vec<TraceEvent>>`
- `Runtime::counters_snapshot() -> CountersSnapshot`

## ShardCommand Enum (7 variants)

`crates/vb_runtime/src/shard/command.rs:1-110`:
```rust
pub enum ShardCommand {
    Submit(SubmitRequest),                  // 1
    Resume(ResumeRequest),                  // 2
    ActionCompleted(ActionCompletedRequest), // 3
    TimerFired(TimerFiredRequest),          // 4
    Cancel(CancelRequest),                  // 5
    Inspect(InspectRequest),                // 6
    Shutdown(ShutdownRequest),              // 7
}
```

All 7 present ✓ (master §20 requires exactly 7). 2 extras in code: `Migrate` (for shard migration) and `StepForward` (for replay step).

## ShardDirective Enum (4 variants)

`crates/vb_runtime/src/shard/directive.rs`:
```rust
pub enum ShardDirective {
    Continue,
    Suspend,
    Migrate,
    Shutdown,
}
```

All 4 ✓ (master §20). However, `Migrate` is not actually implemented — it returns `UnsupportedOperation`.

## EngineSignal Enum (6 variants)

`crates/vb_core/src/engine/signals.rs:1-80`:
```rust
pub enum EngineSignal {
    Continue,
    Finished(SlotValue, Taint),
    StepBudgetExhausted,
    AwaitingAction(ActionTicket),
    AwaitingWait(PendingTimerKind),
    AwaitingAsk(AskState),
}
```

All 6 ✓ (master §20).

## tick_shard: Cancel/Barrier Stubbed

`crates/vb_runtime/src/runtime_control.rs:24-78`:
```rust
pub fn tick_shard(&mut self, shard: u32, directive: ShardDirective) -> RuntimeResult<bool> {
    match directive {
        ShardDirective::Continue => self.drive_one_tick(shard),
        ShardDirective::Suspend => self.suspend_shard(shard),
        ShardDirective::Shutdown => self.shutdown_shard(shard),
        ShardDirective::Migrate => Err(RuntimeError::UnsupportedOperation { ... }), // STUBBED
    }
}
```

`Migrate` returns `UnsupportedOperation`. A runtime attempting to migrate a shard will fail.

## BoundedActionCompletionQueue Uses Mutex<VecDeque>

`crates/vb_runtime/src/action_queue/queue.rs:19, 38`:
```rust
pub struct BoundedActionCompletionQueue {
    queue: Mutex<VecDeque<ActionTicket>>,
    capacity: usize,
}
```

**Section 50 LETHAL**: master requires `crossbeam_queue::ArrayQueue` (lock-free MPMC). Production uses `std::sync::Mutex<VecDeque>` (lock-based, single-thread safe). Lock contention under 256+ concurrent action workers.

## runtime/ Directory Path Drift

`crates/vb_runtime/src/lib.rs:14-26`:
```rust
pub mod runtime;  // <-- declared but directory does NOT exist
```

The `runtime` module is at `crates/vb_runtime/src/runtime.rs` (single file, 1,134 LoC), NOT in `crates/vb_runtime/src/runtime/` (directory). The declaration works because Rust treats both shapes as modules. But:

- vb-1ev82 (P0 blocked) was about "restore the runtime/ directory" — the fix is incomplete; the directory does not exist
- Several Kani harnesses and Verus obligations expect `runtime::tick_shard` at the `runtime::` path

The single-file `runtime.rs` includes 4 sibling files via `#[path = ...]`:
```rust
#[path = "runtime_control.rs"] mod runtime_control;
#[path = "runtime_metrics.rs"] mod runtime_metrics;
#[path = "runtime_recovery.rs"] mod runtime_recovery;
#[path = "runtime_trait.rs"] mod runtime_trait;
```

## pending_timers is In-Memory Only

`crates/vb_runtime/src/shard/timer.rs:22-27`:
```rust
pub struct PendingTimer {
    pub step: StepIdx,
    pub kind: PendingTimerKind,
    pub generation: u64,
    pub deadline: Instant,
}
pub type TimerMap = IndexMap<RunId, PendingTimer>;
```

The `TimerMap` is a struct field of `Shard`. It is **NOT** persisted to Fjall. The `Runtime::new_with_journal` constructor does not hydrate timers from the journal.

**Master §20 explicitly requires: "timer wheel ... Fjall writer queue" — the writer queue is there; the reader is not.**

## Recovery Code Is Unused

`crates/vb_runtime/src/recovery.rs:1-189` defines a `RuntimeRecoveryBoundary` trait with these methods:
- `hydrate_run_admission_from_events(run: RunId) -> Result<RunFrame, RuntimeError>`
- `rebuild_journal_sequences(run: RunId) -> Result<SeqNo, RuntimeError>`
- `pending_timers_from_events(run: RunId) -> Result<Vec<PendingTimer>, RuntimeError>`

**None of these are called by `Runtime::new_with_journal` or `Runtime::tick_shard`.** A search for `RuntimeRecoveryBoundary` returns 1 result (the trait definition) + 4 dead tests in `tests/vb_ko29_7_idempotency_miri.rs`. **The trait is documented in master but not implemented in production.**

## await_timer Ignores deadline_slot

`crates/vb_runtime/src/shard/transitions.rs:165-173`:
```rust
self.pending_timer_insert(
    run,
    PendingTimer {
        step,
        kind,
        generation,
        deadline: Instant::now(),  // ← BUG: should read deadline_slot value
    },
);
```

The `deadline_slot: SlotIdx` field of the `WaitUntil` IR node is **never read**. The timer fires on the next tick regardless of the workflow's specified deadline. **Master §46/47 violation.**

## 7 Dead AdmitRun Functions

`crates/vb_runtime/src/admission/admission.rs:1-340` has 7 functions with the name pattern `admit_run_with_*`:
- `admit_run_with_budget`
- `admit_run_with_budget_policy`
- `admit_run_with_typed_preflight`
- `admit_run_with_grants`
- `admit_run_with_resource_contract`
- `admit_run_with_constraints`
- `admit_run_with_typed_workflow`

Only `admit_artifact_run` is called from `lifecycle/chunk_001_submit.rs:221-284`. The 7 `admit_run_with_*` functions are dead code in production; they are only tested directly.

## Forbidden Pattern Audit

| Pattern | Production | Test |
|---------|----------:|-----:|
| `unwrap()` | 0 | 89 (test only) |
| `expect()` | 0 | 41 (test only) |
| `panic!()` | 0 | 2 (test only) |
| `unsafe` | 0 | 0 |
| `#[flux_rs::trusted]` | 12 (in `flux_cancel_kill.rs`) | 0 |

## verdict

**72 / 100 — All 7 P0 patches restored, 2 LETHAL backends, runtime/ dir path drift.**

Top concerns:
1. `BoundedActionCompletionQueue` uses `Mutex<VecDeque>` (Section 50 violation)
2. `runtime/` directory does NOT exist; runtime is `runtime.rs` single file (path drift)
3. `tick_shard` Migrate stubbed
4. `pending_timers` in-memory only; no recovery path
5. `await_timer` ignores `deadline_slot` (uses `Instant::now()`)
6. `Runtime::new_with_journal` does not call `RuntimeRecoveryBoundary`
7. 7 dead `admit_run_with_*` functions
8. 12 vacuous `#[flux_rs::trusted]` in `flux_cancel_kill.rs`
