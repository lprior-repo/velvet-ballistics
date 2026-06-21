# CW-012: Expression validation clones opcode arrays just to recompute stack depth

- **Severity**: Info
- **Category**: perf
- **Location**: `crates/vb_core/src/workflow/validation/expressions.rs:12-17`
- **Confidence**: confirmed

## Description

Expression validation clones each expression's boxed opcode array only to feed `ExprProgram::try_from_parts`. The stack checker already accepts a borrowed slice, so admission can avoid this allocation and copy.

## Evidence

```rust
// expressions.rs:12
fn validate_expression(
    expression: &ExprProgram,
    accessor_count: usize,
) -> Result<(), WorkflowError> {
    ExprProgram::try_from_parts(expression.ops.clone(), expression.max_stack)?;
    validate_expression_accessors(expression, accessor_count)
}
```

The underlying stack validator is borrowed:

```rust
// expr.rs:114
pub fn check_expr_stack_bound(ops: &[ExprOp], capacity: u8) -> CoreResult<u8> {
    validate_expr_op_count(ops)?;
    ...
}
```

## Adversarial Check

This is not a hot runtime execution bug, but it is a real admission-path allocation. Workflows may carry thousands of expressions, and validation already has immutable access to each `ExprProgram`; cloning the opcode box adds memory churn without improving correctness.

## Suggested Fix

Replace the clone with a borrowed validation helper: call `check_expr_stack_bound(expression.ops.as_ref(), expression.max_stack)`, compare the computed depth to `expression.max_stack`, and map any `CoreError` into `WorkflowError::Expression` just as `try_from_parts` does.
