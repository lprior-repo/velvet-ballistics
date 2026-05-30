# Architectural Drift Report: vb_storage Journal Tests

**File**: `crates/vb_storage/src/journal/tests.rs`
**Date**: 2026-05-29
**Agent**: architectural-drift

## Summary

| Metric | Value |
|--------|-------|
| Total Lines | 2426 |
| Test Count | 90 |
| Size Category | **SEVERE DRIFT** (>300 lines) |
| Location | `crates/vb_storage/src/journal/tests.rs` |

## Drift Analysis

### Size Violation
- **Threshold**: 300 lines (architectural contract)
- **Actual**: 2426 lines
- **Violation**: 2108 lines over threshold (708% of max)
- **Status**: `REFACTOR REQUIRED`

### Structural Concerns
1. **Single monolithic test file** - 90 tests in one file violates single-responsibility
2. **Test organization** - Tests are grouped by category (round-trip, isolation, edge cases, etc.) but not split
3. **No evidence of planned splitting** - The file grows organically

### DDD Assessment (Scott Wlaschin)
- Test helpers (`temp_journal`, `make_event`, `make_step_started`, `corrupt_magic_preserving_crc`) are present
- Primitive obsession: No newtypes observed in test helpers (acceptable for tests)
- Workflow modeling: Tests exercise explicit state transitions properly

## Recommendations

### Immediate (Required)
1. **Split the test file** into logical modules:
   - `tests_roundtrip.rs` - Write/read tests for each record type
   - `tests_isolation.rs` - Keyspace isolation tests
   - `tests_sequence.rs` - Sequential ordering and gap detection
   - `tests_duplicate.rs` - Duplicate rejection tests
   - `tests_batch.rs` - Batch operation tests
   - `tests_validation.rs` - Corruption/magic/schema validation
   - `tests_edge_cases.rs` - Boundary conditions, empty journals, large payloads
   - `tests_close.rs` - Close/drop behavior tests

2. **Update `mod.rs`** in `journal/` to expose the test modules

### Suggested (Best Practice)
- Create a shared `tests/common/mod.rs` for test fixtures/helpers
- Consider `#[cfg(test)]` module organization within the main source tree
- Target 150-200 lines per test file for optimal maintainability

## Status

```
STATUS: REFACTOR REQUIRED
SEVERITY: HIGH
```

The 2426-line test file is 8x the architectural limit. Splitting is mandatory per the architectural contract.
