# Architectural Drift Report: `crates/vb_storage/src/batch.rs`

**File**: `crates/vb_storage/src/batch.rs`
**Total Lines**: 1066
**Line Limit**: 300
**Violation**: **CRITICAL — 355% over budget**

---

## Executive Summary

`batch.rs` is a **1066-line god object** that violates:
1. **<300 line rule** (355% over)
2. **Scott Wlaschin DDD** — primitive obsession, implicit state, mixed concerns
3. **Single Responsibility Principle** — handles 9 distinct record types

---

## 1. Line Count Violation

| Section | Lines | Status |
|---------|-------|--------|
| Implementation (`JournalWriteBatch`) | 1–260 | 260 lines |
| Tests | 261–1066 | **806 lines** |
| **TOTAL** | | **1066 lines** |

**Required Action**: Move tests to `crates/vb_storage/tests/batch_tests.rs`. The 806-line test block is **80% of the file**.

---

## 2. Batch Operations Map

`JournalWriteBatch` exposes **9 distinct write operations**:

| Method | Record Type | Keyspace | Validates Digest? |
|--------|-------------|----------|-------------------|
| `put_workflow_source` | `WorkflowSourceRecord` | `workflow_source` | ✅ Yes |
| `put_compiled_ir` | `CompiledIrRecord` | `compiled_ir` | ❌ No |
| `put_run_header` | `RunHeaderRecord` | `run_header` | ❌ No |
| `put_snapshot` | `RunSnapshot` | `run_snapshot` | ❌ No |
| `put_blob` | `BlobRecord` | `blob` | ✅ Yes |
| `put_status_index` | marker | `index_status` | ❌ N/A |
| `put_workflow_index` | marker | `index_workflow` | ❌ N/A |
| `put_action_index` | marker | `index_action` | ❌ N/A |
| `append_event` | `JournalEvent` | `events` | ❌ No |

---

## 3. Primitive Obsession Violations

### 3.1 `timestamp: u64` — Line 169

```rust
pub fn put_status_index(
    &mut self,
    state: crate::types::IndexStatusState,
    timestamp: u64,   // ❌ PRIMITIVE OBSESSION
    run: vb_core::RunId,
) -> Result<(), JournalError>
```

**Problem**: `u64` for timestamp has no domain meaning. Is this milliseconds? Seconds? Unix epoch?

**Fix**: Create `TimestampMs(u64)` newtype in `types.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct TimestampMs(u64);

impl TimestampMs {
    pub const fn new(ms: u64) -> Self { Self(ms) }
    pub const fn get(self) -> u64 { self.0 }
}
```

### 3.2 `accepted_at_ms: u64` — `RunHeaderRecord` (records.rs:260)

```rust
pub struct RunHeaderRecord {
    pub run: RunId,
    pub workflow_id: WorkflowId,
    pub compiled_digest: WorkflowDigest,
    pub status: u8,           // ❌ Uses raw u8, should be RunHeaderStatus
    pub accepted_at_ms: u64,  // ❌ PRIMITIVE OBSESSION
}
```

**Problem**: Same issue — milliseconds as raw `u64`.

**Fix**: Replace with `TimestampMs` or create `UnixEpochMs(u64)`.

### 3.3 `status: u8` — `RunHeaderRecord` (records.rs:258)

```rust
pub status: u8,  // ❌ Should use RunHeaderStatus newtype
```

**Problem**: Already has `RunHeaderStatus` newtype (lines 13, 44-82 in records.rs) but the struct field uses raw `u8`.

**Note**: The code does have accessor methods (`run_header_status()`, `set_run_header_status()`) but the field is public `u8`.

### 3.4 Magic number `0` in `encode_record` calls

```rust
// Line 83: flags parameter is raw 0
encode_record(MAGIC_WORKFLOW_SOURCE, RecordKind::WorkflowSource, 0, record, MAX_WORKFLOW_SOURCE_BYTES)

// Line 103, 117, 131: same pattern
encode_record(MAGIC_COMPILED_ARTIFACT, RecordKind::CompiledIr, 0, ...)
```

**Problem**: `0` passed as "flags" has no type safety. Should be a typed flag struct.

---

## 4. DDD Violations (Scott Wlaschin)

### 4.1 God Object — `JournalWriteBatch`

`JournalWriteBatch` handles **4 distinct bounded contexts**:
1. **Workflow domain** — `put_workflow_source`, `put_compiled_ir`
2. **Run domain** — `put_run_header`, `put_snapshot`, `append_event`
3. **Blob domain** — `put_blob`
4. **Index domain** — `put_status_index`, `put_workflow_index`, `put_action_index`

**DDD Principle Violated**: Each bounded context should have its own batch or command object.

### 4.2 Implicit State — `aborted: bool` (Line 44)

```rust
pub struct JournalWriteBatch<'j> {
    aborted: bool,  // ❌ IMPLICIT STATE MACHINE
    // ...
}
```

**Problem**: The batch has implicit states that are not modeled as a proper state machine.

**Fix**: Model as explicit `BatchState` enum:
```rust
pub enum BatchState {
    Open,
    Aborted { reason: JournalError },
    Committed,
}
```

### 4.3 Inconsistent Validation Pattern

| Method | Error Handling |
|--------|---------------|
| `put_workflow_source` | Sets `aborted = true` on error |
| `put_blob` | Sets `aborted = true` on error |
| `put_compiled_ir` | Returns error, does NOT set `aborted` |
| `put_run_header` | Returns error, does NOT set `aborted` |
| `append_event` | Sets `aborted = true` on duplicate |

**Problem**: `aborted` flag is set inconsistently. Some methods set it, others don't.

### 4.4 `Parse, Don't Validate` Not Followed

Each `put_*` method does **both** validation AND encoding together:

```rust
pub fn put_workflow_source(&mut self, record: &WorkflowSourceRecord) -> Result<(), JournalError> {
    // 1. Validate digest
    if let Err(e) = verify_content_digest(&record.source, &record.digest.as_bytes()) {
        self.aborted = true;
        return Err(e);
    }
    // 2. Encode key
    let key = workflow_source_key(record.digest.as_bytes())?;
    // 3. Encode value
    let value = encode_record(...)?;
    // 4. Insert
    self.inner.insert(...);
    Ok(())
}
```

**DDD Principle**: Should separate:
- `Key = parse_key(record)?` — returns `Result<Key, Error>`
- `Value = encode_value(record)?` — pure encoding
- `Batch::insert(key, value)` — side effect

---

## 5. File Structure Violations

### 5.1 Tests Inline (806 lines)

Lines 261–1066 are all tests. Per workspace rules, tests should be in `crates/vb_storage/tests/`.

### 5.2 Multiple Modules in One File

This file contains:
- Batch implementation (260 lines)
- No actual module declarations for sub-components

---

## 6. Positive Elements (Don't Break)

These are correctly implemented and should be preserved:

| Element | Location | Assessment |
|---------|----------|------------|
| `EventSeq` newtype | `types.rs:68-94` | ✅ Correct u64 wrapper |
| `IndexStatusState` enum | `types.rs:222-262` | ✅ Proper state enum with `from_u8`/`to_u8` |
| `RunHeaderStatus` newtype | `records.rs:6-133` | ✅ Proper newtype for status byte |
| `StorageKey` enum | `types.rs:264-301` | ✅ Covers all key variants |
| `JournalWriteBatch::strict()` | Line 246 | ✅ Builder pattern for durability |
| `PhantomData<*mut FjallJournal>` | Line 45 | ✅ Correct `!Send + !Sync` enforcement |

---

## 7. Required Refactors

### Phase 1: Extract Tests (Non-Negotiable)
```
crates/vb_storage/tests/batch_tests.rs  (806 lines)
```

### Phase 2: Create Missing Newtypes
```rust
// types.rs — add:
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct TimestampMs(u64);

impl TimestampMs {
    pub const fn new(ms: u64) -> Self { Self(ms) }
    pub const fn get(self) -> u64 { self.0 }
}
```

### Phase 3: Fix `RunHeaderRecord`
```rust
// records.rs — change:
pub accepted_at_ms: u64,
// TO:
pub accepted_at_ms: TimestampMs,
```

### Phase 4: Fix `put_status_index` Signature
```rust
pub fn put_status_index(
    &mut self,
    state: IndexStatusState,
    timestamp: TimestampMs,  // Not u64
    run: RunId,
) -> Result<(), JournalError>
```

### Phase 5: Extract Sub-Batches (Future Consideration)
Consider splitting `JournalWriteBatch` into domain-specific batches:
- `WorkflowWriteBatch` — workflow_source, compiled_ir
- `RunWriteBatch` — run_header, snapshot, events
- `BlobWriteBatch` — blob
- `IndexWriteBatch` — status, workflow, action indices

This is a larger refactor and requires `vb_storage` architecture review.

---

## 8. Severity Assessment

| Violation | Severity | Effort to Fix |
|------------|----------|---------------|
| Line count (1066 vs 300) | **CRITICAL** | Low (move tests) |
| Primitive obsession: timestamp u64 | **HIGH** | Medium (newtype + fix callsites) |
| Primitive obsession: accepted_at_ms | **HIGH** | Medium (newtype + fix callsites) |
| God object: JournalWriteBatch | **MEDIUM** | High (architectural change) |
| Implicit aborted state | **MEDIUM** | Medium (state enum) |
| Inconsistent aborted flag | **MEDIUM** | Low (normalize) |
| Tests inline | **HIGH** | Low (move to tests/) |

---

## 9. Immediate Actions

1. **MOVE TESTS** to `crates/vb_storage/tests/batch_tests.rs` — removes 806 lines
2. **CREATE** `TimestampMs` newtype in `types.rs`
3. **UPDATE** `put_status_index` to use `TimestampMs`
4. **UPDATE** `RunHeaderRecord.accepted_at_ms` to use `TimestampMs`
5. **AUDIT** all call sites of these functions for downstream impact

**Estimated reduction**: 806 (tests) + ~20 (newtypes) = **240 lines remaining** — well under 300.

---

## 10. Conclusion

**STATUS: REFACTOR REQUIRED**

This file is a **structural time-bomb**. The 1066-line monolith:
- Violates the <300 line rule by 355%
- Exposes raw primitives where domain types exist
- Mixes 4 bounded contexts in one god object
- Has inconsistent error handling via implicit `aborted` flag

The good news: **80% of the violations are in the test block**. Moving tests to `tests/` solves the immediate crisis. The remaining newtype work is straightforward.

**Do not merge new code to this file until tests are extracted.**
