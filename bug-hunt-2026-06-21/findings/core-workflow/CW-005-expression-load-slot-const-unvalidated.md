# CW-005: Expression validation never bounds-checks `LoadSlot` or `LoadConst`

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_core/src/workflow/validation/expressions.rs:20-28`
- **Confidence**: confirmed

## Description

Expression validation checks stack shape and `LoadAccessor` indices, but it never validates `ExprOp::LoadSlot` against `slot_count` or `ExprOp::LoadConst` against the constant pool. An untrusted workflow can be admitted with an expression that references an out-of-bounds slot or constant.

## Evidence

```rust
// expr.rs:42
LoadSlot(SlotIdx),
// expr.rs:44
LoadConst(ConstIdx),
// expr.rs:46
LoadAccessor(AccessorIdx),
```

```rust
// expressions.rs:25
for op in expression.ops.as_ref() {
    if let ExprOp::LoadAccessor(accessor) = op {
        validate_accessor(*accessor, accessor_count)?;
    }
}
```

`validate_parts` calls `expressions::validate_expressions(&parts.expressions, parts.accessors.len())` at `validation/mod.rs:39`, so the expression validator is not even given `slot_count` or `constants.len()`. `EvalExpr` node validation only checks that the expression index exists; it does not inspect the expression bytecode operands.

## Adversarial Check

`ExprProgram::try_from_parts` does not close this gap because it only recomputes stack depth from opcode effects. A bytecode program containing `LoadSlot(SlotIdx::new(slot_count))` followed by valid stack effects can pass stack validation while still referencing a non-existent runtime slot. The same is true for `LoadConst` and the constant pool.

## Suggested Fix

Pass `slot_count` and `constants.len()` into `validate_expressions`. Extend the opcode scan to validate `LoadSlot`, `LoadConst`, and `LoadAccessor` with the same bounds helpers used by node validation.
