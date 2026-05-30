# Architectural Drift Report: `shard/types.rs`

**File**: `crates/vb_runtime/src/shard/types.rs`  
**Line Count**: 858 (violates 300-line hard limit)  
**Status**: CRITICAL REFACTOR REQUIRED

---

## Executive Summary

This file violates the `<300 line` hard limit by **286%** (858 lines). It also contains multiple **primitive obsession** violations and violates **single responsibility** by cramming 15+ distinct type families into a single file. The file must be split.

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 858 | 300 | **VIOLATION** |
| Overage | +558 | 0 | **286% of limit** |

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 `FramePoolKey` — Naked Tuple (Line 27)

```rust
type FramePoolKey = (u16, u16);
```

**Violation**: Tuple used as a composite key without domain wrapping.  
**Scott Wlaschin Rule**: "Make illegal states unrepresentable" — `(u16, u16)` permits any combination of u16 values; a domain newtype could enforce invariants (e.g., first u16 = slot index, second = generation).  
**Refactor**: Create `FramePoolKey(u16, u16)` as a tuple struct with accessors.

### 2.2 `correlation` — Raw `u64` (Lines 181, 270, 288)

```rust
pub correlation: u64,
```

**Violation**: `u64` used for a correlation identifier without domain semantics.  
**Refactor**: Create `CorrelationId(u64)` newtype with `Parse, don't validate` semantics.

### 2.3 `next_epoch` — Raw `u64` (Line 363)

```rust
next_epoch: u64,
```

**Violation**: Raw integer used for epoch counter in `IntrospectionRegistry`.  
**Refactor**: Create `Epoch(u64)` newtype.

### 2.4 `timestamp` — Raw `u64` (Line 806)

```rust
pub timestamp: u64,
```

**Violation**: Raw integer for resume timestamp.  
**Refactor**: Create `ResumeTimestamp(u64)` or use `std::time::Instant` appropriately.

### 2.5 `reason` — Raw `Option<String>` (Lines 167, 174)

```rust
reason: Option<String>,
```

**Violation**: `String` for cancellation/kill reason is primitive obsession. Reasons have domain meaning (e.g., `Cancelled`, `Killed`, `TimedOut`).  
**Refactor**: Create `CancellationReason` enum with variants like `Requested`, `PolicyViolation`, `ResourceExhaustion`, etc.

### 2.6 `encoded_len` — Raw `u32` (Line 210)

```rust
pub encoded_len: u32,
```

**Violation**: `u32` for encoded length without bounds encoding.  
**Refactor**: Consider `EncodedLen(u32)` or a bounded type that validates against max frame size.

### 2.7 Capacity/Budget Primitives — Lines 507-514, 688

```rust
pub const MAX_COMMAND_QUEUE_CAPACITY: usize = 65_536;
pub fn is_valid_command_queue_capacity(capacity: usize) -> bool
pub step_budget_per_tick: u64,
```

**Violation**: `usize` and `u64` used directly for capacity/budget values. These should be wrapped in domain types with validation.  
**Refactor**: Create `CommandQueueCapacity`, `StepBudget`, `TraceCapacity` newtypes with bounded construction.

---

## 3. SINGLE RESPONSIBILITY VIOLATIONS

The file contains **15+ distinct type families** that must be separated:

| Type(s) | Lines | Suggested File |
|---------|-------|----------------|
| `PendingTimerKind`, `PendingTimer` | 29-54 | `shard/timer.rs` |
| `ShardCommand` (god enum, 15 variants) | 57-185 | `shard/command.rs` |
| `AskTicket`, `AskAnswer` | 188-243 | `shard/ask.rs` |
| `RunState` | 246-262 | `shard/run_state.rs` |
| `InspectSnapshot`, `InspectResponse` | 265-290 | `shard/inspect.rs` |
| `UnregisterOutcome`, `RegisterOverlapOutcome` | 297-317 | `shard/introspection.rs` |
| `InspectHandle`, `IntrospectionRegistry`, `InspectSnapshotFormatter` | 322-505 | `shard/introspection.rs` |
| `ShardCommandQueue` | 526-618 | `shard/command_queue.rs` |
| `Shard`, `ShardStatus`, `ShardHealth` | 621-678 | `shard/shard.rs` |
| `ShardConfig` | 682-717 | `shard/config.rs` |
| `RuntimeState`, `RuntimeEvent` | 720-783 | `shard/runtime_state.rs` |
| `ResumeStatus`, `ResumeResult`, `ResumeError` | 786-858 | `shard/resume.rs` |

---

## 4. GOD ENUM ANTIPATTERN

### `ShardCommand` — 15 Variants (Lines 57-185)

```rust
pub enum ShardCommand {
    Submit { run: RunId, workflow: CompiledWorkflow, caps: CapabilitySet },
    SubmitPrePersisted { ... },
    SubmitWithInputs { ... },
    SubmitWithContracts { ... },
    SubmitWithInputsAndContracts { ... },
    Resume { run: RunId },
    ActionCompleted { ticket, output },
    ActionCompletedLegacy { run, step },
    ActionFailed { ticket, failure },
    RuntimeActionFailed { ticket, failure },
    AskAnswered { answer },
    TimerFired { run, generation, deadline, kind },
    Cancel { run, reason },
    Kill { run, reason },
    Inspect { run, correlation },
    Shutdown,
}
```

**Violation**: This is a classic god enum. Each variant has different fields and semantics.  
**Scott Wlaschin Rule**: "One type per key behavior" — these should be separate command types in a module hierarchy:
- `shard/commands/submit.rs` — Submit variants
- `shard/commands/resume.rs` — Resume, ActionCompleted, etc.
- `shard/commands/lifecycle.rs` — Cancel, Kill, Shutdown
- `shard/commands/inspect.rs` — Inspect

### `RuntimeState` / `RuntimeEvent` State Machine (Lines 720-783)

The `RuntimeStateMachine` is implicit (only `RuntimeEvent::is_terminal()` and `is_resumable()` methods exist). This should be an explicit state machine with transitions as functions.

---

## 5. ADDITIONAL DDD CONCERNS

### 5.1 `RunState::action_attempts` — Raw Slice

```rust
pub action_attempts: Box<[u16]>,
```

**Concern**: `Box<[u16]>` for action attempt counters. Could be `ActionAttemptCounters(Vec<u16>)` or similar domain type.

### 5.2 `IntrospectionRegistry` Uses `std::collections::HashMap`

**Line 362**:
```rust
inner: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<RunId, u64>>>,
```

**Concern**: Raw `HashMap` instead of a domain-typed equivalent (e.g., `EpochMap<RunId, Epoch>`).

### 5.3 Missing Value Objects

No value objects for:
- `StepBudget` — currently raw `u64`
- `TraceCapacity` — currently raw `usize`
- `MaxActiveRuns` — currently raw `usize`

---

## 6. RECOMMENDED REFACTOR MAP

```
shard/types.rs (858 lines)
├── shard/mod.rs          — Module declarations
├── shard/primitives.rs   — Newtypes: FramePoolKey, CorrelationId, Epoch, CancellationReason, etc.
├── shard/timer.rs         — PendingTimerKind, PendingTimer
├── shard/command.rs      — ShardCommand enum
├── shard/command_queue.rs — ShardCommandQueue
├── shard/ask.rs          — AskTicket, AskAnswer
├── shard/run_state.rs    — RunState
├── shard/inspect.rs      — InspectSnapshot, InspectResponse, InspectHandle, IntrospectionRegistry
├── shard/shard.rs        — Shard, ShardStatus, ShardHealth
├── shard/config.rs       — ShardConfig
└── shard/resume.rs       — RuntimeState, RuntimeEvent, ResumeStatus, ResumeResult, ResumeError
```

---

## 7. MANDATORY ACTIONS

1. **SPLIT FILE**: Decompose into 10+ modules per the map above
2. **CREATE NEWtypes**: Wrap all primitive obsessions listed in Section 2
3. **EXPLODE GOD ENUM**: Split `ShardCommand` into command module hierarchy
4. **ADD DOMAIN VALIDATORS**: Replace raw capacity/budget functions with `TryFrom` implementations on newtypes
5. **UPDATE MOD.RS**: Ensure `pub mod shard;` exports all new submodules

---

## 8. EVIDENCE COMMANDS

```bash
# Verify line count
wc -l crates/vb_runtime/src/shard/types.rs
# Expected: <300

# Verify no primitive obsession in new types
rg 'type FramePoolKey' crates/vb_runtime/src/shard/
rg 'pub correlation: u64' crates/vb_runtime/src/shard/
```

---

**VERDICT**: `shard/types.rs` is a **CRITICAL ARCHITECTURAL DRIFT** artifact. The 858-line monolith must be decomposed before any new work lands on this codebase. No exceptions.
