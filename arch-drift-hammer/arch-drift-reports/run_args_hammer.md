# Architectural Drift Report: `run.rs` (402 lines)

**File**: `/home/lewis/src/velvet-ballistics/crates/vb_cli/src/args/tests/run.rs`
**Status**: FAILED — 402 lines (VIOLATION: exceeds 300-line limit by 34%)
**Date**: 2026-05-29
**Enforcer**: arch-drift-hammer

---

## Executive Summary

This file VIOLATES the <300 line rule by 102 lines and commits multiple **Primitive Obsession** violations by scattering `PathBuf` and raw `i32` across tests instead of using domain types.

---

## Violation 1: File Size (<300 Lines)

| Metric | Value |
|--------|-------|
| Actual | 402 lines |
| Limit | 300 lines |
| Over | 102 lines (34%) |

**Verdict**: FAIL — Must be split.

---

## Violation 2: Primitive Obsession — `PathBuf` Unleashed

Every test that constructs a `Command::Run` or similar uses raw `PathBuf` for domain concepts:

| Domain Concept | Raw Type Used | Should Be |
|---------------|---------------|-----------|
| Workflow path | `PathBuf` | `WorkflowPath` (value object) |
| Input binary | `PathBuf` | `InputBinPath` (value object) |
| Database path | `PathBuf` | `DatabasePath` (value object) |
| Step input | `PathBuf` | `StepInputPath` (value object) |
| IPC socket | `PathBuf` | `SocketPath` (value object) |

**Evidence** (24 occurrences across file):
```rust
// Line 27-30
assert_eq!(workflow, PathBuf::from("workflow.yaml"));
assert_eq!(input_bin, PathBuf::from("input.bin"));
assert_eq!(db, Some(PathBuf::from("journal-db")));

// Line 173-174
assert_eq!(target.step_id, 3);
assert_eq!(target.step_input, PathBuf::from("step-data.bin"));
```

**Scott Wlaschin Principle**: "Make illegal states unrepresentable." Raw `PathBuf` carries no domain semantics — any string can become any path, making it possible to swap `workflow` and `input_bin` with zero type-level enforcement.

---

## Violation 3: Primitive Obsession — Raw `i32` for `step_id`

The `StepTarget` struct and its tests use raw `i32`:

```rust
// Line 229-235
let target = StepTarget {
    step_id: 5,  // <-- raw i32, not StepId
    step_input: PathBuf::from("data.bin"),
};
assert_eq!(target.step_id, 5);  // <-- asserts against primitive
```

**Should be**: A `StepId` newtype wrapper:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepId(u32);  // or u16, u8 depending on bounds
```

**Current test (line 228-235)**: Tests `StepTarget` as a plain struct with primitives. This should be a proper value object with its own constructor and invariants.

---

## Violation 4: Primitive Obsession — String Literals in Tests

Durability mode strings are hardcoded directly:

```rust
// Lines 14-16, 44-45, 65-66, etc.
"--durability", "journaled",
"--durability", "strict",
"--durability", "none",
```

These should be constants or methods on `DurabilityMode`:
```rust
impl DurabilityMode {
    pub const JOURNALED: &'static str = "journaled";
    pub const STRICT: &'static str = "strict";
    pub const NONE: &'static str = "none";
}
```

---

## Violation 5: Test Logic Duplication

The same assertion pattern repeats 20+ times:

```rust
if let Ok(Command::Run { workflow, input_bin, durability, db, .. }) = parsed {
    // assertions
} else {
    assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
}
```

**Scott Wlaschin Principle**: "Duplication is far cheaper than the wrong abstraction." A helper function `assert_run_command_ok(parsed, ...)` or a test builder would eliminate 150+ lines.

---

## Violation 6: Monolithic Test File Structure

24 tests in a single file covering 5 distinct command scenarios:

| Command | Test Count |
|---------|------------|
| `run` | 11 tests |
| `run-compiled` | 3 tests |
| `submit` | 2 tests |
| `ipc-serve` | 2 tests |
| `step_target` | 1 test |
| Error cases | 5 tests |

**Should be**: Tests split into `run_cmd_tests.rs`, `submit_cmd_tests.rs`, `ipc_serve_cmd_tests.rs`, `step_target_tests.rs`.

---

## Required Refactors

### 1. Create Value Objects

```rust
// vb_cli/src/args/domain.rs (new file)
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowPath(PathBuf);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputBinPath(PathBuf);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabasePath(PathBuf);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepInputPath(PathBuf);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocketPath(PathBuf);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepId(u32);
```

### 2. Update `StepTarget`

```rust
// vb_cli/src/args/step_target.rs (update)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepTarget {
    pub step_id: StepId,           // was i32
    pub step_input: StepInputPath,  // was PathBuf
}
```

### 3. Split Test File

```
vb_cli/src/args/tests/
├── run_cmd_tests.rs        # 11 tests (~170 lines)
├── run_compiled_tests.rs   # 3 tests (~60 lines)
├── submit_tests.rs         # 2 tests (~50 lines)
├── ipc_serve_tests.rs      # 2 tests (~50 lines)
└── step_target_tests.rs    # 1 test + error cases (~50 lines)
```

### 4. Add Test Helpers

```rust
fn assert_run_command(
    parsed: Result<Command, ParseError>,
    workflow: &str,
    input_bin: &str,
    durability: DurabilityMode,
    db: Option<&str>,
) {
    // ... helper implementation
}
```

---

## Summary of Findings

| # | Violation | Severity | Lines Affected |
|---|-----------|----------|-----------------|
| 1 | File size > 300 | CRITICAL | 402 total |
| 2 | `PathBuf` primitive obsession | HIGH | ~120 lines |
| 3 | `i32` for `step_id` | HIGH | ~20 lines |
| 4 | String literals for durability | MEDIUM | ~30 lines |
| 5 | Test logic duplication | MEDIUM | ~150 lines |
| 6 | Monolithic structure | MEDIUM | 24 tests in 1 file |

**Total technical debt**: ~320 lines of refactoring needed to restore architectural compliance.

---

## Verdict

**ARCHITECTURAL DRIFT DETECTED** — File is non-compliant with <300 line rule and contains multiple primitive obsession violations per Scott Wlaschin DDD principles.

**Required Actions**:
1. Create domain value object types for all path-like primitives
2. Replace raw `i32` with `StepId` newtype
3. Split into 4-5 test modules by command
4. Extract test assertion helpers to reduce duplication
5. Reduce total file count to ≤300 lines

---

*Report generated by arch-drift-hammer*
