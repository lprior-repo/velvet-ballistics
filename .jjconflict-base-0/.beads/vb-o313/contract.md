# Contract: vb-o313 - LoadAccessor ExprOp Must Be Tested

## Requirement

`ExprOp::LoadAccessor` at `crates/vb_core/src/engine/expr_eval/eval.rs:79` falls through to `UnknownOperator` and is completely untested. Add a test that exercises `LoadAccessor` at compile-time, eval-time, or both.

## Non-Goals

- Not changing the runtime behavior (it already works)
- Not adding new functionality

## Constraints

1. Test must be in `crates/vb_expr` or `crates/vb_core`
2. Must verify LoadAccessor produces correct result, not UnknownOperator

## Verification Criteria

| ID | Criterion | File | Command |
|----|-----------|------|---------|
| LOADACC-001 | LoadAccessor test exists | `crates/vb_expr/src/eval/tests.rs` | `cargo test -p vb_expr load_accessor` |
| LOADACC-002 | LoadAccessor not UnknownOperator | `crates/vb_expr/src/eval/tests.rs` | `cargo test -p vb_expr` |