# Architectural Drift Report: `chunk_002.rs`

**File:** `crates/vb_runtime/src/journal/chunk_002.rs`
**Line Count:** 357 (VIOLATION: exceeds 300-line limit by 57 lines)
**Workspace:** `/home/lewis/src/velvet-ballistics/arch-drift-hammer`
**Status:** `VIOLATION_DETECTED`

---

## 1. Line Count Violation

| Metric | Value |
|--------|-------|
| Actual lines | 357 |
| Maximum allowed | 300 |
| Excess | **57 lines (19% over)** |

The file MUST be split. The two logical responsibilities (StorageRuntimeJournal impl + QueuedStorageRuntimeJournal impl) are cleanly separable.

---

## 2. Responsibilities Map

### 2.1 `StorageRuntimeJournal` (impl blocks: lines 1–265)

**Purpose:** Adapter bridging `RuntimeJournalEvent` (runtime-local, in-memory) to `JournalEvent` (durable, storage-native) with two durability profiles.

**Sub-responsibilities:**
| Responsibility | Lines | Issue |
|---|---|---|
| Factory constructors (`journaled`, `strict`, shared variants) | 1–30 | Duplicated across both types |
| `append_storage_event` (dispatch to Fjall) | 32–39 | Primitive dispatch on `DurabilityProfile` enum |
| `run_storage_event` (Runtime→Journal for run lifecycle) | 41–101 | Hardcoded `attempt: 1`; information loss |
| `action_storage_event` (Runtime→Journal for action lifecycle) | 103–179 | Same hardcoded `attempt: 1` |
| `boundary_storage_event` (Runtime→Journal for wait/ask/slot) | 181–247 | `RuntimeResult<Option<...>>` overengineering |
| `storage_event` (top-level router) | 249–264 | Sequential if-let cascade instead of match |
| `RuntimeJournal` impl | 276–296 | Trivial passthrough |

### 2.2 `QueuedStorageRuntimeJournal` (impl blocks: lines 298–357)

**Purpose:** Queued wrapper that stages events in `JournalWriterQueue` before durable flush.

**Sub-responsibilities:**
| Responsibility | Lines | Issue |
|---|---|---|
| Factory constructors (4 variants) | 305–342 | Identical duplication to StorageRuntimeJournal |
| `flush_batch` | 345–349 | Trivial delegation |
| `drain_all` | 351–356 | Trivial delegation |

### 2.3 Standalone utility

| Function | Lines | Issue |
|---|---|---|
| `encoded_slot_taint_extra` | 267–274 | Thin wrapper; `Option<Vec<u8>>` is primitive obsession |

---

## 3. Primitive Obsession Violations

### 3.1 `attempt: 1` — Hardcoded Magic Constant (CRITICAL)

**Every** branch in `run_storage_event`, `action_storage_event`, and `boundary_storage_event` passes the literal `1` as the attempt number:

```
Line  44: attempt: 1,  // RunAccepted
Line  47: attempt: 1,  // RunAdmission
Line  53: attempt: 1,  // RunFinished
Line  56: attempt: 1,  // RunFinished
Line  59: attempt: 1,  // RunFailedEvent
Line  62: attempt: 1,  // RunFailedEvent
Line  64: attempt: 1,  // RunCancelled
Line  67: attempt: 1,  // RunCancelled
Line  70: attempt: 1,  // RunKilled
Line  73: attempt: 1,  // RunKilled
Line  75: attempt: 1,  // StepStarted
Line  78: attempt: 1,  // StepStarted
Line  83: attempt: 1,  // StepSucceeded
Line  88: attempt: 1,  // StepSucceeded
Line 106: attempt: 1,  // ActionScheduled
Line 111: attempt: 1,  // ActionScheduled
Line 114: attempt: 1,  // ActionCompletedEvent
Line 120: attempt: 1,  // ActionCompletedEvent
Line 127: attempt: 1,  // ActionScheduledTicket
Line 133: attempt: 1,  // ActionScheduledTicket
Line 141: attempt: 1,  // ActionCompletedEnvelope
Line 147: attempt: 1,  // ActionCompletedEnvelope
Line 157: attempt: 1,  // ActionFailedEvent
Line 162: attempt: 1,  // ActionFailedEvent
Line 187: attempt: 1,  // WaitScheduledEvent
Line 191: attempt: 1,  // WaitScheduledEvent
Line 195: attempt: 1,  // RetryScheduledEvent
Line 199: attempt: 1,  // RetryScheduledEvent
Line 203: attempt: 1,  // AskScheduledEvent
Line 207: attempt: 1,  // AskScheduledEvent
Line 211: attempt: 1,  // AskAnsweredEvent
Line 215: attempt: 1,  // AskAnsweredEvent
Line 224: attempt: 1,  // SlotWrittenEvent
Line 230: attempt: 1,  // SlotWrittenEvent
Line 258: attempt: 1,  // RunFailedEvent (fallback)
Line 261: attempt: 1,  // RunFailedEvent (fallback)
```

**Violation:** `u16` is used directly instead of a named type (e.g., `AttemptCount` or `Attempt`). This is a loss of semantic information — `1` carries no meaning about *which* attempt this represents.

**Fix:** Define `type AttemptNumber = u16;` and use a named constant `const DEFAULT_ATTEMPT: AttemptNumber = 1;` or extract from source events where available.

---

### 3.2 `Option<Vec<u8>>` for `extra` (MEDIUM)

Lines 229, 269, 270: The function `encoded_slot_taint_extra(taint: Taint, extra: Option<Vec<u8>>)` uses `Option<Vec<u8>>` as a raw byte buffer. This conflates three concepts:
- "extra is absent" → `None`
- "extra is present and empty" → `Some(vec![])`
- "extra is present and non-empty" → `Some(bytes)`

`Vec<u8>` is a primitive byte array with no domain semantics. The domain concept is **EncodedSlotExtra** — a wrapper that makes the encoding explicit and non-optional (the Option belongs at the call site, not inside the encoding).

---

### 3.3 Raw `i32`/`usize` not observed but worth flagging

`EventSeq` (from vb_storage) is a type alias. If it is `u64` or `NonZeroUsize`, that's acceptable. However, `encoded_len: u32` in `ActionCompletedEnvelope` (line 138) is a raw `u32`. It should be `ByteLength(u32)` or `EncodedLen(u32)` as a newtype.

---

### 3.4 `DurabilityProfile` enum used as boolean discriminant (LOW)

Lines 33–37:

```rust
let result = if self.profile == DurabilityProfile::Strict {
    self.journal.append_strict(event)
} else {
    self.journal.append_journaled(event)
};
```

This is `Parse, don't validate` violated — the enum is pattern-matched as a binary flag. If a third variant (`Async`, `WriteBehind`, etc.) were added, this would silently fall through to the `else` branch. Prefer exhaustive match or a dedicated strategy pattern.

---

## 4. Structural / DDD Violations

### 4.1 Duplicated Factory Pattern (DRY)

`StorageRuntimeJournal` and `QueuedStorageRuntimeJournal` both implement an identical 4-variant factory pattern:

| Variant | StorageRuntimeJournal | QueuedStorageRuntimeJournal |
|---|---|---|
| `journaled` | lines 4–8 | lines 308–313 |
| `strict` | lines 13–17 | lines 318–323 |
| `shared_journaled` | lines 22–24 | lines 328–332 |
| `shared_strict` | lines 28–30 | lines 337–342 |

These should be consolidated into a shared `JournalBuilder` or `RuntimeJournalFactory`.

---

### 4.2 Sequential If-Let Cascade Instead of Exhausted Match

Lines 249–264:

```rust
fn storage_event(event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<JournalEvent> {
    if let Some(storage_event) = Self::run_storage_event(event.clone(), seq) {
        return Ok(storage_event);
    }
    if let Some(storage_event) = Self::action_storage_event(event.clone(), seq) {
        return Ok(storage_event);
    }
    match Self::boundary_storage_event(event.clone(), seq)? {
        Some(storage_event) => Ok(storage_event),
        None => Ok(JournalEvent::RunFailedEvent { ... }),
    }
}
```

**Issue:** This is not an explicit state machine. The three-phase dispatch (run → action → boundary) is implicit ordering knowledge. A caller cannot determine from the type system which events produce which variants.

**Scott Wlaschin:** Workflows should be explicit state transitions. Here, `RuntimeJournalEvent → JournalEvent` is a closed catalog of 30+ transformations encoded as three separate match statements with implicit ordering. This should be a single `fn convert(event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<JournalEvent>` with one exhaustive match.

---

### 4.3 Information Loss in Fallback (Line 258–262)

```rust
None => Ok(JournalEvent::RunFailedEvent {
    run: event.run_id(),
    seq,
    attempt: 1,
}),
```

When `boundary_storage_event` returns `None`, the code manufactures a `RunFailedEvent`. This is a **silent error suppression** — an unhandled event variant is converted to a failure without logging or returning an error. This masks the fact that `boundary_storage_event` returned `None` for an event that should have been handled.

---

## 5. Refactoring Plan

### Split 1: `journal/chunk_002_storage.rs` (~180 lines)

Contains `StorageRuntimeJournal` — factory constructors, `append_storage_event`, and the three `*_storage_event` helper functions. Further split by extracting a `convert_event` helper that does the run→action→boundary dispatch in one exhaustive match.

### Split 2: `journal/chunk_002_queued.rs` (~60 lines)

Contains `QueuedStorageRuntimeJournal` — factory constructors, `flush_batch`, `drain_all`.

### Split 3: `journal/chunk_002_shared.rs` (~30 lines)

Contains the shared utility `encoded_slot_taint_extra` and ideally the shared factory pattern extraction.

### NewType Recommendations

| Raw Type | NewType | Location |
|---|---|---|
| `u16` (attempt) | `AttemptNumber` | `vb_core::journal` or `vb_runtime::journal` |
| `u32` (encoded_len) | `EncodedByteLen` | `vb_storage::events` |
| `Option<Vec<u8>>` (extra) | `EncodedSlotExtra` | `vb_storage::events` |
| `DurabilityProfile` dispatch | `StorageStrategy` enum + `append_with_strategy` | `journal/chunk_002_storage.rs` |

---

## 6. Verdict

| Check | Result |
|---|---|
| Line count ≤ 300 | **FAIL** (357 lines) |
| No primitive obsession | **FAIL** (attempt: 1, Vec<u8>) |
| Explicit state transitions | **PARTIAL** (sequential if-let cascade) |
| Parse don't validate | **FAIL** (DurabilityProfile as binary flag) |
| DRY (factory pattern) | **FAIL** (duplicated across both types) |
| Error handling non-silent | **FAIL** (fallback RunFailedEvent masks None) |

**STATUS: REFACTOR_REQUIRED**
