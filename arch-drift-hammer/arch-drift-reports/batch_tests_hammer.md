# Architectural Drift Report: `batch/tests.rs`

**File:** `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/batch/tests.rs`  
**Line Count:** 711 (EXCEEDS 300-LINE LIMIT BY 411 LINES)  
**Status:** 🚨 REFACTOR REQUIRED

---

## 1. Line Count Violation

| Metric | Value |
|--------|-------|
| Current lines | 711 |
| Limit | 300 |
| Over by | 411 lines (137% violation) |

**Verdict:** File MUST be split into multiple smaller test modules.

---

## 2. Batch Test Responsibilities Map

The file contains **27 tests** exercising `JournalWriteBatch`. Here's the responsibility map:

| Category | Tests | Lines |
|----------|-------|-------|
| Empty batch construction | `new_batch_is_empty_with_zero_length`, `new_batch_from_journal_batch_method_is_empty` | 44-62 |
| Length increment (events) | `len_increments_after_each_append_event` | 64-79 |
| Length increment (header) | `len_increments_after_put_run_header` | 81-90 |
| Length increment (status index) | `len_increments_after_put_status_index` | 92-101 |
| Length increment (workflow index) | `len_increments_after_put_workflow_index` | 103-113 |
| Length increment (action index) | `len_increments_after_put_action_index` | 115-126 |
| Empty commit | `empty_batch_commit_succeeds` | 128-138 |
| Single event commit | `commit_with_single_event_is_readable` | 140-158 |
| Multiple events commit | `commit_with_multiple_events_is_readable` | 160-198 |
| Workflow source commit | `batch_put_workflow_source_with_valid_digest_commits` | 200-222 |
| Compiled IR commit | `batch_put_compiled_ir_with_valid_digest_commits` | 224-244 |
| Run header commit | `batch_put_run_header_commits_and_is_readable` | 246-262 |
| Snapshot commit | `batch_put_snapshot_commits_and_is_readable` | 264-289 |
| Blob commit | `batch_put_blob_with_valid_digest_commits` | 291-311 |
| Strict mode | `batch_strict_mode_commits_successfully` | 313-326 |
| Mixed operations atomic | `batch_mixed_operations_across_keyspaces_commit_atomically` | 328-393 |
| Digest mismatch (workflow source) | `batch_put_workflow_source_rejects_digest_mismatch` | 395-413 |
| Digest mismatch (blob) | `batch_put_blob_rejects_digest_mismatch` | 415-433 |
| Empty strict commit | `empty_strict_batch_commit_succeeds` | 435-445 |
| Index operations len | `batch_index_operations_increment_len_without_payloads` | 447-485 |
| Compiled IR commit (dup) | `batch_put_compiled_ir_commits_and_is_readable` | 487-506 |
| Append event commit | `batch_append_event_commits_and_is_readable` | 508-521 |
| Duplicate event rejection | `batch_append_event_rejects_duplicate_event` | 523-547 |
| Len monotonicity (random ops) | `len_equals_staged_count_after_random_operations` | 549-578 |
| is_empty invariant | `is_empty_equals_len_zero_invariant` | 580-605 |
| Len monotonicity (never decreases) | `batch_len_never_decreases` | 607-633 |
| All-or-nothing commit | `all_or_nothing_commit_across_keyspaces` | 635-672 |
| Digest verification mandatory (workflow) | `digest_verification_mandatory_on_workflow_source` | 674-691 |
| Digest verification mandatory (blob) | `digest_verification_mandatory_on_blob` | 693-710 |

---

## 3. Primitive Obsession Violations

### 3.1 Raw `status: 1` Instead of `RunHeaderStatus`

**Occurrences:** Lines 39, 262, 352, 653

**Problem:** The `make_run_header` helper constructs `RunHeaderRecord` with raw `status: 1` instead of using `RunHeaderStatus::Accepted`.

```rust
// Line 34-42: VIOLATION
fn make_run_header(run: RunId) -> RunHeaderRecord {
    RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(1),
        compiled_digest: WorkflowDigest::from_bytes([0xAB; DIGEST_BYTES]),
        status: 1,  // <-- Primitive obsession: should be RunHeaderStatus::Accepted
        accepted_at_ms: 1000,  // <-- Also primitive: u64 instead of Timestamp
    }
}
```

**Root cause:** `RunHeaderRecord` exposes a `status: u8` field (see `records.rs:258`) with typed accessors `run_header_status()` and `set_run_header_status()`, but the test helper bypasses the type system.

### 3.2 Raw `accepted_at_ms: u64` Instead of Typed Timestamp

**Occurrences:** Lines 40, 353, 653

**Problem:** Millisecond timestamps as raw `u64` without domain typing.

### 3.3 Raw `slots: vec![1, 2, 3]` in `RunSnapshot`

**Occurrences:** Line 273

```rust
// Line 269-275: VIOLATION
let snapshot = RunSnapshot {
    run,
    seq: EventSeq::new(5),
    workflow,
    slots: vec![1, 2, 3],  // <-- Raw Vec<i32> instead of Slots type
    taint: vec![0],         // <-- Raw Vec<u8> instead of TaintFlags type
};
```

### 3.4 Raw `attempt: 1` in JournalEvent Variants

**Occurrences:** Lines 173, 179

```rust
// Lines 169-180: VIOLATION
let e1 = JournalEvent::StepStarted {
    run,
    seq: EventSeq::new(1),
    step: StepIdx::new(0),
    attempt: 1,  // <-- Raw u32 instead of AttemptCount
};
let e2 = JournalEvent::RunFinished {
    run,
    seq: EventSeq::new(2),
    result: SlotIdx::new(0),
    attempt: 1,  // <-- Raw u32 instead of AttemptCount
};
```

### 3.5 Raw `12345` Timestamp in Index Operations

**Occurrences:** Lines 98, 365, 456

```rust
// Line 97-99: VIOLATION
batch
    .put_status_index(IndexStatusState::Submitted, 12345, run)  // <-- 12345 is magic number
    .expect("put status index");
```

### 3.6 `DIGEST_BYTES` Constant Used as Literal `[0xAB; 32]`

**Occurrences:** Lines 38, 168, 268, etc.

The tests use `[0xAB; 32]` hardcoded instead of `[0xAB; DIGEST_BYTES]` for digests.

---

## 4. Scott Wlaschin DDD Violations

| Violation | Location | Description |
|-----------|----------|-------------|
| Primitive obsession | `make_run_header` | Bypasses `RunHeaderStatus` type, uses raw `u8` |
| Primitive obsession | `RunSnapshot` slots/taint | Raw vectors instead of domain types |
| Primitive obsession | `attempt` field | Raw `u32` instead of `AttemptCount` |
| Primitive obsession | `accepted_at_ms` | Raw `u64` instead of `Timestamp` |
| Primitive obsession | Magic timestamp `12345` | Untyped integer in `put_status_index` |
| Missing value object | Status byte | `RunHeaderRecord.status` is `u8` not `RunHeaderStatus` at construction |
| "Parse, don't validate" violation | `make_run_header` | Creates invalid state (status=1 may not equal Accepted) |

---

## 5. Suggested Split Strategy

Split the 711-line file into these modules:

1. **`batch/empty_tests.rs`** (~60 lines) — Empty batch behavior tests
2. **`batch/len_tests.rs`** (~120 lines) — Length increment tests across operations
3. **`batch/commit_event_tests.rs`** (~100 lines) — Event append/commit/replay tests
4. **`batch/commit_record_tests.rs`** (~130 lines) — Record put/commit/read tests (workflow, IR, header, snapshot, blob)
5. **`batch/mixed_atomic_tests.rs`** (~80 lines) — Mixed keyspace atomic commit tests
6. **`batch/digest_rejection_tests.rs`** (~70 lines) — Digest mismatch rejection tests
7. **`batch/index_tests.rs`** (~80 lines) — Index operation tests
8. **`batch/invariant_tests.rs`** (~90 lines) — Monotonicity, is_empty invariant tests

**Total target:** 8 files × ~80 lines average ≈ 640 lines (still over 300 per-file limit for some)

**Better approach:** Further split into ~4 test files per category, each ~30-40 lines.

---

## 6. Priority Fixes

### P0 — MUST FIX (before any merge):

1. **Fix `make_run_header`** to use `RunHeaderStatus::Accepted` instead of raw `status: 1`
2. **Split file** into ≤300 line chunks

### P1 — SHOULD FIX:

3. Add `Slots` and `TaintFlags` newtypes in `RunSnapshot`
4. Add `AttemptCount` newtype for `attempt` field
5. Add `Timestamp` newtype for `accepted_at_ms`

---

## 7. Evidence

```bash
$ wc -l /home/lewis/src/velvet-ballistics/crates/vb_storage/src/batch/tests.rs
711 /home/lewis/src/velvet-ballistics/crates/vb_storage/src/batch/tests.rs
```

**RunHeaderRecord definition (records.rs:246-261):**
```rust
pub struct RunHeaderRecord {
    pub run: RunId,
    pub workflow_id: WorkflowId,
    pub compiled_digest: WorkflowDigest,
    pub status: u8,           // <-- Primitive, with typed accessors
    pub accepted_at_ms: u64,   // <-- Primitive
}
```

**IndexStatusState** is properly typed (`types.rs:229`) but `put_status_index` takes raw timestamp as second argument.

---

**VERDICT:** 🚨 VIOLATION — File at 711 lines exceeds 300-line limit. Primitive obsession in `make_run_header` bypasses the typed `RunHeaderStatus` accessor. Split required before architectural compliance.
