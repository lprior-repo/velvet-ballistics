# Test Suite Review — LETHALs 5–8 (MODE 2: Suite Inquisition)

## VERDICT: REJECTED

---

## Tier 0 — Static Analysis

### [PASS] Banned pattern scan
No `assert!(result.is_ok())` or `assert!(result.is_err())` in the 4 target files.
The `let _ = rx.try_recv()` at `bounded_queue_tests.rs:452` is a documented channel drain
(pre-condition setup), not silent error suppression. The subsequent `assert!(warning.is_ok())`
(line 458) is the actual assertion. Clean.

### [PASS] Determinism/evidence scan
No `static mut`, `lazy_static!`, or `once_cell::Mutex`/`RwLock` in the 4 target files.

### [PASS] Mock interrogation
No `mockall`, `Mock::new()`, or `.expect_()` in the 4 target files.

### [PASS] Integration test purity
`slot_written_ordering_integration_tests.rs` uses only public crate APIs (`vb_core::*`,
`vb_runtime::*`, `vb_storage::*`). No `use crate::internal::*` violations in this file.

### [N/A] Error variant completeness
No error enums defined in the test files themselves. The `ActionQueueError::QueueFull`
variant is exercised with exact-field assertions in `bounded_queue_tests.rs` (lines 164, 179,
188, 199–204). Clean.

### [FAIL] Density audit
- `bounded_queue_tests.rs`: 831 lines, 69 tests covering constructors, enqueue, dequeue,
  backpressure, invariants, boundaries — adequate.
- `journal_event_tests.rs`: 644 lines, 18+ tests covering all parse_event error variants
  and is_valid invariants — adequate.
- `ui_command_tests.rs`: 376 lines, but 5 of 13 tests are EMPTY STUBS (see LETHAL #2).
- `slot_written_ordering_integration_tests.rs`: 1331 lines, 16 tests covering B-1/B-2/B-3 —
  adequate in coverage but FAILS TO COMPILE (see LETHAL #1).

---

## Tier 1 — Execution

### [PASS] Test compile (vb_runtime, vb_storage, vb_cli)
```
cargo test -p vb_runtime --no-run   → clean
cargo test -p vb_storage --no-run   → clean
cargo test -p vb_cli --no-run       → clean (warnings only)
```

### [FAIL] Test compile (workspace_tests — slot_written_ordering_integration_tests.rs)
```
cargo test -p velvet-ballastics-workspace-tests --no-run
```
**10 compilation errors, all in `slot_written_ordering_integration_tests.rs`:**

| Error | Location | Detail |
|-------|----------|--------|
| `E0432` | line 31 | `vb_storage::capability::CapabilitySet` — `capability` does not exist in `vb_storage` |
| `E0425` | line 627 | `postcard::to_allocate` — function does not exist (6 occurrences; should be `to_allocvec`) |
| `E0308` | line 208 | `drive_deterministic_full`: expected `&[ActionContract]`, found `&Box<[ConstValue]>` |
| `E0308` | line 299 | Same type mismatch as above |
| `E0164` | line 790 | `SlotIdx::new(1)` in pattern match position — expected tuple struct, found associated function |

The suite cannot run because this file does not compile.

### [PASS] vb_runtime nextest: 1469 passed, 0 failed, 0 flaky
### [PASS] vb_storage nextest: 1016 passed, 0 failed, 4 skipped

### [FAIL] vb_cli nextest: 5 FAILED (ui subcommand not implemented)
The `ui` command is not yet wired into the CLI parser. All 5 integration tests that
invoke `vb ui` fail with `"unknown command: ui"`.

```
cmd_ui_returns_error_when_workspace_path_does_not_exist   FAILED
cmd_ui_returns_error_when_workspace_path_is_a_file       FAILED
cmd_ui_accepts_valid_workspace_directory                 FAILED
ui_command_missing_workspace_flag_exits_with_error        FAILED
ui_command_rejects_path_traversing_parent                FAILED
```

The comment at `ui_command_tests.rs:11` reads: *"Tests are designed to fail until the
`ui` command is fully implemented."* — This is not a valid test suite state. A test
that is designed to fail is not a test; it is a placeholder.

---

## Tier 2 — Coverage

Skipped. Compilation fails. No coverage data.

---

## Tier 3 — Mutation

Skipped. Compilation fails. No mutants to run.

---

## LETHAL FINDINGS

### LETHAL #1 — slot_written_ordering_integration_tests.rs: DOES NOT COMPILE
**Severity:** LETHAL — blocks all execution, coverage, and mutation tiers.

The file has 10 distinct compilation errors spanning 3 categories:

1. **Missing module import** (`line 31`): `vb_storage::capability::CapabilitySet` does not
   exist. The `vb_storage` crate has no `capability` submodule in its public API.
   **Fix:** Remove the import and use `vb_core::CapabilitySet` (which is imported on line 20
   of `journal_event_tests.rs` and exists in `vb_core`).

2. **Wrong postcard API** (6 occurrences, lines 627, 647, 749, 966, 1252, 1266):
   `postcard::to_allocate(&SlotValue::I64(...))` — `to_allocate` does not exist in
   postcard 1.x. Should be `postcard::to_allocvec(&SlotValue::I64(...))`.
   **Fix:** Replace all 6 occurrences.

3. **Wrong argument type to `drive_deterministic_full`** (lines 208, 299):
   The function expects `&[ActionContract]` as the constants argument but
   `&Box<[ConstValue]>` is being passed. This is a signature mismatch — the test is
   calling the function with the wrong type entirely.
   **Fix:** Determine correct API; the `constants` parameter should likely be
   `&[ActionContract]` or the function signature needs to be updated to accept
   `&[ConstValue]`.

4. **`SlotIdx::new` in pattern position** (line 790):
   `slot: SlotIdx::new(1)` in a `matches!` pattern. `SlotIdx::new` is a constructor
   function, not a tuple variant. Should be `slot: SlotIdx(slot_idx)` or direct
   comparison.
   **Fix:** Use correct pattern syntax for `SlotIdx`.

---

### LETHAL #2 — ui_command_tests.rs: 5 EMPTY STUB TESTS
**Severity:** LETHAL — tests prove nothing; they have no test body.

| Test | Line | Issue |
|------|------|-------|
| `parse_ui_command_accepts_valid_workspace_flag` | 64 | Body is only comments (lines 65–88). No assertions. |
| `parse_ui_command_rejects_missing_workspace_flag` | 92 | Body is only comments. No assertions. |
| `parse_ui_command_rejects_empty_workspace_value` | 100 | Body is only comments. No assertions. |
| `parse_ui_command_rejects_unknown_flag` | 107 | Body is only comments. No assertions. |
| `parse_ui_command_rejects_parent_traversal_path` | 113 | Body is only comments. No assertions. |

These 5 tests are comments-only. If you delete the implementation of the `ui` command,
these tests would still compile and "pass" (vacuously). They test nothing.

---

### LETHAL #3 — ui_command_tests.rs: 5 INTEGRATION TESTS DESIGNED TO FAIL
**Severity:** LETHAL — the `ui` command does not exist in the CLI; all 5 tests fail
with `"unknown command: ui"`.

```
crates/vb_cli/tests/ui_command_tests.rs:127   cmd_ui_returns_error_when_workspace_path_does_not_exist
crates/vb_cli/tests/ui_command_tests.rs:163   cmd_ui_returns_error_when_workspace_path_is_a_file
crates/vb_cli/tests/ui_command_tests.rs:190   cmd_ui_accepts_valid_workspace_directory
crates/vb_cli/tests/ui_command_tests.rs:218   ui_command_missing_workspace_flag_exits_with_error
crates/vb_cli/tests/ui_command_tests.rs:261   ui_command_rejects_path_traversing_parent
```

These tests call `vb_cli_binary(&["ui", ...])` which invokes `velvet-ballastics ui`.
The CLI parser does not recognize `"ui"` as a valid subcommand. Every one of these
tests fails at the binary-invocation level — not at the argument-validation level
they are designed to test.

The file comment at line 11 acknowledges this: *"Tests are designed to fail until the
`ui` command is fully implemented."* — This is not a valid test suite. The tests
must be written against implemented behavior, not against hypothetical behavior.

---

### LETHAL #4 — journal_event_tests.rs: MODULE-LEVEL `unwrap` ALLOWANCE
**Severity:** LETHAL — module-level `#![allow(clippy::unwrap_used)]` (line 6) defeats
the purpose of the zero-unwrap enforcement.

Every test in `journal_event_tests.rs` that calls `result.expect("parse should succeed")`
or `parsed.expect(...)` is using unwrap-as-assertion. The module-level allow means
clippy will never flag these. The skill explicitly states:

> `unwrap()` where the unwrap IS the assertion = **LETHAL** (use `assert_eq!(result, Ok(expected))`)

The correct form for `parse_event` tests is `assert_eq!(result, Ok(expected))` or a
`match` with `assert_eq!` on each field. The `.expect()` calls throughout this file are
exactly the prohibited pattern.

Example at line 50:
```rust
let parsed = result.expect("parse_event should succeed for valid record");
```
Should be:
```rust
let Ok(parsed) = result else { panic!("parse_event should succeed for valid record") };
// then assert_eq! on each field
```

---

## MAJOR FINDINGS (0 — all LETHALs consumed the threshold)

---

## MINOR FINDINGS (0)

---

## MANDATE

The following must exist before resubmission:

1. **slot_written_ordering_integration_tests.rs — Fix ALL 10 compilation errors:**
   - Remove or correct `vb_storage::capability::CapabilitySet` import
   - Replace all 6 `postcard::to_allocate` with `postcard::to_allocvec`
   - Fix `drive_deterministic_full` argument type mismatch (determine correct API)
   - Fix `SlotIdx::new` pattern syntax

2. **ui_command_tests.rs — Remove or implement the 5 empty stub tests:**
   Tests at lines 64, 92, 100, 107, 113 must either have actual test bodies or be
   removed. Comment-only tests are not tests.

3. **ui_command_tests.rs — Fix the 5 failing integration tests:**
   Either wire the `ui` subcommand into the CLI parser or mark these tests as
   `#[ignore]` with a tracking issue. They cannot be left in a permanently-failing
   state.

4. **journal_event_tests.rs — Remove `#![allow(clippy::unwrap_used)]`:**
   Replace all `.expect()` calls used as assertions with proper `assert_eq!` forms.
   The module-level allow is a blanket waiver that defeats the entire purpose of
   zero-unwrap enforcement.

After fixes: re-run ALL tiers from Tier 0. Compilation must pass for all 4 target
files before Tier 1 execution is attempted.
