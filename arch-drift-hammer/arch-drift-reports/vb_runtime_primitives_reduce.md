# Architectural Drift Report: `vb_runtime/primitives/reduce.rs`

**File**: `crates/vb_runtime/src/primitives/reduce.rs`  
**Analyzer**: architectural-drift skill  
**Date**: 2026-05-29

---

## 1. Line Count

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | **1025** | 300 | ❌ VIOLATION |

**Severity**: CRITICAL — file exceeds limit by **241%**.

---

## 2. DDD Cohesion Analysis

### Production Code vs. Test Code Split

| Region | Lines | % of File |
|--------|-------|-----------|
| Production (pub fn) | 1–100 | ~10% |
| Test module | 102–1025 | ~90% |

### Assessment

- **Single Responsibility Violation**: The file mixes **workflow primitive handlers** (reduce_start, reduce_next, reduce_finish) with **BDD test scenarios**.
- **Cohesion Smell**: The module name `reduce.rs` suggests a domain primitive, but 90% of its mass is behavioral tests, not implementation.
- **Inappropriate for `src/` location**: Tests at this density belong in `tests/` integration files or a sibling `reduce_test.rs` under `tests/primitives/`.

---

## 3. Violations

### 🚨 CRITICAL

| ID | Violation | Rule |
|----|-----------|------|
| V1 | **File exceeds 300 lines (1025 found)** | Hard limit |
| V2 | **Test code bloats production module** (~900 lines tests vs ~100 lines impl) | DDD cohesion |

### ⚠️ WARNING

| ID | Violation | Rule |
|----|-----------|------|
| W1 | `InternalInvariantViolation` with stringly-typed `reason` field | Prefer typed error variants |
| W2 | `#[allow(clippy::too_many_arguments)]` on `reduce_start` | Holzman Rust: limit arguments |
| W3 | Tests use `panic!`-style assertions instead of `mustpanick`-less variants | Testing hygiene |

---

## 4. DDD Smell Summary

| Smell | Severity | Description |
|-------|----------|-------------|
| **Test Blob** | 🔴 Critical | 90% of file is tests obscuring domain logic |
| **Large Function** | 🟡 Warning | `reduce_start` has 9 parameters — violates "fewer arguments" heuristic |
| **Stringly-typed Error** | 🟡 Warning | `EngineError::InternalInvariantViolation { reason: &str }` should be a proper enum variant |

---

## 5. Priority

| Priority | Rationale |
|----------|-----------|
| **P0 — MANDATORY REFACTOR** | File cannot ship at 1025 lines. Must split. |

---

## 6. Recommended Actions

1. **Move tests out** → Create `crates/vb_runtime/tests/primitives/reduce.rs` and move all `#[cfg(test)]` content there.
2. **Verify `mod.rs` exports** still work after move.
3. **Optionally suppress `too_many_arguments`** with justification in a comment, or refactor `reduce_start` to take a config struct.

---

## 7. Status

```
STATUS: REFACTOR_REQUIRED
LINES: 1025 (VIOLATION)
DDD: FAIL (test blob anti-pattern)
```

**Next Action**: Extract tests to `tests/primitives/reduce.rs`, confirm build, re-run drift check.
