# Architectural Drift Report: `vb_cli/src/main_tests.rs`

**File**: `/home/lewis/src/velvet-ballistics/crates/vb_cli/src/main_tests.rs`
**Status**: CRITICAL DRIFT — **980 lines (327% of 300-line limit)**
**Date**: 2026-05-29
**Enforcer**: architectural-drift

---

## Executive Summary

This file is a **massive architectural violation**. At **980 lines**, it exceeds the 300-line hard limit by **680 lines** (~327% over). It violates Single Responsibility Principle by mixing **8 distinct test domains** into a single file, and exhibits rampant **Primitive Obsession** by using raw integers where domain types exist.

---

## VIOLATION #1: File Size (CRITICAL)

| Metric | Value | Limit | Violation |
|--------|-------|-------|-----------|
| Total Lines | 980 | 300 | **+680 lines (+227%)** |
| Test Functions | ~40 | N/A | N/A |
| Domains Mixed | 8 | 1 | **+7 extra domains** |

**Hard Limit Exceeded**: YES — This file MUST be split.

---

## VIOLATION #2: Single Responsibility Principle

This file contains **8 distinct test responsibility domains** that must be separated:

### Domain Map

| Lines | Domain | Test Count | Module Target |
|-------|--------|------------|--------------|
| 34–76 | `ai-context` command parsing | 3 | `cli_args_ai_context_tests.rs` |
| 130–201 | `run`/`run-compiled` command parsing | 3 | `cli_args_run_tests.rs` |
| 203–233 | Input mapping error messages | 2 | `cli_input_mapping_tests.rs` |
| 236–368 | Action list/inspect command parsing | 10 | `cli_args_action_tests.rs` |
| 371–477 | Action registry introspection | 3 | `cli_action_registry_tests.rs` |
| 488–539 | Runtime input mapping logic | 3 | `cli_input_mapping_tests.rs` |
| 542–653 | Journaled storage/resolver | 2 | `cli_storage_resolver_tests.rs` |
| 694–790 | Step execution | 5 | `cli_step_execution_tests.rs` |
| 793–875 | Parse error variant coverage | 2 | `cli_args_error_tests.rs` |
| 881–971 | `parse_run_id` | 7 | `cli_run_id_tests.rs` |
| 977–980 | OutputFormat | 1 | `cli_output_format_tests.rs` |

**Required Splits**: Minimum **6 new test files** based on domain boundaries.

---

## VIOLATION #3: Primitive Obsession

### 3.1 Raw `u16` for Action IDs (Lines 281, 421–441)

```rust
// LINE 281: Raw action_id: 2
Ok(Command::ActionInspect {
    action_id: 2,  // <-- RAW INTEGER
    ...
})
```

```rust
// LINES 421–441: Raw assertions on action properties
assert_eq!(first.id, 1);                    // <-- u16 raw
assert_eq!(first.idempotency, "deterministic_pure"); // <-- string
assert_eq!(first.retry_safety, "safe");
assert_eq!(first.side_effect, "none");
assert_eq!(first.input_slot_count, 1);       // <-- u16 raw
assert_eq!(first.output_slot_count, 1);      // <-- u16 raw
assert_eq!(first.timeout_ms, 1_000);         // <-- u32 raw
```

**FIX REQUIRED**: Use `ActionId::new(2)` and typed `TimeoutMs`, `SlotCount` wrappers.

### 3.2 Raw `u16` for Step IDs (Lines 697–701)

```rust
// LINES 696–701: StepTarget with primitives
let target = StepTarget {
    step_id: 5,                          // <-- RAW u16
    step_input: PathBuf::from("data.bin"), // <-- RAW PathBuf
};
assert_eq!(target.step_id, 5);           // <-- assertion on raw
assert_eq!(target.step_input, PathBuf::from("data.bin"));
```

**FIX REQUIRED**: Use `StepId::new(5)` and a `StepInputPath` value object.

### 3.3 Raw ExitCode Construction (Lines 913, 926, 940)

```rust
// LINES 913, 926, 940: Raw ExitCode from cast
std::process::ExitCode::from(CliExitCode::ValidationFailed as u8)
```

**FIX REQUIRED**: Use `CliExitCode::to_exit_code()` or a proper `From<CliExitCode> for ExitCode` impl.

### 3.4 Raw `u16` Collection from Contracts (Lines 383–392)

```rust
// LINES 383–392: Collecting raw u16 from contracts
let first_listing: Vec<u16> = registry
    .registered_contracts()
    .iter()
    .map(|contract| contract.id.get())  // <-- .get() returns raw u16
    .collect();
```

**FIX REQUIRED**: Return `Vec<ActionId>` or a newtype `ActionIdList`.

---

## VIOLATION #4: Inline Test Infrastructure

### 4.1 `main_test_tempdir()` (Lines 24–31)

```rust
fn main_test_tempdir() -> std::io::Result<tempfile::TempDir> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/vb-cli-main-tests-tmp");
    std::fs::create_dir_all(&root)?;
    tempfile::Builder::new()
        .prefix("vb-cli-main-")
        .tempdir_in(root)
}
```

**VIOLATION**: This is **not** a test helper; it's a **test infrastructure concern**.
**FIX**: Move to `vb_cli/tests/support/` or `vb_cli/src/test_support.rs`.

### 4.2 `args()` Helper (Lines 89–91)

```rust
fn args(parts: &[&str]) -> Vec<OsString> {
    parts.iter().map(|part| OsString::from(*part)).collect()
}
```

**VIOLATION**: Could be shared across multiple test files.
**FIX**: Move to a shared test helper module.

### 4.3 `finish_workflow()` Test Factory (Lines 93–128)

```rust
fn finish_workflow() -> Option<CompiledWorkflow> {
    let set_const = CompiledNode { /* 12 fields */ };
    let finish = CompiledNode { /* 10 fields */ };
    let parts = WorkflowParts { /* 12 fields */ };
    CompiledWorkflow::try_from_parts(parts).ok()
}
```

**VIOLATION**: 36 lines of complex domain construction logic embedded in a test file.
**FIX**: Either use a proper test fixture file or move to `vb_cli/tests/fixtures/` directory.

---

## VIOLATION #5: Deeply Nested Conditionals

### Example: Lines 488–506 (`map_runtime_inputs_decodes_slot_values`)

```rust
fn map_runtime_inputs_decodes_slot_values() {
    let compiled = finish_workflow();
    assert!(compiled.is_some(), "test workflow should compile");
    if let Some(compiled) = compiled {              // NESTED #1
        let values: Box<[vb_core::SlotValue]> = Box::from([vb_core::SlotValue::Bool(true)]);
        let payload = postcard::to_allocvec(&values);
        assert_eq!(payload.as_ref().map(|_| ()), Ok(()));
        let Ok(payload) = payload else {           // NESTED #2
            return;
        };
        let mapped = map_runtime_inputs(&compiled, &payload);
        assert_eq!(mapped, Ok(Box::from([(vb_core::SlotIdx::ZERO, vb_core::SlotValue::Bool(true))])));
    }
}
```

**Pattern**: Every test has 2–4 levels of nested conditionals via `if let Some()` and `let Ok()`.
**FIX**: Use early returns or `?` operator with proper error conversion.

---

## VIOLATION #6: Magic Numbers

| Line(s) | Value | Context | Should Be |
|---------|-------|---------|-----------|
| 383–394 | `1, 2, 3` | Action IDs | `ActionId::new(1..=3)` |
| 421–441 | `1_000, 5_000, 10_000` | Timeout ms | `TimeoutMs::new(1_000)` etc. |
| 421–441 | `1, 2, 1, 0` | Slot counts | `SlotCount` wrapper |
| 697 | `5` | Step ID | `StepId::new(5)` |
| 469–470 | `65_536` | Max bytes | Named constant |

---

## VIOLATION #7: Testing Private Implementation Details

Tests reach into `super::*` imports extensively (lines 5–13):

```rust
use super::{
    ActionRegistryMode, CliExitCode, Command, DurabilityMode,
    INPUT_MAPPING_DECODE_FAILED_MESSAGE, /* 15+ items */
    action_contract_detail, action_idempotency_name, action_table_rows,
    build_step_frame, decode_step_inputs, execute_step_isolated, ...
};
```

**VIOLATION**: Tests should exercise **public API behavior**, not internal helper functions.
**FIX**: Refactor to test through the public CLI interface, or ensure helpers are truly module-private with clear contracts.

---

## Summary of Required Refactors

| Priority | Issue | Effort |
|----------|-------|--------|
| **P0** | Split into 6+ separate test files by domain | HIGH |
| **P0** | Replace raw `u16` action/step IDs with domain types | MEDIUM |
| **P1** | Move `main_test_tempdir()` to test infrastructure | LOW |
| **P1** | Move `args()` helper to shared test module | LOW |
| **P1** | Replace magic numbers with named constants | LOW |
| **P2** | Flatten nested conditionals using `?` or early returns | MEDIUM |
| **P2** | Replace raw `ExitCode::from(u8)` with proper `From` impl | LOW |
| **P2** | Consider fixture files for `finish_workflow()` data | MEDIUM |

---

## Architecture Contract Reference

- **300-line hard limit**: ENFORCED
- **One domain per file**: ENFORCED
- **No primitive obsession**: Types must wrap primitives where domain concepts exist
- **Tests as specifications**: Tests should express behavior, not implementation details
- **Scott Wlaschin DDD**: Types first, functions second; make illegal states unrepresentable

---

## Verdict

**REJECTED** — This file cannot be approved in its current form.

**Immediate Action Required**:
1. Split into minimum 6 domain-specific test files
2. Introduce `ActionId`, `StepId`, `TimeoutMs`, `SlotCount` value objects
3. Extract test infrastructure to `tests/support/`
4. Remove magic numbers in favor of named constants

**Estimated Lines After Refactor**: ~160 lines across 6 files (within limits)

---

*Report generated by architectural-drift enforcer*
*Workspace: `/home/lewis/src/velvet-ballistics/arch-drift-hammer`*
