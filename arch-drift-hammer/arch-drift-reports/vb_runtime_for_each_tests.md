# Architectural Drift Report: `vb_runtime::for_each_tests.rs`

## File Metadata

| Field | Value |
|-------|-------|
| **Path** | `crates/vb_runtime/src/for_each_tests.rs` |
| **Total Lines** | 1891 |
| **Test Count** | 45 |
| **Location Category** | `tests/` inline within `src/` |
| **Reviewed** | 2026-05-29 |

## Size Assessment

| Metric | Threshold | Status |
|--------|-----------|--------|
| Line count | 300 (skill rule) | ❌ **EXCEEDS** (1891 lines) |
| Test count | N/A | 45 tests |

## Findings

### 1. File Size Violation
- **Rule**: Files shall not exceed 300 lines (architectural-drift skill).
- **Actual**: 1891 lines — **6.3× over limit**.
- **Category**: `src/`-level test file (not in `tests/` compilation unit).

### 2. Test Count
- 45 `#[test]` functions covering `for_each_start`, `for_each_next`, `for_each_join`.
- Tests include: happy path, error cases, adversarial BDD, Phase 22 fanout limit verification.

### 3. Architectural Concerns

| Issue | Severity | Detail |
|-------|----------|--------|
| File size | **High** | 1891 lines in a single test file violates DDD cohesion and structural drift rules |
| Test organization | Medium | Tests are co-located with `src/` rather than in `crates/vb_runtime/tests/` |
| DDD violation | Medium | `for_each` primitives span `for_each_start`, `for_each_next`, `for_each_join` — boundary unclear |

### 4. Structural Drift Observations
- Tests directly call `super::*` imports — tightly coupled to implementation module.
- Helper `fresh_frame()` and `list_in_slot` live in `crate::test_harness`.
- `DEFAULT_FANOUT = 64` constant is duplicated in test scope (line 1063) rather than imported from `ResourceContract`.

## Recommendations

| Priority | Action |
|----------|--------|
| **P0** | **Split** `for_each_tests.rs` into perprimitive test modules: `for_each_start_tests.rs`, `for_each_next_tests.rs`, `for_each_join_tests.rs` — each ≤300 lines |
| **P1** | Move split test files to `crates/vb_runtime/tests/` (proper integration test crate) |
| **P2** | Import `DEFAULT_FANOUT` from `ResourceContract` instead of re-defining in test scope |
| **P3** | Consolidate `fresh_frame`/`list_in_slot` helpers into a shared `test_harness` module at crate root or test crate |

## Summary

```
Lines: 1891
Tests: 45
Category: src/ inline tests (non-compliant)
Recommendation: SPLIT IMMEDIATELY — target ≤300 line files
```
