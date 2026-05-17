# Architecture Refactor: vb_expr/eval.rs

## Bead: r12-drift-8

## Status: REFACTORED

## Problem
The original `crates/vb_expr/src/eval.rs` was **2,474 lines** — massively over the 300-line architectural limit. It violated:
1. **File length limit** (300 lines max)
2. **Single responsibility principle** — 15+ distinct responsibilities混在一起
3. **Scott Wlaschin DDD** — No typed domain separation, Stringly operations via format!("{op:?}")

## Solution

### Module Split

| File | Lines | Responsibility |
|------|-------|----------------|
| `eval/mod.rs` | 91 | Main entry points, stack management, re-exports |
| `eval/arity.rs` | 48 | Helper argument arity validation |
| `eval/dispatch.rs` | 68 | Bytecode operator dispatch |
| `eval/helpers.rs` | 251 | Scalar-only helper functions (type-check stubs) |
| `eval/helpers_store.rs` | 471 | Store-aware helper functions (full implementations) |

**Sibling modules (unchanged):**
| File | Lines | Responsibility |
|------|-------|----------------|
| `stack_ops.rs` | 65 | Stack primitives (push, pop, expect_bool, expect_i64, pop_triple) |
| `builtin_eval.rs` | 96 | Binary/unary operator evaluation |
| `slot_eval.rs` | 36 | Slot and constant loading |

### Key Architectural Improvements

1. **Separation of Concerns**
   - `dispatch` handles ExprOp -> evaluator routing
   - `arity` validates helper argument counts
   - `helpers` provides scalar-only (erroring) helpers
   - `helpers_store` provides full ValueStore-aware helpers

2. **Eliminated Duplication**
   - Removed duplicate `eval_load_slot`, `eval_load_const` (already in slot_eval.rs)
   - Removed duplicate `eval_eq`, `eval_binary_stack`, `eval_unary_stack` (already in builtin_eval.rs)
   - Removed duplicate `eval_binary_op`, `eval_unary_op`, `eval_i64_values`, `eval_div_values`, `eval_i64_cmp_values` (already in builtin_eval.rs)
   - Removed duplicate `push_value`, `pop_value`, `pop_pair`, `expect_bool`, `expect_i64` (already in stack_ops.rs)

3. **DDD Improvements**
   - Helper dispatch now uses typed `ExprHelper` enum instead of Stringly format!("{op:?}")
   - Store-aware vs scalar-only helpers are now separate modules
   - Type expectations (`expect_symbol`, `expect_list`, `expect_object`) are co-located with their consumers

4. **Data-Calc-Actions Compliance**
   - Stack operations (Data): `stack_ops.rs`, `slot_eval.rs`
   - Calculator functions (Calc): `builtin_eval.rs`, `helpers.rs`, `helpers_store.rs`
   - Action dispatch (Actions): `eval/mod.rs`, `eval/dispatch.rs`

## Metrics

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| eval.rs lines | 2,474 | 91 (mod.rs) | -96% |
| Largest eval file | 2,474 | 471 (helpers_store.rs) | -81% |
| Total eval/ lines | 2,474 | 929 | -62% |
| Files under 300 lines | 0/1 | 4/5 | +4 |

## Notes

1. **helpers_store.rs (471 lines)** is still over 300 lines due to 12 store-aware helper implementations. Each helper requires ~20-30 lines for error handling around ValueStore access. Further splitting by arity (1-arg, 2-arg, 3-arg helpers) was considered but rejected as it would hurt cohesion.

2. **Test files** were moved from inline in eval.rs to `eval/tests/` subdirectory, maintaining existing test structure.

3. **Public API** remains unchanged — all re-exports in `mod.rs` ensure `crate::eval::{eval_binary_op, eval_expr_program, ...}` continues to work.

## Files Changed

- `crates/vb_expr/src/eval.rs` → deleted, replaced by `eval/mod.rs`
- `crates/vb_expr/src/eval/mod.rs` → new (was eval/core.rs, renamed)
- `crates/vb_expr/src/eval/arity.rs` → new
- `crates/vb_expr/src/eval/dispatch.rs` → new
- `crates/vb_expr/src/eval/helpers.rs` → new
- `crates/vb_expr/src/eval/helpers_store.rs` → new
- `crates/vb_expr/src/stack_ops.rs` → added `pop_triple`
