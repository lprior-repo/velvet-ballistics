# Architectural Drift Report: `vb_expr_helpers_tests`

## File Analysis

| File | Lines | Size |
|------|-------|------|
| `crates/vb_expr/src/helpers/tests/edge_case_tests.rs` | 1798 | 62.6K |
| `crates/vb_expr/src/helpers/tests/mod.rs` | 3 | 91B |
| **Total** | **1801** | **62.7K** |

## Test Count

- **Total tests**: 78 (`#[test]` functions in `edge_case_tests.rs`)

## Drift Status

| Rule | Status | Finding |
|------|--------|---------|
| File size ≤ 300 lines | **VIOLATION** | 1798 lines (exceeds by 1498) |
| DDD cohesion | OK | Tests organized by operation (empty, unique, contains, etc.) |
| Primitive obsession | OK | Uses typed IDs (SymbolId, ObjectId, ListId, etc.) |

## Findings

1. **SIZE VIOLATION**: `edge_case_tests.rs` is 1798 lines — **5.99× the 300-line threshold**. This single test file holds all 78 edge case tests for 12 helper functions.

2. **Test organization**: Tests are split into 15 `mod` blocks by operation:
   - `empty_edge_cases` (5 tests)
   - `unique_edge_cases` (3 tests)
   - `contains_edge_cases` (3 tests)
   - `starts_with_edge_cases` (5 tests)
   - `ends_with_edge_cases` (5 tests)
   - `has_edge_cases` (1 test)
   - `append_edge_cases` (3 tests)
   - `append_if_edge_cases` (3 tests)
   - `sum_edge_cases` (4 tests)
   - `length_edge_cases` (5 tests)
   - `count_edge_cases` (3 tests)
   - `merge_edge_cases` (6 tests)
   - `text_ops_oob_edge_cases` (3 tests)
   - `exists_edge_cases` (5 tests)
   - `has_more_edge_cases` (3 tests)
   - `empty_more_edge_cases` (3 tests)
   - `unique_more_edge_cases` (1 test)
   - `sum_more_edge_cases` (2 tests)
   - `length_more_edge_cases` (1 test)

## Recommendation

**REFACTOR REQUIRED** — File must be split.

### Split Strategy

Split by operation group into separate files under `helpers/tests/`:

```
helpers/tests/
├── mod.rs              (3 lines) — keep pub mod exports
├── empty_tests.rs      (~170 lines, 8 tests)
├── unique_tests.rs     (~100 lines, 4 tests)
├── contains_tests.rs   (~100 lines, 3 tests)
├── starts_with_tests.rs (~200 lines, 5 tests)
├── ends_with_tests.rs  (~200 lines, 5 tests)
├── has_tests.rs        (~170 lines, 4 tests)
├── append_tests.rs     (~130 lines, 3 tests)
├── append_if_tests.rs  (~140 lines, 3 tests)
├── sum_tests.rs        (~150 lines, 6 tests)
├── length_tests.rs     (~170 lines, 6 tests)
├── count_tests.rs      (~110 lines, 3 tests)
├── merge_tests.rs      (~250 lines, 6 tests)
├── text_oob_tests.rs   (~100 lines, 3 tests)
└── exists_tests.rs     (~200 lines, 5 tests)
```

Each split file should stay under 300 lines while keeping related edge cases co-located.

### Update Plan

1. Create new files for each operation group
2. Move corresponding `mod` blocks and their tests
3. Update `mod.rs` to pub mod each new module
4. Delete `edge_case_tests.rs`

## Verdict

**STATUS: REFACTORED** (requires split)
