# Architectural Drift Report: vb_core/src/replay/ops.rs

**File:** `crates/vb_core/src/replay/ops.rs`  
**Analysis Date:** 2026-05-29  
**Status:** 🚨 CRITICAL DRIFT DETECTED

---

## 1. Line Count Analysis

| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| Total lines | **2101** | < 300 | 🚨 VIOLATION (700% over) |
| Production code | ~271 | - | ✅ CLEAN |
| Test code | ~1830 | - | ❌ EXTERNAL |

**Verdict:** File exceeds 300-line limit by **1801 lines** (700% over threshold).

---

## 2. DDD Cohesion Analysis

### 2.1 Domain Boundary: ACCEPTABLE
- File correctly encapsulates **replay expression evaluation operations**
- Single responsibility: evaluating `ExprOp` variants against a `RunFrame` + `ValueStore`
- Clear domain vocabulary: `eval_load_slot`, `eval_add`, `eval_eq`, etc.

### 2.2 DDD Patterns Detected

| Pattern | Status | Notes |
|---------|--------|-------|
| Value Objects | ✅ | `SlotValue` used throughout; no raw primitives in public API |
| Domain Services | ✅ | `eval_replay_op` is a proper domain operation dispatcher |
| NewTypes | ✅ | `SlotIdx`, `ConstIdx`, `AccessorIdx`, `StepIdx` wrapped appropriately |
| `Parse, Don't Validate` | ✅ | `expect_bool_replay`, `expect_i64_replay` convert or error |
| Workflow as State Machine | ⚠️ | The `match op` dispatch is explicit but grows with ops |

### 2.3 Cohesion Violations

| Smell | Severity | Location |
|-------|----------|----------|
| **Shotgun Surgery** | MEDIUM | 19 `eval_*` functions + 3 `expect_*` + 2 `pop_*` helpers |
| **Parallel Hierarchy** | MEDIUM | Test module lines 272-2101 mirrors production function structure |
| **Feature Envy** | LOW | `eval_*` functions only operate on `SlotValue`/`ReplayExprStack` - acceptable |
| **Speculative Generality** | LOW | `ExprOp::_` catch-all returns error for unsupported ops |

---

## 3. All Violations

### 🚨 CRITICAL

1. **LINE_COUNT_EXCEEDED**
   - Required: < 300 lines
   - Actual: 2101 lines
   - Remediation: Extract test modules to separate files

### ⚠️ MEDIUM

2. **TEST_CODE_EMBEDDED**
   - 1830 lines of tests (87% of file) embedded in production module
   - Location: lines 272-2101
   - Should be: `replay/ops_tests.rs` or `replay/ops/blackhat_tests.rs`

3. **BLACKHAT_SECURITY_TESTS_NOT_ISOLATED**
   - 260 lines of security regression tests (lines 1841-2101)
   - Should be extracted to `replay/ops_security_tests.rs`
   - Labels: BH-OPS-01 through BH-OPS-07

4. **STEPIDX_ZERO_SEMANTIC_DRIFT**
   - Multiple uses of `StepIdx::ZERO` as error placeholder:
     - Line 139: `step: StepIdx::ZERO`
     - Line 149: `step: StepIdx::ZERO`
     - Line 159: `step: StepIdx::ZERO`
     - Line 169: `step: StepIdx::ZERO`
     - Line 258: `step: StepIdx::ZERO`
     - Line 267: `step: StepIdx::ZERO`
   - `StepIdx::ZERO` should not be used as a sentinel for "no step context"
   - Suggestion: Add `ReplayError::NoStepContext` variant

### ℹ️ LOW (Informational)

5. **DUPLICATED_PATTERN_expcet_*_replay**
   - `expect_bool_replay` (lines 254-261) and `expect_i64_replay` (lines 263-270)
   - Both follow identical match-on-variant structure
   - Could be generic: `fn expect_value<T>(value: SlotValue, variant: fn(V)->T)`
   - Not a bug, just minor duplication

---

## 4. File Structure Breakdown

```
ops.rs (2101 lines total)
├── Lines 1-11:    Module docs + imports
├── Lines 13-44:   eval_replay_op dispatcher (33 lines)
├── Lines 46-102:  eval_load_slot, eval_load_const, eval_load_accessor (57 lines)
├── Lines 104-192: eval_eq through eval_lte (89 lines) 
├── Lines 194-240: eval_accessor_for_replay (47 lines)
├── Lines 242-270: pop_pair, pop_i64_pair, expect_bool_replay, expect_i64_replay (29 lines)
├── Lines 272-1839: Unit tests (1568 lines)
└── Lines 1841-2101: BLACKHAT security regression tests (260 lines)
```

---

## 5. Remediation Plan

### Priority 1 (CRITICAL - Immediate)

| Action | Target | Effort |
|--------|--------|--------|
| Extract unit tests to `replay/ops_tests.rs` | 1568 lines | Medium |
| Extract BLACKHAT tests to `replay/ops_security_tests.rs` | 260 lines | Medium |
| Update `replay/mod.rs` to include new test modules | - | Low |

**Result:** `ops.rs` would be reduced to ~273 lines (within 300-line threshold)

### Priority 2 (MEDIUM - Soon)

| Action | Description |
|--------|-------------|
| Add `ReplayError::NoStepContext` variant | Replace `StepIdx::ZERO` placeholders |
| Consider `TryFrom<SlotValue>` impls | Replace `expect_*_replay` functions |

---

## 6. Summary

| Aspect | Score | Notes |
|--------|-------|-------|
| **Line Count** | ❌ FAIL | 2101 vs 300 limit |
| **DDD Cohesion** | ✅ PASS | Domain well-modeled |
| **Test Isolation** | ❌ FAIL | 87% of file is tests |
| **Type Safety** | ✅ PASS | No unsafe, no unwrap |
| **Arithmetic Safety** | ✅ PASS | All ops use `checked_*` |

**Overall Verdict:** `ops.rs` production code is architecturally sound but **must be split** to comply with file-size limits. The 1830 lines of tests are well-structured but belong in separate test modules.

---

## 7. Compliance Statement

```
🚨 STATUS: MUST REFACTOR
- File size: 2101 lines (VIOLATION)
- DDD cohesion: ACCEPTABLE  
- Test isolation: VIOLATION
- Blackhat security tests: NOT ISOLATED
```

**Required Actions:**
1. Extract `#[cfg(test)]` modules to `replay/ops_tests.rs`
2. Extract BLACKHAT tests to `replay/ops_security_tests.rs`
3. Add `ReplayError::NoStepContext` to replace `StepIdx::ZERO` semantics
4. Re-run architectural drift check after refactoring

---
*Report generated by architectural-drift agent*
