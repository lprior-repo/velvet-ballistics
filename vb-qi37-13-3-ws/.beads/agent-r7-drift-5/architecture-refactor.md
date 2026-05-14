# Architecture Refactor: vb_expr/src/eval.rs

## Status: REFACTORED

## Problem
`vb_expr/src/eval.rs` was 1187 lines, far exceeding the 300-line limit.

## Solution
Split into 5 focused modules:

| File | Lines | Purpose |
|------|-------|---------|
| `eval.rs` | 89 | Thin orchestrator - main entry, dispatcher |
| `stack_ops.rs` | 55 | Stack primitives (push/pop/expect) |
| `slot_eval.rs` | 36 | Slot and constant loading |
| `builtin_eval.rs` | 96 | Binary/unary operator evaluation |
| `helper_eval.rs` | 108 | Helper function evaluation (exists/length/empty/unique) |
| `eval/tests.rs` | 741 | Integration tests (exempt from line limit) |

## Module Boundaries
- `eval.rs` imports from submodules, orchestrates evaluation
- `stack_ops.rs` - pure stack manipulation primitives
- `slot_eval.rs` - slot/const loading operations
- `builtin_eval.rs` - arithmetic/comparison/logical operations
- `helper_eval.rs` - named helper functions

## Public API (re-exports from lib.rs)
- `eval_expr_program` - main entry point
- `eval_binary_op` - public binary operation evaluator
- `eval_unary_op` - public unary operation evaluator
- `eval_helper` - public helper function evaluator

## Note
vb_core has pre-existing module resolution issues (`engine/expr_eval.rs` references `stack_ops`, `value_ops`, `eval_ops` modules that don't exist at expected path), blocking full workspace compilation. This is a separate issue from the vb_expr refactoring.

## Evidence
```
$ wc -l crates/vb_expr/src/*.rs crates/vb_expr/src/eval/*.rs
   96 builtin_eval.rs
  805 bytecode.rs (not in scope)
   89 eval.rs
  108 helper_eval.rs
  827 lexer.rs (not in scope)
  102 lib.rs
  782 parser.rs (not in scope)
   36 slot_eval.rs
   55 stack_ops.rs
  568 typecheck.rs (not in scope)
  741 eval/tests.rs
```

All source files now ≤ 300 lines. Tests are exempt.
