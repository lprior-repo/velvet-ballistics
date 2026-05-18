# Test Review — Round 2, A5–A8 (Mode 2: Suite Inquisition)

## Target Files
- `crates/workspace_tests/tests/slot_written_ordering_integration_tests.rs`
- `crates/vb_cli/tests/ui_command_tests.rs`
- `crates/vb_storage/src/journal/journal_event_tests.rs`

---

## VERDICT: REJECTED

---

### Tier 0 — Static Analysis

**[FAIL] Banned pattern scan** — `slot_written_ordering_integration_tests.rs:790`
- `assert!(result.is_ok())` / `assert!(result.is_err())` — NOT FOUND in A5–A8
- `let _ = | .ok();` — NOT FOUND in A5–A8
- `#[ignore]` — NOT FOUND in A5–A8
- `sleep` — NOT FOUND in A5–A8

**[FAIL] Compilation gate** — LETHAL
- `slot_written_ordering_integration_tests.rs` **FAILS TO COMPILE**

**[PASS] Determinism/evidence scan**
- No `static mut`, `lazy_static!`, `once_cell.*Mutex` found in target files

**[PASS] Mock interrogation**
- No `mockall` patterns found in target files

**[PASS] Integration test purity**
- No `use crate::` private module access found in A5–A8

**[PASS] Error variant completeness**
- `JournalError` variants covered exhaustively in `journal_event_tests.rs`

**[PASS] Density audit**
- 18 tests in `journal_event_tests.rs` covering all 18 `JournalEvent` variants
- 10 tests in `slot_written_ordering_integration_tests.rs` covering B-1/B-2/B-3
- 11 tests in `ui_command_tests.rs` covering CLI argument parsing + validation

---

### Tier 1 — Execution

**[FAIL] Test compile: FAILED**

```
error[E0532]: cannot match against a tuple struct which contains private fields
   --> crates/workspace_tests/tests/slot_written_ordering_integration_tests.rs:790:23
    |
790 |                 slot: SlotIdx(1),
    |                       ^^^^^^^
note: constructor is not visible here due to private fields
   --> crates/workspace_tests/tests/slot_written_ordering_integration_tests.rs:790:31
    |
790 |                 slot: SlotIdx(1),
    |                               ^ private field
```

**Root cause**: `SlotIdx` is a `#[repr(transparent)]` tuple struct with a private `u16` field (defined via `numeric_id!` macro in `vb_core/src/ids/mod.rs:56`). Pattern-matching `SlotIdx(1)` to extract the inner value requires access to private fields.

**Fix**: Replace the private-field pattern with a guard clause:
```rust
// BROKEN (line 790):
JournalEvent::SlotWrittenEvent {
    seq,
    slot: SlotIdx(1),  // ❌ private field
    ..
} if seq.get() == 5

// FIXED:
JournalEvent::SlotWrittenEvent {
    seq,
    slot,
    ..
} if seq.get() == 5 && slot.get() == 1
```

**[SKIP] nextest execution** — Cannot run due to compile failure

**[SKIP] Ordering probe** — Cannot run due to compile failure

**[SKIP] Insta staleness** — Insta not detected in A5–A8

---

### Tier 2 — Coverage

**[SKIP]** — Cannot run due to compile failure

---

### Tier 3 — Mutation

**[SKIP]** — Cannot run due to compile failure

---

## LETHAL FINDINGS

1. **`slot_written_ordering_integration_tests.rs:790`** — `cannot match against a tuple struct which contains private fields` — `SlotIdx(1)` pattern is illegal because `SlotIdx`'s inner `u16` field is private
   - **Impact**: `snapshot_plus_tail_replays_tail_slot_writes_after_snapshot` test cannot compile
   - **Fix required**: Change `slot: SlotIdx(1)` to `slot` with a guard `&& slot.get() == 1`

---

## MAJOR FINDINGS (0)

None in A5–A8 after fixing the compile error.

---

## MINOR FINDINGS (2)

1. **`ui_command_tests.rs:18`** — `vb_cli` function is dead code (`never used`) — warn dead_code
2. **`ui_command_tests.rs:13,15`** — unused imports `OsString` and `Output` — warn unused_imports

*These are warnings only, not lethal. Test bodies are present and substantive.*

---

## Mandate (Required Before Resubmission)

1. **Fix `slot_written_ordering_integration_tests.rs:790`** — Change `slot: SlotIdx(1)` pattern to use guard clause:
   ```rust
   JournalEvent::SlotWrittenEvent {
       seq,
       slot,
       ..
   } if seq.get() == 5 && slot.get() == 1
   ```
2. **Re-run full Tier 0 → Tier 3 after fix** — especially confirm all 3 packages compile and nextest passes

---

## Per-File Status

| File | Compile | Test Bodies | `unwrap_used` allow | Banned Patterns |
|------|---------|------------|-------------------|----------------|
| `slot_written_ordering_integration_tests.rs` | **FAIL** (L790) | ✅ Full bodies | ✅ None | ✅ Clean |
| `ui_command_tests.rs` | ✅ PASS | ✅ Full bodies | ✅ None | ✅ Clean |
| `journal_event_tests.rs` | ✅ PASS | ✅ Full bodies | ✅ None | ✅ Clean |
