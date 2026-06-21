# CW-006: `ErrorHandler` kind `error_slot` is not slot-validated

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_core/src/workflow/validation/nodes/kinds.rs:154-156`
- **Confidence**: confirmed

## Description

`CompiledNodeKind::ErrorHandler` has its own `error_slot`, but kind validation ignores it. The common `CompiledNode.error_slot` field is validated separately and does not cover the nested `ErrorHandler` field.

## Evidence

```rust
// node.rs:181
ErrorHandler {
    /// Body step to execute.
    body: StepIdx,
    /// Handler step to route to on body failure.
    handler: StepIdx,
    /// Optional slot to write failed step index for handler consumption.
    error_slot: Option<SlotIdx>,
},
```

```rust
// common.rs:25
validate_optional_slot(node.output, parts.slot_count)?;
validate_optional_step(node.next, parts.nodes.len())?;
validate_optional_step(node.on_error, parts.nodes.len())?;
validate_optional_slot(node.error_slot, parts.slot_count)
```

```rust
// kinds.rs:154
CompiledNodeKind::ErrorHandler { body, handler, .. } => {
    validate_two_steps(*body, *handler, parts)
}
```

An `ErrorHandler { error_slot: Some(out_of_bounds), .. }` passes the kind-specific validator because the `..` pattern discards the field.

## Adversarial Check

This is not covered by common-field validation: `CompiledNode.error_slot` and `CompiledNodeKind::ErrorHandler.error_slot` are distinct fields with distinct storage locations. The kind field's doc says it is a slot reference, so it must obey the same slot bounds contract as every other `SlotIdx` in the IR.

## Suggested Fix

Destructure the field and validate it:

```rust
CompiledNodeKind::ErrorHandler { body, handler, error_slot } => {
    validate_two_steps(*body, *handler, parts)?;
    validate_optional_slot(*error_slot, parts.slot_count)
}
```
