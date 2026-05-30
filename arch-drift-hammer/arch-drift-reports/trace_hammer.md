# Architectural Drift Report: `trace.rs`

**File**: `crates/vb_runtime/src/trace.rs`  
**Total Lines**: 1380  
**Limit**: 300 lines  
**Violation**: 460% of limit — MANDATORY REFACTOR

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 1380 | 300 | 🔴 VIOLATION |
| Implementation | ~290 | 200 | 🔴 VIOLATION |
| Test code | ~1000 | 100 | 🔴 VIOLATION |

---

## 2. RESPONSIBILITY MAP

### Current Module: `trace.rs`

| Symbol | Type | Responsibility | Violations |
|--------|------|----------------|------------|
| `TraceRing` | Struct | SPSC ring buffer + history management | SRP, 2 concerns |
| `TraceEvent` | Enum | All trace event variants (12 total) | God enum, Primitive obsession |
| `TraceRing::new` | Constructor | Ring initialization | — |
| `TraceRing::push` | Method | Event insertion with overflow tracking | Mixed concerns |
| `TraceRing::drain` | Method | Batch consumption | — |
| `TraceRing::drain_into` | Method | Bounded drain to provided vec | — |
| `TraceRing::drain_for_run` | Method | Filtered drain by RunId | Primitive `limit` |
| `TraceRing::snapshot_for_run` | Method | Non-destructive filtered read | Primitive `limit` |
| `TraceRing::has_terminal_event_for_run` | Method | Terminal state detection | Primitive `inspected` counter |
| `TraceEvent::run_id` | Method | RunId extractor | Cross-cutting match |
| `TraceEvent::is_terminal_for_run` | Method | Terminal predicate | Cross-cutting match |

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### 3.1 `TraceEvent::SlotWritten { value: Vec<u8> }`

**Violation**: Raw byte vector carrying "postcard-encoded SlotValue"

```rust
SlotWritten {
    run: RunId,
    slot: SlotIdx,
    value: Vec<u8>,  // ← PRIMITIVE OBSESSION
}
```

**Problem**: `Vec<u8>` is untyped bytes. The comment says "postcard-encoded SlotValue" but the type system does not enforce this. Callers can pass any bytes.

**Fix**: Create `EncodedSlotValue(Vec<u8>)` newtype OR use the actual `SlotValue` type if available.

---

### 3.2 `TraceRing::dropped: u64`

**Violation**: Raw counter without domain semantics

```rust
pub struct TraceRing {
    producer: rtrb::Producer<TraceEvent>,
    consumer: rtrb::Consumer<TraceEvent>,
    capacity: usize,   // ← PRIMITIVE OBSESSION
    dropped: u64,       // ← PRIMITIVE OBSESSION
    history: VecDeque<TraceEvent>,
}
```

**Fix**: Create `DroppedCount(u64)` and `Capacity(usize)` newtypes.

---

### 3.3 Loop Counters: `inspected: usize`, `drained: usize`

**Violation**: Raw `usize` counters in private methods

```rust
pub fn drain_for_run(&mut self, target: RunId, limit: usize) -> Vec<TraceEvent> {
    let bounded_limit = limit.min(self.capacity);
    let mut events = Vec::with_capacity(bounded_limit);
    let mut inspected = 0usize;  // ← PRIMITIVE OBSESSION
    while inspected < bounded_limit { ... }
}
```

**Fix**: `InspectedCount(usize)`, `DrainedCount(usize)`, `Limit(usize)` newtypes.

---

## 4. GOD ENUM VIOLATION

### `TraceEvent` has 12 variants — THREE distinct domains

```
Run-level events:    RunSubmitted, RunFinished, RunFailed, RunCancelled, RunKilled
Step-level events:   StepStarted, StepEnded
Action-level events: ActionScheduled, ActionCompleted, ActionFailed
Slot-level events:   SlotWritten, AskAnswered
```

**Problem**: All 12 variants lumped into one enum. A change to slot-level events requires touching run-level event code. No boundary between domains.

**Fix**: Create type hierarchy:

```rust
// trace/events/mod.rs
pub mod run_events { RunSubmitted, RunFinished, RunFailed, RunCancelled, RunKilled }
pub mod step_events { StepStarted, StepEnded }
pub mod action_events { ActionScheduled, ActionCompleted, ActionFailed }
pub mod slot_events { SlotWritten, AskAnswered }

// Or use a trait objects / sealed trait pattern
```

---

## 5. SINGLE RESPONSIBILITY PRINCIPLE VIOLATION

### `TraceRing` mixes TWO concerns

1. **SPSC ring buffer** — `producer`, `consumer` from `rtrb`
2. **History management** — `history: VecDeque<TraceEvent>`

```rust
pub struct TraceRing {
    producer: rtrb::Producer<TraceEvent>,   // Concern 1: SPSC
    consumer: rtrb::Consumer<TraceEvent>,   // Concern 1: SPSC
    capacity: usize,
    dropped: u64,
    history: VecDeque<TraceEvent>,           // Concern 2: History
}
```

**Problem**: `push()` does TWO things:
1. Push to ring buffer
2. Remember in history

**Fix**: Extract `TraceHistory` as separate struct, or use composition.

---

## 6. TEST CODE BLOATE

### Metrics

| Section | Lines | Limit | Status |
|---------|-------|-------|--------|
| `#[cfg(test)]` block | ~1000 | 100 | 🔴 VIOLATION |

### Duplicative Test Patterns

Many tests assert the same invariants repeatedly:
- `trace_event_run_id_returns_correct_run_for_all_variants` (lines 406-427)
- `trace_event_run_id_all_variants` (lines 633-657) — **DUPLICATE**

**Fix**: Extract shared test utilities into a `trace/testing` module.

---

## 7. RECOMMENDED REFACTORING PLAN

### Phase 1: File Split (Reduce to <300 lines each)

| New File | Contents | Est. Lines |
|----------|----------|------------|
| `trace/events.rs` | `TraceEvent` enum + methods | ~120 |
| `trace/ring.rs` | `TraceRing` struct + impl | ~200 |
| `trace/history.rs` | `TraceHistory` (extracted) | ~80 |
| `trace/types.rs` | Newtypes (`Capacity`, `DroppedCount`, `EncodedSlotValue`, etc.) | ~60 |
| `trace/tests.rs` | Integration tests only | ~150 |
| `trace/mod.rs` | Module re-exports | ~20 |

### Phase 2: Primitive Obsession Fixes

```rust
// trace/types.rs
pub struct Capacity(usize);
pub struct DroppedCount(u64);
pub struct EncodedSlotValue(Vec<u8>);
pub struct Limit(usize);
pub struct InspectedCount(usize);
```

### Phase 3: Event Hierarchy

```rust
// trace/events/run.rs
pub enum RunEvent { Submitted, Finished, Failed, Cancelled, Killed }

// trace/events/step.rs
pub enum StepEvent { Started { step: StepIdx }, Ended { step: StepIdx } }

// trace/events/mod.rs
pub enum TraceEvent {
    Run(RunEvent),
    Step(StepEvent),
    Action(ActionEvent),
    Slot(SlotEvent),
}
```

### Phase 4: History Extraction

Extract `VecDeque<TraceEvent>` management into `TraceHistory` with its own tests.

---

## 8. SUMMARY

| Violation | Severity | Fix Complexity |
|-----------|----------|----------------|
| 1380 lines (>300) | 🔴 CRITICAL | High (file split required) |
| Primitive `Vec<u8>` in `SlotWritten` | 🔴 CRITICAL | Medium (newtype) |
| Primitive `u64`, `usize` counters | 🔴 CRITICAL | Medium (newtypes) |
| God enum (12 variants) | 🔴 CRITICAL | High (hierarchy) |
| `TraceRing` dual responsibility | 🟡 MODERATE | Medium (extract History) |
| Test bloat (~1000 lines) | 🟡 MODERATE | Low (move to tests.rs) |

**Verdict**: MANDATORY REFACTOR. File exceeds line limit by 460%. All three critical violations must be addressed. Start with file split to bring under 300 lines, then fix primitives, then address god enum.
