# Architectural Drift Report: vb_expr_helpers_edge_case_tests.rs

## File Summary

| Metric | Value |
|--------|-------|
| **File** | `crates/vb_expr/src/helpers/tests/edge_case_tests.rs` |
| **Total Lines** | 1798 |
| **Test Count** | 78 |
| **Location Category** | `helpers/tests/` (test helper module) |
| **Drift Status** | ⚠️ **VIOLATION** — File exceeds 300-line soft cap by 499.8% |

## Size Analysis

```
Line Count:  1798 lines
Soft Cap:    300 lines
Overflow:    1498 lines (499.8% of cap)
```

## Test Coverage Breakdown

| Module | Test Count | Helpers Tested |
|--------|------------|----------------|
| `empty_edge_cases` | 5 | `eval_empty` |
| `unique_edge_cases` | 3 | `eval_unique` |
| `contains_edge_cases` | 3 | `eval_contains` |
| `starts_with_edge_cases` | 5 | `eval_starts_with` |
| `ends_with_edge_cases` | 5 | `eval_ends_with` |
| `has_edge_cases` | 1 | `eval_has` |
| `append_edge_cases` | 3 | `eval_append` |
| `append_if_edge_cases` | 3 | `eval_append_if` |
| `sum_edge_cases` | 4 | `eval_sum` |
| `length_edge_cases` | 5 | `eval_length` |
| `count_edge_cases` | 3 | `eval_count` |
| `merge_edge_cases` | 6 | `eval_merge` |
| `text_ops_oob_edge_cases` | 3 | OOB checks for text ops |
| `exists_edge_cases` | 5 | `eval_exists` |
| `has_more_edge_cases` | 3 | `eval_has` additional |
| `empty_more_edge_cases` | 3 | `eval_empty` additional |
| `unique_more_edge_cases` | 1 | `eval_unique` additional |
| `sum_more_edge_cases` | 2 | `eval_sum` additional |
| `length_more_edge_cases` | 3 | `eval_length` additional |
| `count_more_edge_cases` | 2 | `eval_count` additional |
| `append_more_edge_cases` | 1 | `eval_append` additional |
| `starts_with_more_edge_cases` | 2 | `eval_starts_with` additional |
| `ends_with_more_edge_cases` | 2 | `eval_ends_with` additional |
| `contains_more_edge_cases` | 2 | `eval_contains` additional |
| `merge_more_edge_cases` | 3 | `eval_merge` additional |
| **TOTAL** | **78** | **12 helpers + OOB** |

## Structural Observations

### ✅ Positive Attributes
1. **Excellent test organization** — Tests are logically grouped by helper function into modules
2. **Comprehensive edge case coverage** — Type mismatches, OOB errors, empty inputs, boundary conditions
3. **Clear module documentation** — Each module has header comments explaining what's tested
4. **Shared test infrastructure** — `eval_ops_with_slots` helper avoids code duplication
5. **No unsafe code** — File uses `#![forbid(unsafe_code)]`

### ⚠️ Drift Violations
1. **File size catastrophic** — 1798 lines is 6x the 300-line recommendation
2. **Module proliferation** — 24 test modules in a single file is excessive
3. **Test count concentration** — 78 tests in one file may slow compile/test cycles

## Recommendation: **SPLIT BY HELPER FUNCTION**

The file should be broken into individual test modules, one per helper:

```
helpers/tests/
├── edge_case_tests.rs          # Shared infra only (~90 lines)
├── eval_empty_tests.rs        # ~130 lines (5+3 tests)
├── eval_unique_tests.rs       # ~110 lines (3+1 tests)
├── eval_contains_tests.rs     # ~120 lines (3+2 tests)
├── eval_starts_with_tests.rs  # ~140 lines (5+2 tests)
├── eval_ends_with_tests.rs    # ~140 lines (5+2 tests)
├── eval_has_tests.rs          # ~100 lines (1+3 tests)
├── eval_append_tests.rs       # ~120 lines (3+1 tests)
├── eval_append_if_tests.rs    # ~130 lines (3 tests)
├── eval_sum_tests.rs          # ~130 lines (4+2 tests)
├── eval_length_tests.rs       # ~150 lines (5+3 tests)
├── eval_count_tests.rs        # ~120 lines (3+2 tests)
├── eval_merge_tests.rs        # ~200 lines (6+3 tests)
├── eval_exists_tests.rs       # ~140 lines (5 tests)
└── text_ops_oob_tests.rs      # ~100 lines (3 tests)
```

**Expected outcome**: Each file under 200 lines, compile-time parallelism improved, clearer ownership boundaries.

## Risk Assessment

| Dimension | Current | Target | Status |
|-----------|---------|--------|--------|
| Lines | 1798 | ≤300 | 🔴 FAIL |
| Tests per file | 78 | ≤50 | 🟡 CAUTION |
| Modules | 24 | ≤10 | 🔴 FAIL |
| Compile cache efficiency | Low | High | 🔴 FAIL |

---
*Report generated: 2026-05-29*
*Tool: architectural-drift agent*
