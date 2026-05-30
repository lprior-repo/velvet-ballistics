# Architectural Drift Report: vb_runtime_shard_helpers_tests

**File:** `crates/vb_runtime/src/shard/helpers/tests.rs`  
**Analysis Date:** 2026-05-29  
**Analyst:** architectural-drift agent

---

## Summary

| Metric | Value |
|--------|-------|
| Total Lines | 2125 |
| Test Count | 74 |
| Location Category | `tests.rs` (inline test module) |
| Size Threshold | 300 lines (canonical) |
| Drift Status | **VIOLATION** |

---

## Findings

### 1. File Size Violation

The file exceeds the 300-line architectural limit by **1825 lines** (608% over threshold).

- **Current size:** 2125 lines
- **Limit:** 300 lines
- **Overflow:** 1825 lines

### 2. Test Distribution

The file contains **74 individual test functions** organized into test groups:

| Group | Tests |
|-------|-------|
| `new_action_attempts` | 4 |
| `record_scheduled_attempt` | 4 |
| `seed_input_slots` | 4 |
| `validate_action_completion` | 9 |
| `advance_after_action_completion` | 3 |
| `timer_registration_required` | 5 |
| `retry_metadata_exists` | 3 |
| `retry_policy_after_action` | 6 |
| `record_retry_attempt` | 7 |
| `find_error_handler_for_failure` | 5 |
| `result_slot_for_finished_run` | 3 |
| `snapshot_from_state` | 5 |
| `advance_after_timer_fire` | 4 |
| `normalize_scheduled_ticket` | 3 |
| Edge-case fixtures + tests | 9+ |

### 3. Fixture Proliferation

The file defines **9 workflow factory functions** and **2 helper functions** that construct test workflows:

- `suspended_workflow()`
- `finished_workflow()`
- `wait_workflow()`
- `error_handler_workflow()`
- `retry_workflow()`
- `wait_event_no_timeout_workflow()`
- `wait_workflow_no_next()`
- `wait_event_with_timeout_workflow()`
- `ask_with_timeout_workflow()`
- `ask_without_timeout_workflow()`
- `error_handler_with_slot_workflow()`
- `make_run_state()`
- `ticket()`

---

## Architectural Violations

1. **Size Violation:** File is 7x the recommended maximum
2. **Single-file concentration:** All 74 tests reside in one file
3. **Fixture duplication:** Large workflow builders embedded in test file
4. **Co-location violation:** Tests should ideally be in `workspace_tests/` crate per canonical workspace structure

---

## Recommendations

### Immediate (Refactor)

1. **Split by function under test** into separate test files:
   - `test_action_attempts.rs`
   - `test_timer_logic.rs`
   - `test_retry_policy.rs`
   - `test_error_handling.rs`
   - `test_scheduling.rs`
   - `test_snapshots.rs`

2. **Extract fixtures** to a shared `test_fixtures` module or a separate `fixtures.rs` within the test directory

3. **Move complex workflow builders** to a dedicated test support module

### Target Structure

```
src/shard/helpers/
├── lib.rs
├── helpers.rs      # Helper functions (production)
├── tests/
│   ├── mod.rs
│   ├── test_action_attempts.rs    (~250 lines, ~10 tests)
│   ├── test_timer_logic.rs       (~300 lines, ~9 tests)
│   ├── test_retry_policy.rs      (~350 lines, ~13 tests)
│   ├── test_error_handling.rs    (~250 lines, ~8 tests)
│   ├── test_scheduling.rs        (~300 lines, ~10 tests)
│   ├── test_snapshots.rs         (~200 lines, ~5 tests)
│   └── fixtures.rs               (~400 lines, shared helpers)
```

---

## Risk Assessment

| Risk | Level | Notes |
|------|-------|-------|
| Maintainability | **HIGH** | 74 tests in one file is difficult to navigate |
| Merge conflicts | **HIGH** | Large file causes frequent conflicts in team environments |
| Test isolation | **MEDIUM** | Fixture sharing is ad-hoc |
| Cognitive load | **HIGH** | 2125 lines requires significant scrolling |

---

## Conclusion

**Status: DRIFT DETECTED**

This file is in severe architectural violation. It should be broken into 6-8 smaller test files with shared fixtures extracted. This is a textbook example of test file bloat that the 300-line limit is designed to prevent.
