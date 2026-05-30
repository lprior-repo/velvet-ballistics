# Architectural Drift Report: vb_runtime_primitives_reentry_tests

**File:** `crates/vb_runtime/src/primitives/reentry_tests.rs`  
**Analysis Date:** 2026-05-29  
**Analyst:** architectural-drift agent

---

## 1. Metrics Summary

| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| Total Lines | 1737 | 300 | 🔴 DRIFT |
| Unit Tests | 22 | — | — |
| Proptest Cases | 6 | — | — |
| **Total Tests** | **28** | — | — |

---

## 2. Location Category

**Category:** `vb_runtime::primitives::` (runtime loop-primitive re-entry tests)

This is the correct module for re-entry state machine tests covering:
- `for_each_next` re-entry after body completion
- `reduce_next` re-entry after body completion
- `collect_next` / `collect_page` re-entry after page body completion
- `repeat_attempt` / `repeat_check` re-entry after attempt completion

**No cross-boundary drift detected.** Tests appropriately live in `vb_runtime::primitives`.

---

## 3. File Structure Analysis

```
Lines 1-25:     Module documentation + imports
Lines 28-84:    vb_y4pa_* bug-regression tests (6 tests)
Lines 87-299:   tc_* unit tests (10 tests)
Lines 302-330:  minimal_workflow() helper
Lines 336-874:  tc005-tc014 continuation (10 tests)
Lines 880-1197: GWT-RE-* BDD scenario tests (6 tests)
Lines 1203-1237: minimal_workflow_with_const() + decode_packed() helpers
Lines 1244-1737: proptest_reentry submodule (6 property tests)
```

---

## 4. Drift Findings

### 4.1 Size Drift (🔴 CRITICAL)

**1737 lines vs 300-line threshold = 479% over limit**

This file is 5.8× the recommended maximum. It should be split into focused test modules.

### 4.2 Helper Duplication

Two near-identical helper functions exist:
- `minimal_workflow()` (line 302)
- `minimal_workflow_with_const()` (line 1203)

Both construct the same `CompiledWorkflow` structure. The proptest module has its own copy at line 1265.

**Recommendation:** Extract to `crate::primitives::test_helpers` module.

### 4.3 Proptest Module Scope

The `proptest_reentry` submodule (lines 1244–1737) adds 493 lines of property-based testing. While valuable for state-machine proof, this pushes the file significantly over the size limit.

**Recommendation:** Move proptest cases to `reentry_proptest_tests.rs` in the same directory.

---

## 5. Architectural Compliance

| Rule | Status |
|------|--------|
| File < 300 lines | 🔴 FAIL |
| DDD cohesion (primitives module) | ✅ PASS |
| Tests use `vb_core` via public re-exports | ✅ PASS |
| No unsafe/unwrap/panic | ✅ PASS (test file) |
| No cross-crate boundary violations | ✅ PASS |

---

## 6. Recommendation

**Priority:** HIGH

### Immediate Actions

1. **Split the file** into at minimum:
   - `reentry_unit_tests.rs` (~600 lines) — vb_y4pa_*, tc_*, GWT-RE_* tests
   - `reentry_proptest_tests.rs` (~500 lines) — proptest_reentry submodule
   - `reentry_helpers.rs` (~100 lines) — shared helper functions

2. **Extract duplicate helpers** to `primitives::test_helpers`:
   ```rust
   // In primitives/test_helpers.rs
   pub fn minimal_workflow() -> CompiledWorkflow { ... }
   pub fn minimal_workflow_with_const(cv: i64) -> CompiledWorkflow { ... }
   pub fn decode_packed(packed: i64) -> (u16, u16) { ... }
   ```

3. **Create integration test module** if BDD scenarios need full workflow context.

### Justification for Variance

This file was created to document and test a **critical state-machine bug** (Succeeded→Pending transition missing). The comprehensive test coverage (22 unit + 6 property) was intentionally added to:
- Capture the bug behavior (vb_y4pa_*)
- Provide regression coverage (tc_*)
- Enable formal verification (GWT-RE_* + proptest)
- Support Kani harness generation (`reentry_proofs_hammer.md` exists)

**Risk of reverting to 300-line limit:** May lose test correlation for the bug fix. Maintain grouped structure but reduce file length.

---

## 7. Test Catalog

| Test Name | Lines | Category | Purpose |
|-----------|-------|----------|---------|
| vb_y4pa_001 | 28–84 | Bug Regression | for_each two-item re-entry |
| vb_y4pa_002 | 87–139 | Bug Regression | reduce re-entry |
| vb_y4pa_003 | 142–198 | Bug Regression | collect_next re-entry |
| vb_y4pa_004 | 201–248 | Bug Regression | collect_page re-entry |
| vb_y4pa_005 | 251–273 | Bug Regression | repeat_attempt re-entry |
| vb_y4pa_006 | 276–299 | Bug Regression | repeat_check re-entry |
| tc005–tc014 | 336–874 | Unit Tests | Extended coverage |
| gwt_re1–gwt_re6 | 880–1197 | BDD Scenarios | State machine contracts |
| prop1–prop6 | 1319–1736 | Property Tests | Exhaustive state coverage |

---

**Report Generated:** architectural-drift agent  
**Next Action:** Create split plan via `planner` skill for file decomposition
