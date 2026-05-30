# Architectural Drift Report: `vb_expr_eval.md`

**File:** `crates/vb_expr/src/eval.rs`  
**Total Lines:** 1016  
**Limit:** 300 lines  
**Status:** VIOLATION (316% over limit)

---

## 1. Line Count Violation

| Metric | Value |
|--------|-------|
| Actual Lines | 1016 |
| Limit | 300 |
| Excess | 716 lines (239%) |
| Status | **CRITICAL** |

---

## 2. DDD Cohesion Analysis

### Domain Boundaries (Confused)

The file conflates multiple DDD domains:

| Domain | Functions | Lines | Problem |
|--------|-----------|-------|---------|
| **Stack Ops** | `push_value`, `pop_value`, `pop_pair`, `pop_triple` | ~30 | Should be `stack.rs` |
| **Binary Ops** | `eval_binary_op`, `eval_add/sub/mul/div/gt/gte/lt/lte_op` | ~150 | Should be `ops/binary.rs` |
| **Unary Ops** | `eval_unary_op`, `eval_neg_op` | ~20 | Should be `ops/unary.rs` |
| **Type Expectations** | `expect_bool/i64/symbol/list/object` | ~50 | Should be `types.rs` |
| **Helper Ops** | `eval_helper_op_with_store`, `eval_helper_with_store` | ~100 | Should be `helpers.rs` |
| **Store Helpers** | 11 `eval_helper_*_with_store` functions | ~200 | Should be `helpers/store_impls.rs` |
| **Main Evaluator** | `eval_expr_program`, `eval_expr_program_with_store` | ~50 | Could stay in `eval.rs` but smaller |

### Cohesion Score: **LOW** (0.25)

Functions share only generic `SlotValue` types, not a coherent domain concept.

---

## 3. Violations

### Critical

| # | Violation | Location | Description |
|---|-----------|----------|-------------|
| 1 | **File Size** | Lines 1-1016 | 1016 lines vs 300 max |
| 2 | **Feature Envy** | Lines 338-413, 420-479 | `eval_helper_op_with_store` and `eval_helper_with_store` do nothing but dispatch to helper impls |
| 3 | **Duplicated Code** | Lines 659-918 | 11 `*_with_store` functions have identical `store.X(id).map_err(...)?` error patterns |

### Major

| # | Violation | Location | Description |
|---|-----------|----------|-------------|
| 4 | **Primitive Obsession** | Lines 47-48, 106-130 | Raw `usize` index access instead of domain `SlotIdx`/`ConstIdx` wrapper methods |
| 5 | **Switch Statement Bloat** | Lines 86-103 | `eval_expr_op_with_store` has 17 match arms, many fall through to same helper |
| 6 | **Inconsistent Abstraction** | Lines 486-607 | `eval_helper` (no store) exists alongside `eval_helper_with_store`, duplicating dispatch logic |

### Minor

| # | Violation | Location | Description |
|---|-----------|----------|-------------|
| 7 | **Similar Functions** | Lines 609-648 | `one_arg`, `two_args`, `three_args` share identical structure |
| 8 | **Type Conversion Repetition** | Lines 985-1013 | 5 `expect_*` functions follow identical match-then-convert pattern |

---

## 4. DDD Smells

| Smell | Severity | Example |
|-------|----------|---------|
| **Primitive Obsession** | Medium | `fn eval_load_slot(..., idx: vb_core::SlotIdx)` — uses `SlotIdx` correctly here but then does `slots.get(idx.as_usize())` exposing raw usize |
| **Feature Envy** | High | `eval_helper_length_with_store` (lines 659-691) knows too much about `ValueStore` internals |
| **Duplicated Code** | High | `store.symbol(id).map_err(...)?` appears ~30 times |
| **Large Class** | Critical | Single 1016-line file doing stack management + all operations + all helpers |
| **Parallel Inheritance** | Medium | `eval_helper` mirrors `eval_helper_with_store` with degraded behavior |

---

## 5. Refactoring Plan

### Proposed Split (6 files)

```
vb_expr/src/
├── eval.rs          (~80 lines)   # Main entry, stack frame, dispatch
├── ops/
│   ├── binary.rs   (~100 lines)  # Binary ops: add, sub, mul, div, cmp
│   ├── unary.rs    (~30 lines)   # Unary ops: not, neg
│   └── mod.rs      (~10 lines)
├── stack.rs        (~40 lines)   # push, pop, pop_pair, pop_triple
├── types.rs        (~40 lines)   # expect_bool, expect_i64, etc.
├── helpers/
│   ├── mod.rs      (~30 lines)   # Arity validation, dispatch
│   └── store.rs    (~180 lines)  # All *_with_store implementations
└── mod.rs          (update exports)
```

### Priority: **CRITICAL**

This file must be split before further development. The 1016-line monolith blocks:
- Parallel work (merge conflicts)
- Testing isolation
- Tooling (verification, profiling)
- Maintainability

---

## 6. Summary

| Metric | Value |
|--------|-------|
| **Lines** | 1016 (VIOLATION) |
| **DDD Smell** | Feature Envy, Duplicated Code, Primitive Obsession |
| **Priority** | **CRITICAL** |
| **Estimated Refactor** | 6 hours (6 file split) |
