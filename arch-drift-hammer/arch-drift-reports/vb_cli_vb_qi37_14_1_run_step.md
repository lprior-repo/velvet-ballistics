# Architectural Drift Report: `vb_qi37_14_1_run_step.rs`

**File:** `crates/vb_cli/tests/vb_qi37_14_1_run_step.rs`  
**Analyzed:** 2026-05-29  
**Lines:** 1540  
**Test Count:** 26  
**Size Category:** ❌ **CRITICAL** (>300 lines)

---

## Executive Summary

| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| Total Lines | 1540 | 300 | ❌ OVER LIMIT |
| Test Count | 26 | N/A | ✓ |
| File Category | `crates/vb_cli/tests/` | `tests/` | ✓ |

**Drift Verdict:** ❌ **SEVERE ARCHITECTURAL DRIFT**  
This file exceeds the 300-line limit by **413%** (5.1× over budget).

---

## Structural Analysis

### Test Distribution

| Test Group | Count | Lines (est.) | Section |
|------------|-------|--------------|---------|
| VB-PRE001 (Durability gate) | 2 | ~200 | Lines 131–229 |
| VB-PRE002 (Invalid step ID) | 2 | ~220 | Lines 235–346 |
| VB-PRE003 (Compile failure) | 2 | ~200 | Lines 351–446 |
| VB-PRE005 (Output format) | 3 | ~300 | Lines 452–600 |
| VB-POST001 (Step execution) | 1 | ~100 | Lines 606–653 |
| VB-POST002 (JSON schema) | 2 | ~220 | Lines 659–781 |
| VB-POST003 (Step/kind/signal) | 1 | ~100 | Lines 787–842 |
| VB-POST004 (Delta schema) | 4 | ~400 | Lines 848–1113 |
| VB-POST005 (Finished signal) | 1 | ~120 | Lines 1119–1186 |
| VB-POST006 (Error format) | 2 | ~200 | Lines 1192–1289 |
| VB-POST007 (Durability error) | 1 | ~100 | Lines 1294–1343 |
| VB-POST008 (Exit codes) | 3 | ~300 | Lines 1348–1495 |
| VB-PRE004 edge (Empty input) | 1 | ~100 | Lines 1501–1540 |

### Code Organization

```
Lines 1–16:    Module documentation
Lines 18–50:   Test fixtures (SETCONST_WORKFLOW, NOP_WORKFLOW)
Lines 56–125:  Helper functions (7 functions)
Lines 131–1540: 26 test functions
```

---

## Violations

### 1. File Size Violation (CRITICAL)

**Rule:** Files must not exceed 300 lines.  
**Actual:** 1540 lines (413% of limit)

This single test file contains more code than many entire crates. The test file has grown organically without enforcement of architectural boundaries.

### 2. Test Helper Duplication

The `forced_assertion_failure()` pattern (lines 56–58) is unusual:

```rust
fn forced_assertion_failure() -> bool {
    false
}
```

This is used to trigger `assert!` failures when setup operations (tempdir, file write, CLI execution) fail, rather than using standard error propagation or `#[should_panic]`.

### 3. Repetitive Test Boilerplate

Each test follows an identical pattern:
```rust
let dir = match run_step_tempdir() {
    Ok(dir) => dir,
    Err(err) => {
        assert!(forced_assertion_failure(), "tempdir failed: {err}");
        return;
    }
};
let workflow_path = dir.path().join("workflow.yaml");
// ... repeated setup ...
let output = match run_cli(&[...]) {
    Some(output) => output,
    None => return,
};
```

This ~15 line pattern appears **26 times**, accounting for ~390 lines of duplication.

---

## Recommendations

### Immediate (Enforce)

1. **SPLIT this file immediately** into test group files:
   - `vb_qi37_14_1_run_step__pre001_durability.rs` (2 tests, ~230 lines)
   - `vb_qi37_14_1_run_step__pre002_invalid_step.rs` (2 tests, ~220 lines)
   - `vb_qi37_14_1_run_step__pre003_compile.rs` (2 tests, ~200 lines)
   - `vb_qi37_14_1_run_step__pre005_output_format.rs` (3 tests, ~300 lines)
   - `vb_qi37_14_1_run_step__post001_single_step.rs` (1 test, ~110 lines)
   - `vb_qi37_14_1_run_step__post002_json_schema.rs` (2 tests, ~230 lines)
   - `vb_qi37_14_1_run_step__post003_step_kind_signal.rs` (1 test, ~110 lines)
   - `vb_qi37_14_1_run_step__post004_delta_schema.rs` (4 tests, ~400 lines)
   - `vb_qi37_14_1_run_step__post005_finished_signal.rs` (1 test, ~120 lines)
   - `vb_qi37_14_1_run_step__post006_error_format.rs` (2 tests, ~200 lines)
   - `vb_qi37_14_1_run_step__post007_durability_exit.rs` (1 test, ~100 lines)
   - `vb_qi37_14_1_run_step__post008_exit_codes.rs` (3 tests, ~300 lines)
   - `vb_qi37_14_1_run_step__pre004_empty_input.rs` (1 test, ~100 lines)

2. **Extract shared helpers** to `crates/vb_cli/tests/helpers/run_step_helpers.rs`

### Short-term (Improve)

3. Replace `forced_assertion_failure()` pattern with standard error propagation or `Result`-based test helpers
4. Consider using `tempfile` crate's `Builder` with `.prefix()` and `.tempdir()` more directly
5. Factor out the repeated CLI argument construction pattern

### Long-term (Structural)

6. Create a test harness that supports parameterized tests for the various workflow scenarios
7. Consider moving workflow fixtures to dedicated YAML files loaded at test time

---

## Impact Assessment

| Aspect | Impact |
|--------|--------|
| Build Times | Minimal (tests compile separately) |
| CI Complexity | Increased (more test binaries) |
| Code Review | High (file is difficult to review) |
| Maintainability | Low (hard to find specific tests) |
| Architectural Coherence | Violated |

---

## Evidence

```bash
$ wc -l crates/vb_cli/tests/vb_qi37_14_1_run_step.rs
1540

$ grep -c '#\[test\]' crates/vb_cli/tests/vb_qi37_14_1_run_step.rs
26
```

---

**Report Generated By:** architectural-drift agent  
**Next Action:** File split required before any new features can be added to this test module
