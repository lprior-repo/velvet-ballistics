# Architectural Drift Report: `vb_expr/src/builtin_eval.rs`

**File**: `crates/vb_expr/src/builtin_eval.rs`  
**Analysis Date**: 2026-05-29  
**Status**: ⚠️ DRIFT DETECTED

---

## 1. Line Count Analysis

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | **349** | 300 | ❌ EXCEEDED |
| Production Code | ~98 | — | ✅ |
| Test Code | 250 | — | ⚠️ |

**Verdict**: File is **49 lines over** the 300-line limit. Primary cause: 250-line `blackhat_tests` module embedded in production file.

---

## 2. DDD Cohesion Analysis

### Cohesion Score: **WEAK**

| Function | Purpose | Domain |
|----------|---------|--------|
| `eval_eq` | Equality comparison | ✅ Domain operation |
| `eval_binary_stack` | Binary op on stack | ✅ Domain operation |
| `eval_unary_stack` | Unary op on stack | ✅ Domain operation |
| `eval_binary_op` | Binary op dispatch | ✅ Domain operation |
| `eval_unary_op` | Unary op dispatch | ✅ Domain operation |
| `eval_i64_values` | i64 arithmetic helper | ✅ Private helper |
| `eval_div_values` | Division with error mapping | ⚠️ Has bug (see §3) |
| `eval_i64_cmp_values` | i64 comparison helper | ✅ Private helper |

### Domain Flow (Correct)
```
Stack → pop_pair/pop_value → eval_*_op → push_value → Stack
```
This is a proper **workflow** with explicit state transitions.

### Primitive Obsession Check
- `SlotValue` is a proper domain type (not raw `String`/`i32`)
- `BinaryOp`/`UnaryOp` are proper enums (not raw integers)
- `ExprResult<>` is a proper error domain type
- ✅ **No primitive obsession detected**

---

## 3. Violations

### 🔴 VIOLATION 1: File Size Exceeded (MANDATORY)
- **Lines**: 349 > 300 (49 over)
- **Root Cause**: `blackhat_tests` module (250 lines) embedded in production file
- **Fix**: Extract tests to `builtin_eval_tests.rs` in `tests/` or `tests/builtin/` subdirectory

### 🔴 VIOLATION 2: eval_div_values Integer Overflow Misdiagnosis (SECURITY BUG)
- **Location**: Lines 80-87
- **Bug**: `i64::MIN / -1` returns `DivisionByZero` instead of `IntegerOverflow`
- **Evidence**: `BH-BE-001` test (lines 119-130) documents this
- **Root Cause**: `checked_div` returns `None` for both division-by-zero AND overflow case `i64::MIN / -1`
- **Impact**: Callers that distinguish error types receive wrong error
- **Fix**: Check for zero explicitly before `checked_div`, or check for `i64::MIN` and `-1` combination

### 🟡 VIOLATION 3: Test Bloat in Production Module
- **Lines**: 250 of 349 (72%) are test code
- **Principle Violated**: Single Responsibility — production logic and tests should be separate modules
- **Impact**: Reduces cohesion, makes production logic harder to navigate

### 🟡 VIOLATION 4: Misleading Module Name
- `blackhat_tests` suggests adversarial/security testing
- Tests are actually unit/property tests for overflow and type safety
- **Recommendation**: Rename to `builtin_eval_tests` or `eval_binary_op_tests`

---

## 4. DDD Smell Assessment

### Smells Detected:

| Smell | Severity | Description |
|-------|----------|-------------|
| **Bloat Module** | HIGH | 349 lines exceeds recommended 300; 72% is test code |
| **Misplaced Test Code** | MEDIUM | Tests should live in `tests/` directory, not inline |
| **Hidden Bug** | HIGH | `eval_div_values` misdiagnoses overflow as division-by-zero |

### Smells NOT Detected:
- ✅ No primitive obsession
- ✅ Proper error domain modeling (`ExprError`)
- ✅ Explicit state transitions (workflow functions)
- ✅ No "parse, don't validate" violations in public API

---

## 5. Priority Assessment

| Priority | Item | Effort |
|----------|------|--------|
| **P0** | Fix `eval_div_values` overflow misdiagnosis | Low (small logic fix) |
| **P1** | Extract `blackhat_tests` to separate file | Medium (mod.rs update) |
| **P2** | Rename `blackhat_tests` → `builtin_eval_tests` | Low |

---

## 6. Recommended Actions

1. **Immediate**: Fix `eval_div_values` to correctly map `i64::MIN / -1` → `IntegerOverflow`
2. **This Session**: Extract `#[cfg(test)]` module to `crates/vb_expr/tests/builtin_eval_tests.rs`
3. **Cleanup**: Update `crates/vb_expr/src/builtin_eval.rs` to re-export tests or remove inline tests
4. **Verify**: Run `cargo test -p vb_expr -- builtin_eval` after refactor

---

## Summary

| Metric | Result |
|--------|--------|
| **Lines Count** | 349 (❌ 49 over limit) |
| **Violations** | 4 (2 HIGH, 2 MEDIUM) |
| **DDD Smell** | Bloat Module + Misplaced Tests + Hidden Bug |
| **Priority** | P0 — Fix overflow misdiagnosis |
| **DDD Cohesion** | WEAK — Tests dominate file |

**Overall Verdict**: `builtin_eval.rs` has correct domain logic but suffers from test bloat (violating file size limit) and contains a hidden integer overflow misdiagnosis bug. The production code structure is sound DDD; the issues are organizational and one correctness bug.
