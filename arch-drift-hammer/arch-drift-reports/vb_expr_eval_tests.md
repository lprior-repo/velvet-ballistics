# Architectural Drift Report: `vb_expr/eval_tests.rs`

## File Identification
- **Path**: `crates/vb_expr/src/eval_tests.rs`
- **Size**: 2740 lines
- **Test Count**: 185 `#[test]` functions + 2 `proptest!` blocks
- **File Age**: Original comment says "Extracted from eval.rs to satisfy the 300-line file limit"

---

## Critical Violations

### 1. File Size Violation (CRITICAL)
| Metric | Limit | Actual | Violation |
|--------|-------|--------|-----------|
| Lines | 300 | 2740 | **+2440 (913% over)** |

The file header claims it was *extracted* to satisfy the 300-line limit, but it now exceeds that limit by **9x**.

### 2. Test Inline vs External Classification

| Aspect | Finding | Assessment |
|--------|---------|------------|
| Location | `crates/vb_expr/src/` | **Inline test location** (should be `tests/` for external) |
| Module structure | `#[cfg(test)] mod tests { ... }` | Inline test module |
| File naming | `*_tests.rs` suffix | Correct convention for external tests |
| **Correct location** | `crates/vb_expr/tests/eval_tests.rs` | NOT `src/` |

**Finding**: Despite the `_tests.rs` suffix suggesting an external integration test file, this is actually an **inline test module** embedded within a source file. The file lives in `src/` not `tests/`.

---

## Architecture Category

**Category**: `inline-test-in-src` (violation)
- Tests marked `#[cfg(test)]` inside `src/` are inline unit tests
- Proper external tests belong in `crates/vb_expr/tests/` directory
- This file violates both the 300-line rule AND the workspace structure convention

---

## Recommendations

### Immediate Actions (Required)
1. **SPLIT this file** into multiple focused test files of ≤300 lines each:
   - `eval_arithmetic_tests.rs` — binary/unary arithmetic operations
   - `eval_comparison_tests.rs` — <, <=, >, >=, ==, !=
   - `eval_boolean_tests.rs` — AND, OR, NOT with all combinations
   - `eval_helper_tests.rs` — Exists, Empty, Unique, Length, etc.
   - `eval_store_tests.rs` — ValueStore-aware helpers
   - `eval_f64_tests.rs` — Floating point operations
   - `eval_edge_cases_tests.rs` — Overflow, underflow, type mismatches
   - `eval_integration_tests.rs` — Full lex→parse→compile→eval pipeline

2. **MOVE to proper location**: Move split files to `crates/vb_expr/tests/` if they are cross-module integration tests, OR keep in `src/` as unit tests but rename to `eval.rs` (inline tests convention is `mod tests` inside the main source file)

### Long-term Fix
- The `eval.rs` file (34.9K) likely also violates the 300-line rule and should be modularized
- Extract evaluation operation handlers into separate `eval_ops/` submodule

---

## Summary

| Field | Value |
|-------|-------|
| **Lines Count** | 2740 |
| **Test Count** | 185 unit tests + 2 proptest blocks |
| **Location Category** | `inline-test-in-src` (VIOLATION) |
| **Primary Drift** | File is 913% over the 300-line limit |
| **Secondary Drift** | Inline tests in `src/` with `_tests.rs` naming confusion |
| **Recommendation** | **SPLIT IMMEDIATELY** into 9+ focused files |
