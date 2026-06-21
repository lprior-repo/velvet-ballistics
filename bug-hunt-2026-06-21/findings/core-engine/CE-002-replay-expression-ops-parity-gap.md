# CE-002: Replay rejects deterministic expression operators that the engine executes

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_core/src/replay/ops.rs:21`
- **Confidence**: confirmed

## Description

Replay has a partial expression evaluator that only supports a subset of deterministic `ExprOp` values. Valid workflows using engine-supported text, list, object, or negation operators can execute normally but fail deterministic replay.

## Evidence

Replay handles only loads, boolean/integer arithmetic, comparisons, and coalesce, then rejects the rest:

```rust
match op {
    ExprOp::LoadSlot(slot) => eval_load_slot(run, slot, stack, taint_accum),
    ExprOp::LoadConst(constant) => eval_load_const(plan, constant, stack),
    ExprOp::LoadAccessor(accessor) => {
        eval_load_accessor(plan, run, store, accessor, stack, taint_accum)
    }
    ExprOp::Eq => eval_eq(stack),
    ...
    ExprOp::Coalesce => eval_coalesce(stack),
    _ => Err(ReplayError::Internal {
        reason: "unsupported expression op for replay",
    }),
}
```

The engine dispatcher supports additional deterministic operators in the same workflow IR:

```rust
ExprOp::Contains => eval_contains(stack, store),
ExprOp::StartsWith => eval_starts_with(stack, store),
ExprOp::EndsWith => eval_ends_with(stack, store),
ExprOp::Has => eval_has(stack, store),
ExprOp::Exists => eval_exists(stack, store),
ExprOp::Length => eval_length(stack, store),
ExprOp::Empty => eval_empty(stack, store),
ExprOp::Append => eval_append(stack, store),
ExprOp::AppendIf => eval_append_if(stack, store),
ExprOp::Merge => eval_merge(stack, store),
ExprOp::Sum => eval_sum(stack, store),
ExprOp::Count => eval_count(stack, store),
ExprOp::Unique => eval_unique(stack, store),
ExprOp::Neg => eval_neg(stack),
```

## Adversarial Check

The rejected operators are not non-deterministic boundaries; they operate on the frame and `ValueStore`, just like operators replay already supports. The replay module is explicitly for reconstructing deterministic slot state, so failing on valid deterministic `EvalExpr` nodes breaks crash recovery/replay parity rather than enforcing a boundary.

## Suggested Fix

Do not maintain a divergent replay expression evaluator. Reuse `engine::expr_eval::eval_expr_with_store` from replay, or implement every deterministic operator in replay with a parity test that compares engine output and replay output for each `ExprOp`.
