# vb-o313 Review: LoadAccessor

## Finding

The claim that `LoadAccessor` falls through to `UnknownOperator` in `vb_expr/src/eval.rs` is **INCORRECT**.

`LoadAccessor` does NOT exist in `vb_expr`. The vb_expr crate is a separate expression evaluator that handles basic operations (LoadSlot, LoadConst, arithmetic, etc.) but does not have LoadAccessor.

The actual `LoadAccessor` implementation is in **vb_core** `engine/expr_eval/core.rs:97`:

```rust
ExprOp::LoadAccessor(accessor) => {
    eval_load_accessor(plan, run, store, stack, accessor, taint_accum)
}
```

And there IS a test at `core.rs:308`:
```rust
#[test]
fn eval_load_accessor_with_empty_path_reads_root() -> Result<(), String> {
```

## Verdict

**vb-core LoadAccessor IS tested.** The gap was misidentified - LoadAccessor is not in vb_expr's scope.

## Status

**CLOSED** - No implementation required. Gap was incorrect.