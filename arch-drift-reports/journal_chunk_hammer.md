# ARCHITECTURAL DRIFT REPORT: journal/chunk_001.rs

**File:** `crates/vb_runtime/src/journal/chunk_001.rs`  
**Line Count:** 394 (VIOLATION: exceeds 300-line limit by 94 lines)  
**Status:** REFACTOR REQUIRED

---

## 1. LINE COUNT VIOLATION

| File | Lines | Limit | Excess |
|------|-------|-------|--------|
| `chunk_001.rs` | 394 | 300 | +94 |

**Root Cause:** This file conflates three distinct DDD layers:
- **Domain Events** (`RuntimeJournalEvent` enum)
- **Port/Trait** (`RuntimeJournal` trait + implementations)
- **Value Objects / Config** (`RuntimeJournalConfig`)

These should be split into separate files along responsibility boundaries.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### VIOLATION 1: `reason: Option<String>` (line 45)
```rust
RunCancelled {
    run: RunId,
    reason: Option<String>,  // <-- PRIMITIVE
},
```

**Problem:** `String` is a primitive. Cancellation reasons have semantic meaning and should be modeled as a domain type.

**Fix:** Create `CancellationReason` newtype:
```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CancellationReason(String);

impl CancellationReason {
    pub fn new(s: impl Into<String>) -> Self { Self(s.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}
```

---

### VIOLATION 2: `timestamp: u64` (line 171)
```rust
Resumed {
    run: RunId,
    timestamp: u64,  // <-- PRIMITIVE (seconds since epoch)
},
```

**Problem:** Raw `u64` for a timestamp has no domain semantics. Is it milliseconds? Seconds? Epoch-relative?

**Fix:** Create `EpochSeconds` or `MonotonicTimestamp` newtype:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EpochSeconds(u64);

impl EpochSeconds {
    pub fn now() -> Self { Self(std::time::SystemTime::now().duration_since(...).unwrap().as_secs()) }
    pub fn get(self) -> u64 { self.0 }
}
```

---

### VIOLATION 3: `attempt: u16` (lines 103, 164, etc.)
```rust
ActionFailed {
    run: RunId,
    step: StepIdx,
    action: ActionId,
    attempt: u16,  // <-- PRIMITIVE
},
```

**Problem:** `u16` for attempt count is untyped. No bounds, no validation.

**Fix:** Create `AttemptCount` newtype:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct AttemptCount(u16);

impl AttemptCount {
    pub fn new(n: u16) -> Self { Self(n.saturating_add(1)) }
    pub fn get(self) -> u16 { self.0 }
}
```

---

### VIOLATION 4: `capacity: usize` (line 270)
```rust
pub struct VolatileRuntimeJournal {
    events: Mutex<Vec<RuntimeJournalEvent>>,
    capacity: usize,  // <-- PRIMITIVE
}
```

**Problem:** `usize` for capacity with no bounds checking.

**Fix:** Already partially mitigated by `NonZeroUsize` in `with_capacity()`, but the field type should be a dedicated `JournalCapacity` newtype.

---

### VIOLATION 5: `encoded_len: u32` (line 88)
```rust
ActionCompletedEnvelope {
    ticket: ActionTicket,
    output: SlotIdx,
    value: Vec<u8>,
    encoded_len: u32,  // <-- PRIMITIVE
    taint: Taint,
    value_digest: [u8; 32],
},
```

**Problem:** `u32` for byte length with no validation that it matches `value.len()`.

**Fix:** Create `EncodedLen(u32)` with a `Validate` trait bound.

---

## 3. MISSING DOMAIN TYPES (Implicit Primitives)

### 3a. `value: Vec<u8>` without domain marker
The `Vec<u8>` in `ActionCompletedEnvelope.value` and `SlotWritten.value` is untyped bytes. Should be:
```rust
pub struct EncodedSlotValue(Vec<u8>);
pub struct EncodedActionOutput(Vec<u8>);
```

### 3b. `extra: Option<Vec<u8>>` without domain marker
```rust
SlotWritten {
    ...
    extra: Option<Vec<u8>>,  // <-- untyped
},
```

---

## 4. RESPONSIBILITY CONFLATION

### Current structure in chunk_001.rs (394 lines):

| Lines | Responsibility | Should Be |
|-------|----------------|-----------|
| 13-201 | `RuntimeJournalEvent` enum (domain events) | `journal/events.rs` |
| 203-234 | `RuntimeJournal` trait (port) | `journal/port.rs` |
| 240-264 | `NoopRuntimeJournal` (impl) | `journal/noop.rs` |
| 267-388 | `VolatileRuntimeJournal` (impl) | `journal/volatile.rs` |
| 327-363 | `RuntimeJournalConfig` (value object) | `journal/config.rs` |
| 391-394 | `StorageRuntimeJournal` struct only | Already in chunk_002 |

**Problem:** `StorageRuntimeJournal` struct is declared in chunk_001 but its impl is in chunk_002 — this is a cross-file split violation.

---

## 5. REQUIRED REFACTORING

### Proposed File Split:

```
journal/
├── mod.rs          # Re-exports
├── events.rs       # RuntimeJournalEvent + domain newtypes (NEW)
├── port.rs         # RuntimeJournal trait (NEW)
├── noop.rs         # NoopRuntimeJournal (NEW)
├── volatile.rs     # VolatileRuntimeJournal (NEW)
├── config.rs       # RuntimeJournalConfig (NEW)
├── chunk_001.rs    # STUB - re-exports from above (REFACTORED)
├── chunk_002.rs    # StorageRuntimeJournal impl + QueuedStorageRuntimeJournal (UNCHANGED)
└── chunk_003.rs    # QueuedStorageRuntimeJournal RuntimeJournal impl (UNCHANGED)
```

### Chunk 001 Refactor (target: <300 lines):
1. Move `RuntimeJournalEvent` to `events.rs` with newtype wrappers
2. Move `RuntimeJournal` trait to `port.rs`
3. Move `NoopRuntimeJournal` to `noop.rs`
4. Move `VolatileRuntimeJournal` to `volatile.rs`
5. Move `RuntimeJournalConfig` to `config.rs`
6. Replace chunk_001.rs content with re-exports

---

## 6. JURISDICTION NOTE

This file is part of the `vb_runtime` crate. The following domain newtypes should ideally live in `vb_core` (not `vb_runtime`) since they are shared domain vocabulary:

- `CancellationReason`
- `EpochSeconds` / `MonotonicTimestamp`
- `AttemptCount`
- `JournalCapacity`
- `EncodedLen`
- `EncodedSlotValue`
- `EncodedActionOutput`

However, since `vb_runtime` cannot depend on `vb_core` in a way that creates circular deps, the newtypes should be defined in `vb_runtime` first, then promoted to `vb_core` if the domain model is generalized.

---

## 7. SUMMARY

| Category | Count |
|----------|-------|
| Line Count Violations | 1 (394 > 300) |
| Primitive Obsession Violations | 5 |
| Missing Domain Types | 7+ |
| Responsibility Conflations | 1 (struct/impl split across files) |
| Files Needing Refactor | 1 (chunk_001.rs) |

**ENFORCEMENT STATUS:** `REFACTOR REQUIRED`

---

*Generated by: architectural-drift agent*  
*Target: velvet-ballistics/vb_runtime*  
*Date: 2026-05-29*
