# Architectural Drift Report: `chunk_001.rs`

**File**: `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs`
**Size**: 404 lines (**VIOLATION**: exceeds 300-line limit by 104 lines)
**Enforcer**: arch-drift-hammer
**Date**: 2026-05-29

---

## 1. FILE SIZE VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 404 | 300 | ❌ VIOLATION (+104) |
| Methods | ~25 | — | — |
| Blank lines | ~30 | — | — |
| Doc comments | ~15 | — | — |

---

## 2. RESPONSIBILITY MAPPING

The `impl Shard` block violates Single Responsibility Principle. It conflates **8 distinct responsibilities**:

| # | Responsibility | Methods | Concern |
|---|---------------|---------|---------|
| 1 | **Construction** | `new`, `new_with_journal`, `new_with_journal_and_artifact_store` | Object creation and DI |
| 2 | **Command Queue Ops** | `enqueue`, `command_queue_len`, `remaining_capacity`, `is_queue_full`, `command_queue_capacity` | Queue state inspection |
| 3 | **Tick/Dispatch** | `tick` | Command routing and processing |
| 4 | **Journal Management** | `append_journal_event`, `journal_sequence_for`, `advance_journal_sequence`, `discard_journal_sequence` | Durable event persistence |
| 5 | **Timer Authority** | `timer_fired_command`, `timer_entry`, `pending_timer_count` | Timer creation and capture |
| 6 | **Run Lifecycle** | `active_run_count`, `snapshot_run`, `status` | Run state observation |
| 7 | **Trace/Evidence** | `trace_ring_mut`, `trace_ring`, `flush_evidence`, `flush_evidence_event`, `flush_step_started`, `flush_step_succeeded` | Observability and evidence chain |
| 8 | **Shutdown/Status** | `is_shutting_down`, `take_inspect_response`, `status` | Lifecycle state |

### Evidence of SRP Violation
- `flush_evidence_event` alone handles 3 event types with divergent logic paths
- `tick` method is a 70-line god match arm handling 15+ command variants
- Mixing `RuntimeResult<()>`, `Option<T>`, and direct state mutation across concerns

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### Critical: Unwrapped Raw Primitives in Public APIs

| Line | Signature/Code | Primitive | Should Be |
|------|---------------|-----------|-----------|
| 308 | `snapshot_run(&self, run: RunId, correlation: u64)` | `u64` | `CorrelationId(RawCorrelationId)` |
| 346 | `trace_capacity: self.trace_ring.capacity()` | `usize` | `Capacity<TraceRing>` |
| 348 | `step_budget_per_tick: self.step_budget_per_tick` | `usize` | `StepBudget` |
| 343 | `command_queue_capacity: self.command_queue.capacity()` | `usize` | `Capacity<CommandQueue>` |
| 342 | `command_queue_depth: self.command_queue.len()` | `usize` | `QueueDepth` |
| 275 | `generation: 0` | raw literal | `TimerGeneration::ZERO` or `Generation(0)` |
| 276 | `deadline: std::time::Instant::now()` | `Instant` | `Deadline(Instant)` |
| 385 | `flush_step_started(&mut self, run: RunId, step: StepIdx)` | `StepIdx` | Already typed (good) |
| 392 | `output: Option<SlotIdx>` | `Option<SlotIdx>` | Already typed (good) |

### Internal Primitive Obsession

| Line | Code | Primitive | Issue |
|------|------|-----------|-------|
| 143-147 | `seq.get().checked_add(1).map(EventSeq::new)` | Raw `u64` arithmetic | Overflow math exposed; should be `seq.next()` on `EventSeq` |
| 165-170 | `free = free.saturating_add(...)` | `usize` arithmetic | `frame_pool_metrics()` returns raw tuple instead of `FramePoolMetrics` struct |
| 309-316 | Match on `self.runs.get(&run)` | Raw `RunId` lookup | Could use `RunState` query object |
| 333 | `let shutting_down = self.shutting_down` | `bool` | `ShuttingDown(ShutdownState)` would type-flag the state |

---

## 4. SCOTT WLASCHIN DDD VIOLATIONS

### A. Feature Envy (Anti-Corruption Layer Missing)

```rust
// Line 308-317: Shard reads its own internal IndexMap and builds InspectResponse manually
pub fn snapshot_run(&self, run: RunId, correlation: u64) -> InspectResponse {
    match self.runs.get(&run) {
        Some(state) => InspectResponse::Found(crate::shard::helpers::snapshot_from_state(...)),
        None => InspectResponse::NotFound { run, correlation },
    }
}
```
**Problem**: `Shard` is both a domain object AND a data structure. The snapshot logic leaks into the domain object. Should have a `RunStateSnapshot` value object built by a query/reader.

### B. Primitive Obsession on Command Construction

```rust
// Line 272-279: Raw timer command construction with primitives
pub fn timer_fired_command(&self, run: RunId) -> ShardCommand {
    ShardCommand::TimerFired {
        run,
        generation: 0,           // VIOLATION: raw u32
        deadline: Instant::now(), // VIOLATION: raw Instant
        kind: PendingTimerKind::Wait,
    }
}
```
**Problem**: Timer authority is fabricated with zero-generation and wall-clock deadline. A `TimerFiredCommand` factory with typed `TimerGeneration` and `Deadline` would enforce invariants.

### C. Primitive Obsession on Status

```rust
// Line 332-351: Raw primitive soup for status
pub fn status(&self) -> ShardStatus {
    ShardStatus {
        health: if shutting_down { ShardHealth::ShuttingDown } else { ShardHealth::Running },
        running: !shutting_down,                        // bool
        shutting_down,                                   // bool
        command_queue_depth: self.command_queue.len(),  // usize
        command_queue_capacity: self.command_queue.capacity(), // usize
        active_runs: self.runs.len(),                   // usize
        max_active_runs: self.max_active_runs,          // usize
        trace_capacity: self.trace_ring.capacity(),     // usize
        trace_dropped: self.trace_ring.dropped(),       // usize
        step_budget_per_tick: self.step_budget_per_tick,// usize
        runtime_policy: self.policy,                    // Already typed (good)
    }
}
```
**Problem**: `ShardStatus` is a Anemic Data Transfer Object with 10 raw primitive fields. Should be `RuntimeStatus` with value objects: `QueueDepth`, `Capacity`, `TraceDroppedCount`, `StepBudget`.

### D. Temporal Coupling in Journal Sequence

```rust
// Line 135-150: Journal sequence advancement requires manual ordering
fn journal_sequence_for(&self, run: RunId) -> EventSeq {
    self.journal_sequences.get(&run).copied().unwrap_or(EventSeq::ZERO)
}

fn advance_journal_sequence(&mut self, run: RunId, seq: EventSeq) -> RuntimeResult<()> {
    let next = seq.get().checked_add(1)...  // VIOLATION: raw u64 math
    self.journal_sequences.insert(run, next);
    Ok(())
}
```
**Problem**: `EventSeq` wraps u64 but forces callers to unwrap and do arithmetic. Should have `EventSeq::next(&self) -> Option<EventSeq>` on the value object itself.

### E. God Method: `tick`

```rust
// Line 175-255: 80-line tick() matches 15 command variants
pub fn tick(&mut self) -> RuntimeResult<bool> {
    if self.shutting_down { return Ok(false); }
    let Some(cmd) = self.command_queue.pop() else { return Ok(true); };
    match cmd {
        ShardCommand::Submit { run, workflow, caps } => self.handle_submit(...)?,
        // ... 13 more arms, some delegating to other handlers
    }
    Ok(true)
}
```
**Problem**: `tick` is a command dispatcher violating Command pattern. Should be `CommandDispatcher` or `ShardCommandHandler` that routes to typed handlers. Each `handle_*` method is an implicit command handler that should be extracted.

---

## 5. AFFINITY CLUSTERS (Suggested Refactor Boundaries)

```
impl Shard should be split into:

┌─────────────────────────────────────────────────────────┐
│  Shard (composition root / wiring only)                 │
│  - new(), new_with_journal() constructors               │
│  - coordinates other components                         │
└─────────────────────────────────────────────────────────┘
          │              │              │
          ▼              ▼              ▼
┌─────────────┐  ┌─────────────┐  ┌─────────────────┐
│ ShardQueue  │  │ ShardJournal│  │ ShardScheduler  │
│ - enqueue   │  │ - append    │  │ - tick dispatch │
│ - tick_pop  │  │ - sequence  │  │ - handle_submit │
│ - metrics   │  │ - advance   │  │ - handle_timer  │
└─────────────┘  └─────────────┘  └─────────────────┘
                                          │
                    ┌─────────────────────┼─────────────────────┐
                    ▼                     ▼                     ▼
           ┌─────────────┐       ┌─────────────┐       ┌─────────────┐
           │ TimerWheel  │       │ RunRegistry │       │ EvidenceMgr │
           │ - timers    │       │ - runs map  │       │ - flush_*   │
           │ - schedule  │       │ - snapshot  │       │ - trace     │
           └─────────────┘       └─────────────┘       └─────────────┘
```

---

## 6. ACTION ITEMS

| Priority | Finding | Fix |
|----------|---------|-----|
| P0 | 404 lines (limit 300) | Split into `chunk_001_queue.rs`, `chunk_001_journal.rs`, `chunk_001_dispatch.rs` |
| P0 | `u64` correlation in `snapshot_run` | Create `CorrelationId(RawCorrelationId(u64))` value object |
| P1 | `tick` god method | Extract `CommandDispatcher` trait + typed handlers |
| P1 | `ShardStatus` anemic DTO | Refine to `RuntimeStatus` with `QueueDepth`, `Capacity`, `StepBudget` value objects |
| P1 | `EventSeq::next()` missing | Add method to value object, hide raw u64 arithmetic |
| P2 | `usize` everywhere for capacities | Wrap in `Capacity<T>`, `QueueDepth`, `TraceDropped` types |
| P2 | `Instant` deadline in `timer_fired_command` | Use `Deadline(Instant)` typed wrapper |
| P2 | Frame pool metrics returns `(usize, usize)` | Return `FramePoolMetrics { free, total }` struct |

---

## 7. VERDICT

**ARCHITECTURAL DRIFT: CONFIRMED**

This file exhibits severe drift:
- **File size**: 35% over limit (404/300)
- **SRP violations**: 8 responsibilities jammed into one impl block
- **Primitive obsession**: 10+ raw primitive usages in public API
- **DDD violations**: Anemic DTOs, feature envy, temporal coupling, god method

**RECOMMENDATION**: Refactor before any additional feature work. The shard impl must be decomposed into role-based submodules with proper value object wrapping.
