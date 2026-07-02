# Contract Specification: vb-c1s0

## Context

- **Bead**: vb-c1s0
- **Title**: bdd: Orchestration runtime acceptance scenarios
- **Source Checkout**: /home/lewis/src/velvet-ballistics
- **Isolated Workspace**: /home/lewis/src/vb-c1s0-workspace
- **State**: Go-skill State 3 (Contract/Proof Planning)

## Domain Terms

| Term | Definition |
|------|------------|
| `Runtime` | Multi-shard orchestration container routing commands to shards |
| `Shard` | Single-threaded execution context owning mutable run state |
| `RunId` | Unique identifier for a workflow execution instance |
| `RunFrame` | Per-run program counter and step state machine |
| `CompiledWorkflow` | Immutable workflow plan with nodes, constants, expressions |
| `EngineSignal` | Step execution outcome: Continue, Finished, AwaitingAction, AwaitingAsk, AwaitingTimer, StepBudgetExhausted |
| `ShardCommand` | Envelope for submit/resume/cancel/action completion/timer/ask operations |
| `TimerEntry` | Timer with generation, deadline, kind (Wait/Ask), owned by a RunId |
| `TimerWheel` | Dual-index timer structure (by_deadline BTreeMap + by_run HashMap) |
| `BoundedActionCompletionQueue` | Thread-safe bounded FIFO queue with 80% backpressure warning |
| `ActionTicket` | Opaque handle for action completion delivery |
| `AskTicket` | Resume proof for ask/answer lifecycle |
| `StepBudget` | Atomic step-count budget for deterministic execution |

## Assumptions

- **ASM-001**: All shards operate single-threadedly; no intra-shard locking required
- **ASM-002**: RunId → Shard routing via `run_id.get() % shard_count` is deterministic and consistent
- **ASM-003**: Journal events provide the canonical replay evidence chain
- **ASM-004**: Timer wheel generation arithmetic is bounded by u64::MAX
- **ASM-005**: BoundedActionCompletionQueue capacity is fixed at construction and never changes
- **ASM-006**: Every action ticket enqueued corresponds to an AwaitingAction signal from the engine
- **ASM-007**: `drive_deterministic` loop exits only on Continue, non-Continue EngineSignal, or budget exhaustion

## Open Questions

- **DISCOVERY_BLOCKED**: Whether vb-c1s0 defines NEW BDD scenarios or validates EXISTING catalog scenarios (BDD-KYYF-001 through BDD-NJJU-004, VB-BDD-CATALOG-001 through 010)
- **DISCOVERY_BLOCKED**: Whether additional scenario scheduling beyond ActionScheduled/WaitScheduled/AskScheduled exists for compound workflows

---

## Preconditions

### PRE-001: Runtime Construction
- `Runtime::new(shard_count, config)` requires `shard_count > 0`
- `Runtime::new_with_journal` additionally requires a valid `SharedRuntimeJournal`

### PRE-002: Submit Admission
- `Runtime::submit_direct(run, workflow)` requires:
  - `run` is a valid `RunId` (non-zero inner value)
  - `workflow` is a valid `CompiledWorkflow` with at least one node
  - The shard for `run` must accept the submission (admission validation)

### PRE-003: Action Completion
- `Runtime::complete_action_with_output(ticket, output)` requires:
  - `ticket` was previously issued by a suspended step (exists in BoundedActionCompletionQueue)
  - `output` carries valid `SlotValue`

### PRE-004: Timer Entry Firing
- `Runtime::timer_entry_fired(entry)` requires:
  - `entry` carries generation, deadline, and kind matching the currently pending timer for `entry.run`
  - `entry.deadline <= now` (timer has expired)

### PRE-005: Shard Tick Invariant
- `Shard::tick()` requires no concurrent access to the same shard

---

## Postconditions

### POST-001: Submit
- `submit_direct` returns `Ok(())` iff admission validation passes
- On success, a `Submit` command is enqueued to the correct shard's command queue
- On failure, no command is enqueued and an error variant is returned

### POST-002: Run Lifecycle Terminal States
- A run reaches terminal state `Finished`, `Failed`, `Cancelled`, or `Skipped` exactly once
- No further commands are processed for a terminal run

### POST-003: Action Completion Routing
- `complete_action_with_output` delivers output to the exact `RunFrame` and `StepIdx` identified by `ticket`
- After delivery, the run resumes from the step following the suspended step

### POST-004: Timer Authority Handoff
- `capture_timer_entry` emits a `TimerEntry` with the current generation
- `timer_entry_fired` validates generation before advancing the run
- Stale timer deliveries (generation mismatch) are discarded silently

### POST-005: tick_all Progress
- `tick_all` processes at most one command per shard per call
- Returns `false` if any shard is shutting down; `true` otherwise

### POST-006: Action Queue Backpressure
- When `BoundedActionCompletionQueue::enqueue` would exceed 80% capacity, a `BackpressureWarning` is emitted
- `QueueFull` error is returned when capacity is reached

---

## Invariants

### INV-001: RunId Shard Consistency
- For any `RunId run`, `runtime.shard_for(run)` always returns the same shard for the lifetime of the runtime
- Formula: `runtime.shard_for(run).eq(runtime.shard_for(run))` (idempotent)

### INV-002: Timer Generation Monotonicity
- For any `RunId run`, successive timer insertions increment the generation
- Formula: `insert(run, g1); insert(run, g2) => g2 = g1 + 1`

### INV-003: No Phantom Timer Delivery
- `timer_entry_fired(entry)` only fires if `entry` matches the current timer state (generation, deadline, kind)
- Mismatched entries are ignored without error

### INV-004: Action Queue FIFO Ordering
- `dequeue` returns tickets in the same order they were `enqueue`d
- No ticket is lost unless explicitly dequeued

### INV-005: Bounded Queue Capacity
- `BoundedActionCompletionQueue` never exceeds its configured capacity
- `len() <= capacity` holds at all times

### INV-006: Budget Exhaustion Safety
- `drive_deterministic` exits with `StepBudgetExhausted` exactly when `budget.try_take()` returns `false`
- No steps execute beyond the budget

### INV-007: Shard Command Processing
- Each `tick_all` call processes at most one command per shard
- Commands are processed in queue insertion order (FIFO per shard)

---

## Error Taxonomy

### RuntimeError Variants

| Variant | Condition |
|---------|-----------|
| `RuntimeError::ShardNotFound` | No shard owns the given RunId |
| `RuntimeError::RunNotFound` | The run does not exist on the shard |
| `RuntimeError::RunAlreadyExists` | A run with the same RunId is already active |
| `RuntimeError::AdmissionRejected` | Capability or contract validation failed |
| `RuntimeError::InvalidTicket` | ActionTicket does not correspond to a pending action |
| `RuntimeError::StaleTimer` | TimerEntry generation does not match pending timer |
| `RuntimeError::ShardShuttingDown` | The shard is not accepting new commands |
| `RuntimeError::Internal` | Unexpected internal error (boxed) |

### ActionQueueError Variants

| Variant | Condition |
|---------|-----------|
| `ActionQueueError::QueueFull { capacity }` | Enqueue would exceed bounded capacity |
| `ActionQueueError::InvalidCapacity` | Constructor received zero capacity |

### TimerWheelError Variants

| Variant | Condition |
|---------|-----------|
| `TimerWheelError::GenerationExhausted` | u64 overflow on generation increment |

### EngineError Variants

| Variant | Condition |
|---------|-----------|
| `EngineError::StepIndexOutOfBounds` | StepIdx exceeds workflow node count |
| `EngineError::SlotIndexOutOfBounds` | SlotIdx exceeds slot count |
| `EngineError::BudgetExhausted` | StepBudget reached zero |

---

## Contract Signatures

```rust
// Runtime submission
pub fn submit_direct(&self, run: RunId, workflow: CompiledWorkflow) -> RuntimeResult<()>
pub fn submit_direct_with_grants(&self, run, workflow, caps) -> RuntimeResult<()>
pub fn submit_direct_with_inputs_grants_and_contracts(&self, run, workflow, inputs, caps, contracts) -> RuntimeResult<()>

// Lifecycle
pub fn cancel_run(&self, run: RunId) -> RuntimeResult<()>
pub fn resume_run(&self, run: RunId) -> RuntimeResult<()>

// Action completion
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

// Shard tick
pub fn tick_shard(&mut self, shard_index: usize) -> RuntimeResult<ShardDirective>
pub fn migrate_shard(&mut self, run: RunId, target_shard: usize) -> RuntimeResult<()>

// Action queue
pub fn enqueue(&self, ticket: ActionTicket) -> Result<(), ActionQueueError>
pub fn dequeue(&self) -> Option<ActionTicket>
pub fn remaining_capacity(&self) -> usize

// Timer wheel
pub fn insert(&mut self, run: RunId, deadline: Instant, kind: PendingTimerKind) -> Result<(), TimerWheelError>
pub fn fire_expired(&mut self, now: Instant) -> Vec<TimerEntry>
```

---

## TLA+-Owned Clauses

- **INV-001**: RunId shard consistency (temporal: routing is deterministic and stable)
- **INV-007**: Shard command FIFO processing per tick
- **POST-002**: Run terminal state uniqueness and reachability
- **Workflow progression**: Actions, timers, asks lead to eventual resumption or terminal state

See `tla-spec.md` for full TLA+ model.

---

## Verus-Owned Clauses

- **INV-002**: Timer generation monotonicity (pure arithmetic, no I/O)
- **INV-003**: No phantom timer delivery (generation/deadline/kind match validation)
- **INV-004**: Action queue FIFO ordering (deterministic queue operations)
- **INV-005**: Bounded queue capacity (capacity enforcement invariant)
- **INV-006**: Budget exhaustion safety (deterministic step counting)

---

## Theorem-Owned Clauses

None at this time. Timer wheel arithmetic is bounded by u64 and expressible in Verus. If u64 overflow is a future concern, a Lean kernel projection may be added.

---

## Non-Goals

- Proof of external system integration (CLI, storage backend)
- Performance regression proof (benchmark evidence only)
- FFI or unsafe code contracts (unsafe code is forbidden in vb_runtime)
- Formal proof of BDD scenario coverage (Fowler tests provide behavioral coverage)
