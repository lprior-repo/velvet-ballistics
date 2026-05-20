# Domain Model Review: vb-c1s0

## Overview

This document reviews the type and domain model for the orchestration runtime BDD acceptance scenarios. It covers the primary data structures, their invariants, and relationships.

---

## 1. Core Types

### 1.1 Runtime (`vb_runtime::runtime::Runtime`)

```rust
pub struct Runtime {
    shards: Vec<Shard>,
    shard_count: usize,
    journal: SharedRuntimeJournal,
}
```

**Purpose**: Multi-shard container that routes commands to the correct shard based on `RunId % shard_count`.

**Key Invariants**:
- `shards.len() == shard_count` at all times
- `shard_for(run)` is deterministic: same `run` always maps to same `shard`

---

### 1.2 Shard (`vb_runtime::shard::Shard`)

```rust
pub struct Shard {
    config: ShardConfig,
    command_queue: ArrayQueue<ShardCommand>,
    runs: IndexMap<RunId, RunState>,
    timer_wheel: TimerWheel,
    action_queue: BoundedActionCompletionQueue,
    // ... counters, journal, etc.
}
```

**Purpose**: Single-threaded execution context owning mutable run state directly.

**Key Invariants**:
- Commands processed one per `tick()` call
- Each `RunId` maps to exactly one `RunState` in `runs`
- `timer_wheel` and `action_queue` are independent bounded resources

---

### 1.3 RunState (`vb_runtime::shard::types::RunState`)

```rust
pub struct RunState {
    run_id: RunId,
    frame: RunFrame,
    workflow: CompiledWorkflow,
    status: RunStatus,
    pending_action: Option<ActionTicket>,
    pending_ask: Option<AskTicket>,
    pending_timer: Option<PendingTimer>,
    // ... resume context
}
```

**RunStatus Variants**: Admitted, Running, AwaitingAction, AwaitingAsk, AwaitingTimer, Finished, Failed, Cancelled

---

### 1.4 RunFrame (`vb_core::frame::RunFrame`)

```rust
pub struct RunFrame {
    run_id: RunId,
    pc: StepIdx,           // Program counter
    executed: u16,          // Steps executed count
    steps: IndexMap<StepIdx, StepState>,
}
```

**Purpose**: Per-run program counter and step state machine.

**Key Invariants**:
- `pc` always points to a valid step in the workflow or terminal state
- `executed` increments exactly once per step execution

---

### 1.5 ShardCommand (`vb_runtime::shard::types::ShardCommand`)

```rust
pub enum ShardCommand {
    Submit { run, workflow, caps },
    SubmitWithInputs { run, workflow, inputs, caps },
    SubmitWithInputsAndContracts { run, workflow, inputs, caps, action_contracts },
    SubmitWithContracts { run, workflow, caps, action_contracts },
    Resume { run },
    ActionCompleted { ticket, output },
    ActionFailed { ticket, failure },
    RuntimeActionFailed { ticket, failure },
    AskAnswered { answer },
    TimerFired { run, generation, deadline, kind },
    Cancel { run, reason },
    Inspect { run, correlation },
    Shutdown,
}
```

**Purpose**: All external commands that can be delivered to a shard.

---

## 2. Timer System

### 2.1 TimerEntry

```rust
pub struct TimerEntry {
    run: RunId,
    generation: u64,
    deadline: Instant,
    kind: PendingTimerKind,  // Wait or Ask
}
```

**Key Properties**:
- `generation` is a freshness token to prevent stale delivery
- `deadline` orders timer firing
- `kind` distinguishes wait timers from ask timers

### 2.2 TimerWheel

```rust
pub struct TimerWheel {
    by_deadline: BTreeMap<Instant, Vec<TimerEntry>>,  // Time-indexed
    by_run: Map<RunId, TimerEntry>,                   // Run-indexed
}
```

**Operations**:
- `insert(run, deadline, kind)`: O(log n), replaces existing timer for same run
- `cancel(run)`: O(log n)
- `fire_expired(now)`: O(k log n) where k = expired timers
- `next_deadline()`: O(1)

**Key Invariants**:
- `by_run.get(run) == by_deadline entry for same run` (consistent)
- Generation increments monotonically per run
- `fire_expired` only returns entries where `deadline <= now`

### 2.3 PendingTimerKind

```rust
pub enum PendingTimerKind {
    Wait,  // Timer from a Wait step
    Ask,   // Timer from an Ask step
}
```

---

## 3. Action Queue System

### 3.1 BoundedActionCompletionQueue

```rust
pub struct BoundedActionCompletionQueue {
    inner: Mutex<Inner>,
    capacity: usize,
    backpressure_tx: Option<mpsc::Sender<BackpressureWarning>>,
}
struct Inner { items: VecDeque<ActionTicket> }
```

**Key Invariants**:
- `len() <= capacity` always holds
- FIFO ordering via `VecDeque`
- Backpressure emitted when `len() >= capacity * 8 / 10`

### 3.2 ActionQueueError

```rust
pub enum ActionQueueError {
    QueueFull { capacity: usize },
    InvalidCapacity,
}
```

---

## 4. Engine Signals

### 4.1 EngineSignal

```rust
pub enum EngineSignal {
    Continue,
    Finished(SlotValue, Taint),
    AwaitingAction { ticket: ActionTicket },
    AwaitingAsk { ticket: AskTicket },
    AwaitingTimer { pending: PendingTimer },
    StepBudgetExhausted,
}
```

**Purpose**: Step execution outcome driving run state transitions.

**Key Transitions**:
```
Continue -> Continue (next step)
Continue -> Finished (terminal)
Continue -> AwaitingAction (action pending externally)
Continue -> AwaitingAsk (ask pending externally)
Continue -> AwaitingTimer (timer scheduled)
Any -> StepBudgetExhausted (budget depleted)
```

---

## 5. Step Budget

### 5.1 StepBudget

```rust
pub struct StepBudget {
    remaining: AtomicU16,
}
```

**Key Operations**:
- `try_take()`: Decrements if remaining > 0, returns false when exhausted
- `new(n)`: Creates budget for n steps
- `MAX`: Maximum budget (u16::MAX)

---

## 6. Run Lifecycle State Machine

```
                    submit_direct
   +---------> Admitted ---------------------+
   |                                          |
   |              drive_run                   |
   +---------> Running ---------------------->+
   |                |                          |
   |           Continue                        |
   |                v                          |
   |      +----> AwaitingAction               |
   |      |           |                        |
   |      |   complete_action_with_output      |
   |      +-----------+                        |
   |                |                          |
   |      +----> AwaitingAsk                   |
   |      |           |                        |
   |      |      answer_ask                    |
   |      +-----------+                        |
   |                |                          |
   |      +----> AwaitingTimer ----------------+
   |      |           |                        |
   |      |    timer_entry_fired               |
   |      +-----------+                        |
   |                |                          |
   |           Finished                        |
   |              /   \                        |
   |         Failed  Cancelled                 |
   |                                          |
   +------------------------------------------+
                    cancel_run
```

**Terminal States**: Finished, Failed, Cancelled (no further commands processed)

---

## 7. Multi-Shard Routing

### 7.1 Routing Formula

```rust
fn shard_for(&self, run: RunId) -> RuntimeResult<&Shard> {
    let index = run.get() % self.shard_count;
    Ok(&self.shards[index])
}
```

**Properties**:
- Deterministic: same RunId always routes to same shard
- Consistent: modulo arithmetic is stable across runtime lifetime
- Uniform: distribution depends on RunId distribution

---

## 8. BDD Scenario Structure

### 8.1 Scenario Definition

```rust
pub struct Scenario {
    pub id: &'static str,              // e.g., "BDD-KYYF-001"
    pub master_behavior: &'static str,
    pub given: &'static str,           // Preconditions
    pub when: &'static str,            // Action
    pub then: &'static str,            // Postconditions
    pub public_surface: &'static str,  // API surface
    pub fixture: &'static str,
    pub expected_outcome: Option<&'static str>,
    pub expected_error: Option<&'static str>,
    pub durability_profile: &'static str,
    pub related_bead: &'static str,
}
```

---

## 9. Type Review Findings

### 9.1 Correctness

| Type | Finding |
|------|---------|
| `TimerWheel::by_run` | Uses `HashMap` in non-kani builds, `BTreeMap` in kani; both support O(1) lookup by RunId |
| `BoundedActionCompletionQueue` | Uses `std::sync::Mutex` (not tokio); Mutex poisoning handled via `into_inner()` |
| `ShardCommand::Submit` variants | 5 submit variants; `SubmitPrePersisted` is new and may need additional BDD coverage |
| `AskTicket` | Contains both `ask_step` and `resume_step`; ensures ask/resume pairing |

### 9.2 Risk Areas

| Area | Risk | Mitigation |
|------|------|------------|
| Timer generation overflow | u64 bound; `TimerWheelError::GenerationExhausted` handles it | Verus proof for monotonicity |
| Action queue capacity | Fixed at construction; never resized | Capacity invariant proven |
| RunId modulo distribution | Uneven distribution possible | Not a correctness issue |

### 9.3 Missing Coverage (DISCOVERY_BLOCKED)

- Whether `SubmitPrePersisted` command variant has corresponding BDD scenarios
- Whether `vb-kyyf` cross-run determinism scenarios are in scope for vb-c1s0
- Whether new scenario definitions are needed or only existing catalog validation

---

## 10. Relationships

```
Runtime
  └── Shard[n]  (shard_count)
        ├── ArrayQueue<ShardCommand>
        ├── IndexMap<RunId, RunState>
        │     └── RunState
        │           ├── RunFrame
        │           ├── CompiledWorkflow
        │           └── pending { ActionTicket?, AskTicket?, PendingTimer? }
        ├── TimerWheel
        │     └── TimerEntry (by_run, by_deadline)
        └── BoundedActionCompletionQueue
              └── ActionTicket

RunFrame
  └── StepIdx (pc)
  └── IndexMap<StepIdx, StepState>

CompiledWorkflow
  └── Vec<CompiledNode>
  └── entry: StepIdx
```

---

## 11. Verus / Theorem Boundaries

**Verus-expressible (Rust-local pure)**:
- Timer generation monotonicity: `insert()` pre/post conditions
- Queue capacity invariant: `enqueue()` preconditions
- Budget exhaustion: `try_take()` correctness
- Timer matching: `matches_authority()` pure function

**Not Verus-expressible (requires TLA+)**:
- Multi-shard command routing stability
- FIFO command processing per tick
- Terminal state reachability and uniqueness
- Liveness: every non-terminal run eventually reaches terminal or continues

**Theorem kernel (potential Lean extraction)**:
- Algebraic proof of timer wheel O(log n) bounds (if needed beyond Verus)
