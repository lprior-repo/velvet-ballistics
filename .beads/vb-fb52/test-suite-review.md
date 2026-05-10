# Test Suite Review — vb-fb52

**Bead:** vb-fb52 (Atomic journal and index write batches)
**State:** 10 (Test Review)
**Next Gate:** State 11
**Artifact:** test-suite-review.md
**Review Mode:** Suite Inquisition (Tier 0–3)

---

## VERDICT: REJECTED

### Tier 0 — Static
[PASS] Banned assertion scan — `assert!(result.is_ok())` / `assert!(result.is_err())` pattern not widespread
[PASS] Silent error discard — no `let _ =` or `.ok()` in batch.rs
[PASS] Ignored tests — none found
[PASS] Sleep in tests — none found
[PASS] Naming violations — tests follow `fn batch_*` convention
[FAIL] **Holzmann Rule 2 — 9 `for` loops in test bodies (LETHAL)**
[PASS] Shared mutable state — none found
[PASS] Mock interrogation — no mockall in batch.rs
[PASS] Integration purity — batch.rs uses only public API
[PASS] Error variant completeness — JournalError variants tested with exact assertions
[PASS] Density audit — 78 tests / 14 pub fns = **5.57x (target ≥5x, PASS)**

### Tier 1 — Execution
[PASS] Clippy: batch.rs warnings only (lines 916, 923, 932 — boolean comparators in asserts, not LETHAL)
[PASS] nextest: 179 batch tests passed, 0 failed, 0 flaky
[PASS] nextest: 1026 vb_storage tests passed, 0 failed, 0 flaky
[PASS] Ordering probe: consistent
[N/A] Insta: not present

### Tier 2 — Coverage
[PASS] Line coverage: 97%+ (batch.rs)
[PASS] Branch coverage: 100% (batch.rs)

### Tier 3 — Mutation
[N/A] Skipped due to Tier 0 FAIL

---

## LETHAL FINDINGS

### 1. `crates/vb_storage/src/batch.rs:888` — `for` loop in `len_equals_staged_count_incrementally`
```rust
for (idx, evt) in events.iter().enumerate() {
    batch.append_event(evt).expect("append should succeed");
    let expected_len = idx + 1;
    assert_eq!(batch.len(), expected_len, ...);
}
```
Holzmann Rule 2: "No loops in test bodies. Period." Should use `iter().count()` to assert length directly without explicit iteration.

### 2. `crates/vb_storage/src/batch.rs:957` — `for` loop in `batch_len_never_decreases`
```rust
for evt in &events {
    batch.append_event(evt).expect("append");
    let new_len = batch.len();
    assert!(new_len > prev_len, ...);
    prev_len = new_len;
}
```
Same violation. The loop structure itself is the problem — proving len monotonically increases without looping is possible via a single append + single assertion.

### 3. `crates/vb_storage/src/batch.rs:1205` — `for` loop in `batch_len_after_commit_equals_committed_count`
```rust
for i in 0..7 {
    batch.append_event(&make_event(run, i)).expect("append");
}
```
Hardcoded `0..7` loop. Should use `vec![...; 7]` or `Vec::from_iter((0..7).map(...))` with direct length assertion.

### 4. `crates/vb_storage/src/batch.rs:1217` — `for` loop in `batch_put_status_index_with_various_states`
```rust
for state in 0..4u8 {
    let mut batch = JournalWriteBatch::new(&journal);
    batch.put_status_index(state, 1000 + u64::from(state), run).expect("put status index");
    batch.commit().expect("commit should succeed");
}
```
Parameterized test via loop. Should use `rstest::case` or `criterion` for parameterized testing.

### 5. `crates/vb_storage/src/batch.rs:1229` — `for` loop in `batch_put_workflow_index_multiple_workflows`
```rust
for i in 0..4u32 {
    let workflow_id = vb_core::WorkflowId::new(i + 1);
    ...
}
```
Same parameterized test violation.

### 6. `crates/vb_storage/src/batch.rs:1243` — `for` loop in `batch_put_action_index_multiple_actions`
```rust
for i in 0..4u16 {
    let action_id = vb_core::ActionId::new(i + 1);
    ...
}
```
Same parameterized test violation.

### 7. `crates/vb_storage/src/batch.rs:1474` — `for` loop in event staging
```rust
for evt in &events {
    batch.append_event(evt).expect("append should succeed");
}
```
Loop to append events. Should use `for_each` or batch-append API if available.

### 8. `crates/vb_storage/src/batch.rs:1481` — `for` loop in replay verification
```rust
for (i, evt) in replayed.iter().enumerate() {
    assert_eq!(evt.seq(), EventSeq::new(i as u64));
}
```
Loop to verify ordering. Should use `zip()` + `all()` or single `assert_eq!` on full vec.

### 9. `crates/vb_storage/src/batch.rs:1905` — `for` loop in event staging
```rust
for evt in &events {
    batch.append_event(evt).expect("append should succeed");
}
```
Duplicate of finding #7.

### 10. `crates/vb_storage/src/batch.rs:1403` — Banned assertion without variant check
```rust
let result = batch2.append_event(&event);
assert!(result.is_err());
```
Banned pattern: `assert!(result.is_err())` without asserting the **specific** error variant (`JournalError::DuplicateEvent`). Should be `assert_eq!(result.unwrap_err(), JournalError::DuplicateEvent)`.

### 11. `crates/vb_storage/src/batch.rs:1418` — Banned assertion without variant check
```rust
let result = batch.put_workflow_source(&WorkflowSourceRecord { digest: wrong_digest, ... });
assert!(result.is_err());
```
Same violation — does not assert `JournalError::PayloadDigestMismatch` specifically.

---

## MAJOR FINDINGS (0)

Density now passes (5.57x). No additional MAJOR findings.

---

## MINOR FINDINGS (0)

---

## MANDATE

The following must exist before resubmission:

1. **Remove all 9 `for` loops from test bodies** — Convert to:
   - `iter().count()` for length assertions
   - Single `assert!` with invariant for monotonicity tests
   - `vec![...; N]` or `FromIterator` for fixed-size collections
   - `rstest::case` / parameterized fixtures for multi-variant tests

2. **Replace 2 banned assertions with variant-specific checks**:
   - Line 1403: `assert_eq!(result.unwrap_err(), JournalError::DuplicateEvent)`
   - Line 1418: `assert_eq!(result.unwrap_err(), JournalError::PayloadDigestMismatch)`

---

## EVIDENCE APPENDIX

### Public Functions in JournalWriteBatch (14)
```
new, put_workflow_source, put_compiled_ir, put_run_header,
put_snapshot, put_blob, put_status_index, put_workflow_index,
put_action_index, append_event, len, is_empty, strict, commit
```

### Test Count (78 total in batch.rs)
Density: 78 / 14 = 5.57x ✓

### For Loops in Test Bodies (9 — ALL LETHAL)
| Line | Test Name | Loop Type |
|------|-----------|-----------|
| 888 | `len_equals_staged_count_incrementally` | `iter().enumerate()` |
| 957 | `batch_len_never_decreases` | `for evt in &events` |
| 1205 | `batch_len_after_commit_equals_committed_count` | `for i in 0..7` |
| 1217 | `batch_put_status_index_with_various_states` | `for state in 0..4u8` |
| 1229 | `batch_put_workflow_index_multiple_workflows` | `for i in 0..4u32` |
| 1243 | `batch_put_action_index_multiple_actions` | `for i in 0..4u16` |
| 1474 | (unnamed helper) | `for evt in &events` |
| 1481 | (replay verification) | `for (i, evt) in replayed.iter().enumerate()` |
| 1905 | (unnamed helper) | `for evt in &events` |

### Banned Assertions (2 — ALL LETHAL)
| Line | Test Name | Issue |
|------|-----------|-------|
| 1403 | `batch_duplicate_event_returns_error` | `assert!(result.is_err())` without variant |
| 1418 | `batch_aborted_operations_set_len_to_zero` | `assert!(result.is_err())` without variant |

---

*Bead: vb-fb52 | Reviewer: test-reviewer (Suite Mode) | Date: 2026-05-09*
