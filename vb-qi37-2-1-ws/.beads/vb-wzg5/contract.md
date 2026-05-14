# Contract: vb-wzg5 - Implement Division By Zero Kani Harness

## Requirement

`crates/vb_core/src/engine/expr_eval/kani_div_zero.rs` is currently a STUB. Implement a Kani proof harness that proves `eval_expr_operator` returns `EngineError::DivisionByZero` and never panics when Div or Mod encounters a zero divisor.

## Non-Goals

- Not implementing the actual division logic (already exists)
- Not adding new error variants

## Constraints

1. Kani proof must use `#[kani::proof]` attribute
2. Must verify error return, not panic
3. Must be in `crates/vb_core/src/engine/expr_eval/kani_div_zero.rs`

## Verification Criteria

| ID | Criterion | File | Command |
|----|-----------|------|---------|
| DIVZERO-001 | Div by zero returns error | `crates/vb_core/src/engine/expr_eval/kani_div_zero.rs` | `cargo kani --harness kani_div_by_zero` |
| DIVZERO-002 | Mod by zero returns error | `crates/vb_core/src/engine/expr_eval/kani_div_zero.rs` | `cargo kani --harness kani_mod_by_zero` |
| DIVZERO-003 | No panic on zero divisor | `crates/vb_core/src/engine/expr_eval/kani_div_zero.rs` | `cargo kani` |