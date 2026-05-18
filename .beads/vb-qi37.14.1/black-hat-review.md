# Black-Hat Review: vb-qi37.14.1

## Bead
- **ID**: vb-qi37.14.1
- **Title**: cli: Add single-step run command
- **Review Date**: 2026-05-18
- **Reviewer**: black-hat-reviewer
- **Status**: PASS

---

## Re-Attack Summary

Re-attacked after fixes for DEFECT-001 and DEFECT-002 were applied. Both defects confirmed **FIXED**.

---

## DEFECT-001 Fix Verification: PASS

**Location**: `crates/vb_cli/src/app_impl.rs:1553`

### Fix Applied
```rust
Err(CliExitCode::ValidationFailed.into())
```
- Previously: `Err(CliExitCode::CompileFailed.into())` (exit code 3)
- Now: `Err(CliExitCode::ValidationFailed.into())` (exit code 2)

### Contract Requirement Met
```
POST-008: "Exit code is ValidationFailed (2) for preconditions PRE-001 through PRE-004 failures."
```
PRE-003 (workflow_compile_error) now correctly returns exit code 2.

---

## DEFECT-002 Fix Verification: PASS

**Location**: `crates/vb_cli/src/app_impl.rs:5179-5187`

### Fix Applied
```rust
fn write_contract_error_json(value: &serde_json::Value, format: OutputFormat) {
    if format == OutputFormat::Text {
        if let Some(msg) = value.get("message").and_then(serde_json::Value::as_str) {
            errln!("{msg}");
        }
    } else {
        // Write the contract-format JSON directly to stderr (Unix convention)
        let stderr = io::stderr();
        let mut handle = stderr.lock();
        // ... writes JSON to stderr
    }
}
```
- Previously: Wrote JSON to stdout
- Now: Writes JSON to stderr via `io::stderr()`

### Contract Requirement Met
POST-002: Consistent structured output behavior. Unix convention (errors → stderr) now followed.

---

## What I Verified: PASS

### ✅ Single-Step Execution (INV-005 / POST-001)
`execute_step_isolated` calls `vb_core::step_once()` exactly once.

### ✅ Durability Gate (PRE-001)
Correctly enforces `DurabilityMode::None` with exit code 2.

### ✅ Delta Reporting (POST-004)
All four delta types present: pc_delta, slot_deltas, taint_deltas, state_deltas.

### ✅ Output Schema (POST-002, POST-003)
All required fields in `build_step_result_json`.

### ✅ Text Output (POST-002)
Step info, input slots, output slot, signal, and taint included.

---

## Edge Cases Examined

| Edge Case | Status |
|-----------|--------|
| Empty step input file | ✅ Handled |
| Out-of-bounds step ID | ✅ Returns exit code 2 |
| Invalid postcard data | ✅ Returns exit code 2 |
| Slots/taint length mismatch | ✅ Safe via `usize::min()` |
| All slots unchanged | ✅ Empty delta arrays |

---

## Security Observations

No security issues. No `unsafe`, `unwrap`, `expect`, or `panic`. Proper error handling throughout.

---

## Conclusion

**STATUS**: PASS

Both DEFECT-001 and DEFECT-002 have been successfully fixed. The implementation is ready to ship.

---

*Black-hat review complete. All critical defects resolved.*
