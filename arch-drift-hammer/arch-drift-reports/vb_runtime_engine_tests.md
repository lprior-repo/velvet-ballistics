# Architectural Drift Report: vb_runtime/engine/tests.rs

**File:** `crates/vb_runtime/src/engine/tests.rs`  
**Analyzed:** 2026-05-29  
**Rule Set:** architectural-drift (< 300 lines per file)

---

## Summary

| Metric | Value |
|--------|-------|
| Total Lines | 2555 |
| Test Count | 90 |
| Size Category | **CRITICAL** (8.5x over limit) |
| Status | `DRIFT DETECTED` |

---

## Violations

### 1. File Size Violation
- **Limit:** 300 lines
- **Actual:** 2555 lines
- **Overflow:** 2255 lines (851% over limit)

### 2. DDD Cohesion Concerns
The file contains tests for multiple domain concepts co-located:
- `RetryPolicy` behavior (lines 36-69)
- `execute_retry_check` routing (lines 75-127)
- `execute_error_handler` routing (lines 133-196)
- `compute_idempotency_key` determinism (lines 202-243)
- `RuntimeSignal` equality (lines 249-321)
- `RuntimeEngineError` conversions (lines 327-416)
- `StepBudget` consumption (lines 422-435)
- `execute_do` / `execute_do_without_contract` (lines 441-628)
- `resume_action_outcome` (lines 659-781)
- `resolve_contract` (lines 787-799)
- `drive_deterministic_full` (lines 805-852)
- Black-hat security tests (lines 858-1470)
- Proptest suites (lines 1476-1515)
- Additional blackhat_engine module (lines 1542+)

---

## Test Breakdown

| Category | Count |
|----------|-------|
| Unit tests (`#[test]`) | 90 |
| Proptest harnesses | 2 |
| Helper modules | 2 (`proptests`, `blackhat_engine`) |

---

## Location Category

**`tests.rs` → Split Required**

Even though this is a test file (not production code), the 300-line rule applies to all `.rs` files. The file was already split from `engine.rs` per the header comment (line 3), but has grown beyond compliance.

---

## Recommendations

### Immediate (Required)
1. **Split into logical test modules** by domain concept:
   ```
   engine/tests/
   ├── retry_policy_tests.rs     (~70 lines, ~5 tests)
   ├── retry_check_tests.rs     (~60 lines, ~6 tests)
   ├── error_handler_tests.rs   (~70 lines, ~5 tests)
   ├── idempotency_tests.rs     (~50 lines, ~5 tests)
   ├── runtime_signal_tests.rs   (~80 lines, ~8 tests)
   ├── runtime_error_tests.rs   (~100 lines, ~8 tests)
   ├── step_budget_tests.rs     (~20 lines, ~2 tests)
   ├── execute_do_tests.rs      (~200 lines, ~4 tests)
   ├── resume_outcome_tests.rs  (~130 lines, ~5 tests)
   ├── drive_loop_tests.rs      (~50 lines, ~1 test)
   ├── evidence_tests.rs        (~180 lines, ~4 tests)
   ├── blackhat_engine_tests.rs (~900 lines, ~30 tests)
   └── proptests.rs             (~50 lines, ~2 tests)
   ```

2. **Update `engine/mod.rs`** to declare all test modules.

### Architecture Notes
- The `blackhat_engine` sub-module (lines 1542-2555) accounts for ~1000 lines and should become its own file.
- Proptests should be extracted to `proptests.rs` in the same directory.
- Each split file should have < 300 lines and focus on one behavioral domain.

---

## Verdict

**DRIFT: CRITICAL**  
**ACTION: Mandatory split before next merge**
