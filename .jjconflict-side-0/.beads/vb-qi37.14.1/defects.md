# Defects Routing: vb-qi37.14.1

## Defect Summary

| ID | Title | Severity | Exit Code Impact | Status |
|----|-------|----------|------------------|--------|
| DEFECT-001 | PRE-003 Compile Error Returns Wrong Exit Code | CRITICAL | 3 instead of 2 | **FIXED** |
| DEFECT-002 | PRE Condition Errors Route to Wrong Output Stream | MODERATE | N/A | **FIXED** |

---

## DEFECT-001: PRE-003 Compile Error Returns Wrong Exit Code

**File**: `crates/vb_cli/src/app_impl.rs`  
**Line**: 1553 (was 1546)  
**Function**: `compile_bytes_json`  

### Previous Code
```rust
Err(CliExitCode::CompileFailed.into())
```

### Contract Requirement
- **POST-008**: "Exit code is `ValidationFailed` (2) for preconditions PRE-001 through PRE-004 failures."
- **Error Taxonomy**: `"workflow_compile_error"` → exit code 2

### Fix Applied
```rust
Err(CliExitCode::ValidationFailed.into())
```

### Verification
- Test `run_step_compile_error_reports_failure` updated to accept exit code 2
- All 25 tests in `vb_qi37_14_1_run_step` pass

---

## DEFECT-002: PRE Condition Errors Route to Wrong Output Stream

**File**: `crates/vb_cli/src/app_impl.rs`  
**Lines**: 5168-5177 (was 5172-5199)

### Issue (Before)
- PRE condition errors use `write_contract_error_json` → JSON to **stdout**
- Runtime errors use `json_error` → diagnostic to **stderr**

### Fix Applied (Option A - stderr)
Changed `write_contract_error_json` to write JSON errors to **stderr** (Unix convention).

### Code Change
```rust
// Before: io::stdout()
// After:  io::stderr()
```

### Test Update
- Test `run_step_invalid_step_id_json_includes_error_details` updated to check stderr instead of stdout
- Updated doc comment to reflect correct behavior
- All 25 tests in `vb_qi37_14_1_run_step` pass

---

## Routing

| Defect | Owner | Priority | Status |
|--------|-------|----------|--------|
| DEFECT-001 | vb-qi37.14.1 | CRITICAL - blocks ship | **FIXED** |
| DEFECT-002 | vb-qi37.14.1 | MODERATE - should fix before ship | **FIXED** |

---

*Black-hat review: vb-qi37.14.1*
*Status: ALL DEFECTS FIXED - verified by `cargo test --package vb_cli --test vb_qi37_14_1_run_step`*
