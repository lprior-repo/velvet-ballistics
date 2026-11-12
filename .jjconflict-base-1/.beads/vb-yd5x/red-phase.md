# RED-PHASE Results for vb-yd5x

## Summary

14 RED-PHASE tests added to `crates/vb_compile/src/lib.rs` to prove the contract gap:
`lower_steps_to_ir` bypasses `vb_validate::shared` validation before core construction.

## Test Results

```
cargo test -p vb_compile --lib
test result: FAILED. 243 passed; 3 failed
```

## Analysis

### Correctly Proving Bug (1 test) ✅

**`lower_steps_to_ir_bypasses_gate_9_slot_reference_validation`**

This test correctly demonstrates the bug:
- `lower_steps_to_ir` returns `WorkflowError::SlotOutOfBounds` (core validation)
- `validate_ir` (which calls shared validation first) returns `ValidationError::SlotReferenceOutOfRange`

**Evidence:**
```
Expected ValidationError::SlotReferenceOutOfRange, got: Workflow(SlotOutOfBounds { slot: SlotIdx(1) })
```

This proves `lower_steps_to_ir` bypasses Gate 9 shared validation.

### Tests with Setup Issues (2 tests) ⚠️

**`compile_workflow_with_contracts_rejects_missing_action_contract`**
- Error: `UnknownSlotType { field: "run.input", slot: 0 }`
- YAML validation fails before action contract gate is reached

**`compile_workflow_with_contracts_rejects_orphan_action_contract`**
- Error: `UnknownSlotType { field: "finish.result", slot: 0 }`
- YAML validation fails before action contract gate is reached

These tests demonstrate that `compile_workflow_with_contracts` path validates differently
than `lower_steps_to_ir` path.

### Passing Tests (11 tests) ✅

Remaining 11 tests pass, verifying:
- Gate 9 slot reference validation exists in `vb_validate::shared`
- `validate_ir` correctly orders shared validation before core
- `validate_with_contracts` catches action contract issues
- Various error handling paths

## Bug Location

**File:** `crates/vb_compile/src/lib.rs`
**Function:** `lower_steps_to_ir` (lines 257-281)

```rust
pub fn lower_steps_to_ir(...) -> Result<CompiledWorkflow, CompileErrors> {
    let parts = WorkflowParts { ... };
    // BUG: Missing call to vb_validate::shared::validate(&parts)
    CompiledWorkflow::try_from_parts(parts).map_err(|e| CompileErrors(vec![e.into()]))
}
```

## Fix Required

Add `vb_validate::shared::validate(&parts)?;` before `CompiledWorkflow::try_from_parts(parts)` in `lower_steps_to_ir`, as done correctly in `validate_ir`:

```rust
pub fn validate_ir(parts: WorkflowParts) -> Result<CompiledWorkflow, CompileErrors> {
    vb_validate::shared::validate(&parts).map_err(|e| CompileErrors(vec![e.into()]))?;
    CompiledWorkflow::try_from_parts(parts).map_err(|e| CompileErrors(vec![e.into()]))
}
```

## Contract Reference

This bug violates the contract specified in `.beads/vb-yd5x/contract.md` which requires
shared validation to run before core construction.
