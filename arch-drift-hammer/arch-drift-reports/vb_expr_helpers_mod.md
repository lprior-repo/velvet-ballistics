# Architectural Drift Report: vb_expr helpers module

**File Analyzed:** `crates/vb_expr/src/helpers/mod.rs`
**Status:** FILE NOT FOUND

---

## 1. Line Count

| File | Lines | Limit | Status |
|------|-------|-------|--------|
| `helpers.rs` | 20 | 300 | ✅ PASS |
| `helpers/tests/mod.rs` | 3 | 300 | ✅ PASS |
| `helpers/tests/edge_case_tests.rs` | **1798** | 300 | ❌ **VIOLATION** |

---

## 2. DDD Cohesion Analysis

**Module Purpose:** Re-export helper function edge case tests for `vb_core::engine::expr_eval::ops_text_list` and `vb_core::engine::expr_eval::ops`.

**Cohesion Verdict:** LOW COHESION

**Findings:**
- `helpers.rs` acts as a pure re-export facade (20 lines) — this is acceptable as a module boundary
- The actual test code (`edge_case_tests.rs`) is **1798 lines** — violates single responsibility
- Test file mixes concerns: edge cases for 12 different helpers (contains, starts_with, ends_with, has, length, empty, sum, count, append, append_if, unique, merge)
- No domain separation within the test file — all 12 helpers' tests are in a single 1798-line file

---

## 3. Violations

| Severity | Violation | Location |
|----------|-----------|----------|
| **CRITICAL** | File exceeds 300 lines | `edge_case_tests.rs` (1798 lines) |
| **HIGH** | Low DDD cohesion — 12 distinct domain operations tested in single file | `edge_case_tests.rs` |
| **MEDIUM** | Primitive obsession — tests use raw `String`, `i64` instead of domain types | `edge_case_tests.rs` |
| **LOW** | Missing module structure — `helpers/mod.rs` does not exist (only `helpers.rs`) | Path issue |

---

## 4. Recommendations

1. **Split `edge_case_tests.rs`** into 12 separate files:
   - `eval_contains_tests.rs`
   - `eval_starts_with_tests.rs`
   - `eval_ends_with_tests.rs`
   - `eval_has_tests.rs`
   - `eval_length_tests.rs`
   - `eval_empty_tests.rs`
   - `eval_sum_tests.rs`
   - `eval_count_tests.rs`
   - `eval_append_tests.rs`
   - `eval_append_if_tests.rs`
   - `eval_unique_tests.rs`
   - `eval_merge_tests.rs`

2. **Create `helpers/mod.rs`** to properly expose the module hierarchy

3. **Introduce domain newtypes** for test inputs instead of raw `String`/`i64`

---

## 5. Summary

| Metric | Value |
|--------|-------|
| **Requested file** | `helpers/mod.rs` — NOT FOUND |
| **Actual module file** | `helpers.rs` — 20 lines ✅ |
| **Violations** | 4 (1 critical, 1 high, 1 medium, 1 low) |
| **DDD Smell** | LOW COHESION + PRIMITIVE OBSESSION |
| **Priority** | **HIGH** — `edge_case_tests.rs` must be split |

---

*Report generated: 2026-05-29*
