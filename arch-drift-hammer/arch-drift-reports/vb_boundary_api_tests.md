# Architectural Drift Report: vb_boundary_inventory API Tests

**File:** `crates/vb_boundary_inventory/src/tests/api_tests.rs`
**Date:** 2026-05-29
**Status:** REFACTOR REQUIRED

---

## Metrics

| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| Total Lines | 1317 | 300 | **EXCEEDED** |
| Test Count | 84 | — | — |
| Size (bytes) | ~44 KB | — | — |
| Lines per Test (avg) | ~15.7 | — | — |

---

## Drift Analysis

### 1. File Size Violation
- **Lines:** 1317 (threshold: 300)
- **Violation:** 4.4x over the maximum
- **Rule:** `architectural-drift` enforces `<300` line files

### 2. DDD Cohesion
- Tests cover 5 public API functions:
  - `discover_boundaries`
  - `classify_boundary`
  - `required_evidence`
  - `validate_inventory`
  - `inventory_completion_status`
- Tests are organized by section headers but all in single file
- Helper functions (`test_tempdir`, `create_valid_workspace`, `make_valid_record`, `make_record_with_class`) are appropriately separated

### 3. Structural Issues
- Single monolithic test file
- 84 tests in one file violates single-responsibility principle
- Test execution order dependencies are mitigated by use of tempfile

---

## Recommendations

### Priority 1: Split by Tested Function
Create separate files:
```
src/tests/
├── api_tests.rs                    # Keep shared helpers only
├── discover_boundaries_tests.rs    # ~300 lines (9 tests)
├── classify_boundary_tests.rs      # ~300 lines (14 tests)
├── required_evidence_tests.rs      # ~300 lines (12 tests)
├── validate_inventory_tests.rs     # ~400 lines (28 tests)
└── inventory_completion_tests.rs  # ~200 lines (9 tests)
```

### Priority 2: Reduce Test Duplication
- `make_valid_record` / `make_record_with_class` can be moved to a shared test module
- `test_tempdir` helper is already shared

### Priority 3: Categorize Tests
Add test categories via custom attributes:
- `@test_unit` — Fast, no I/O
- `@test_integration` — Uses tempfile
- `@test_boundary` — File system boundary checks

---

## Verdict

**STATUS: REFACTOR REQUIRED**

File exceeds size threshold by 4.4x. Must be split before approval.

---

## Evidence

```bash
$ wc -l api_tests.rs
  1317 api_tests.rs

$ grep -c 'fn.*test' api_tests.rs  # or rtk grep -c 'fn test'
  84
```
