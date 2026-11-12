# Test Plan: vb-yd5x — Shared Validated IR Usage

## Context

Bead `vb-yd5x` proves that every `vb_compile` public API that assembles `WorkflowParts` into `CompiledWorkflow` runs `vb_validate::shared::validate` before core construction via `CompiledWorkflow::try_from_parts`.

## Behavior Inventory

### Subjects and Behaviors

| Subject | Action | Outcome when |
|---------|--------|--------------|
| `lower_steps_to_ir` | assembles WorkflowParts | rejects with `ValidationError::SlotReferenceOutOfRange` when Do.input >= slot_count |
| `lower_steps_to_ir` | assembles WorkflowParts | rejects with `ValidationError::ExpressionStackMismatch` when declared stack != computed |
| `lower_steps_to_ir` | assembles WorkflowParts | returns `Ok(CompiledWorkflow)` when shared gates pass |
| `lower_steps_to_ir` | output parts | pass `vb_validate::shared::validate` |
| `validate_ir` | accepts untrusted WorkflowParts | runs shared validation before core construction |
| `validate_ir` | untrusted parts with slot violation | returns `CompileError::Validation` not `Workflow` |
| `validate_ir` | valid parts | returns `Ok(CompiledWorkflow)` |
| `compile_workflow_with_contracts` | workflow with missing contract | returns `CompileError::Validation(ValidationError::ActionContractMissing)` |
| `compile_workflow_with_contracts` | workflow with orphan contract | returns `CompileError::Validation(ValidationError::ActionContractOrphan)` |
| `compile_workflow_with_contracts` | workflow with valid contract | returns `Ok(CompiledWorkflow)` |
| `plain validate` | parts with Do node but no contracts | succeeds (gate 12 not claimed) |
| `validate_with_contracts` | parts with missing contract | returns `ValidationError::ActionContractMissing` |
| `CompileError::Validation` | error preservation | exact `ValidationError` variant preserved through conversion |

## Trophy Allocation

| Behavior | Layer | Justification |
|----------|-------|---------------|
| Shared validation ordering in `lower_steps_to_ir` | Integration (`tests/`) | Verifies full pipeline with real types |
| Error variant preservation | Unit (`#[cfg(test)]` in lib) | Fast, deterministic variant matching |
| `validate_ir` ordering guarantee | Integration | Full pipeline with typed seam |
| `compile_workflow_with_contracts` gate 12 | Integration | YAML → contract validation flow |
| Diagnostic code stability | Unit | Fast, isolated code-string matching |
| Plain vs contract-aware validation separation | Unit | Only gate 12 differs; easy to isolate |

Target: ~60% integration (`test_22.rs`), ~30% unit (error preservation, code stability), ~5% e2e, ~5% static.

## BDD Scenarios

### Scenario 1: Valid workflow validates and compiles through shared IR

**Given** a minimal valid workflow YAML with a `finish` step
**When** `compile_workflow` is called
**Then** it returns `Ok(CompiledWorkflow)`
**And** `workflow.to_parts()` passes `vb_validate::shared::validate`

```
fn validate_ir_returns_valid_workflow_when_parts_pass_all_validation()
fn lower_steps_to_ir_output_passes_shared_validation()
```

### Scenario 2: Existing public validate API remains stable

**Given** the public `vb_validate::shared::validate` function
**When** called with valid `WorkflowParts`
**Then** it returns `Ok(())`
**And** its signature and error types are unchanged

```
fn plain_validate_does_not_claim_gate_12_for_missing_contracts()
```

### Scenario 3: Malformed workflow fails consistently in validate and compile

**Given** `WorkflowParts` with `Do.input >= slot_count`
**When** `lower_steps_to_ir` is called
**Then** it returns `Err(CompileErrors(...))` with `CompileError::Validation(SlotReferenceOutOfRange)`
**And** `validate_ir` returns the same error

```
fn lower_steps_to_ir_returns_slot_reference_out_of_range_when_do_input_exceeds_slot_count()
fn validate_ir_returns_slot_reference_out_of_range_before_core_acceptance()
```

**Given** `WorkflowParts` with wrong `max_stack` in expression
**When** `lower_steps_to_ir` is called
**Then** it returns `Err(CompileErrors(...))` with `CompileError::Validation(ExpressionStackMismatch)`

```
fn lower_steps_to_ir_returns_expression_stack_mismatch_when_declared_stack_is_wrong()
```

### Scenario 4: Diagnostic codes remain stable after deduplication

**Given** a compilation error from `vb_compile`
**When** `.code()` is called
**Then** it returns the same stable string as before deduplication

```
fn compile_error_preserves_slot_reference_out_of_range_variant()
fn compile_error_preserves_expression_stack_mismatch_variant()
fn compile_error_preserves_action_contract_missing_variant()
fn compile_error_preserves_action_contract_orphan_variant()
fn compile_errors_contains_exactly_one_error_for_isolated_validation_failure()
```

## Proptest Invariants

Not applicable — all validation functions deal with structured types (`WorkflowParts`, `CompiledWorkflow`) where property-based testing on arbitrary inputs is covered by integration tests with real YAML parsing.

## Fuzz Targets

No new fuzz targets required — YAML parsing fuzzing is covered by `vb_compile` integration tests. The `WorkflowParts` boundary is exercised via structured test fixtures in `test_22.rs`.

## Kani Harnesses

No Kani harnesses required — the validation logic is pure functions on copyable structs with bounded integers; Clippy and type checking provide sufficient coverage.

## Mutation Testing Checkpoints

| Mutation | Kill Test |
|----------|-----------|
| Remove `vb_validate::shared::validate` call from `lower_steps_to_ir` | `lower_steps_to_ir_returns_slot_reference_out_of_range_when_do_input_exceeds_slot_count` → FAIL |
| Swap validation order (core before shared) | `validate_ir_returns_slot_reference_out_of_range_before_core_acceptance` → FAIL |
| Remove gate 12 from `compile_workflow_with_contracts` | `compile_workflow_with_contracts_rejects_missing_action_contract` → FAIL |
| Change `CompileError::Validation` to string conversion | `compile_error_preserves_slot_reference_out_of_range_variant` → FAIL |

Target: ≥90% kill rate on validation ordering mutations.

## Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| Valid finish-only workflow | Valid YAML, no Do nodes | `Ok(CompiledWorkflow)` | integration |
| Do with oob input slot | `Do { input: slot_count }` | `Err(SlotReferenceOutOfRange)` | integration |
| Expression stack mismatch | `max_stack: 0` but loads slot | `Err(ExpressionStackMismatch)` | integration |
| Missing action contract | Do with action 7, no contracts | `Err(ActionContractMissing)` | integration |
| Orphan action contract | No Do nodes, contract for action 99 | `Err(ActionContractOrphan)` | integration |
| Empty nodes | `nodes: []` | `Err(WorkflowError::EmptyNodes)` | integration |
| Valid contract workflow | Do action 7, contract for 7 | `Ok(CompiledWorkflow)` | integration |
| Error variant preservation | Any ValidationError in CompileError | exact variant preserved | unit |
| Diagnostic code strings | CompileError variants | stable CODE strings | unit |
| Gate 12 not claimed plain | Do node, no contracts, plain validate | `Ok(())` | unit |

## Test File Location

All RED-phase acceptance tests for vb-yd5x live in:
```
crates/vb_compile/src/tests/test_22.rs
```

Helpers and fixtures:
```
crates/vb_compile/src/tests/helpers.rs
```

## Exit Criteria

- [x] `lower_steps_to_ir` calls `vb_validate::shared::validate` before `CompiledWorkflow::try_from_parts`
- [x] `validate_ir` runs shared validation before core construction (ordering guaranteed)
- [x] `compile_workflow_with_contracts` runs `validate_with_contracts` then idempotency gates
- [x] All `CompileError::Validation` failures preserve exact `ValidationError` variant
- [x] All `CompileError::Workflow` failures preserve exact `WorkflowError` variant
- [x] Plain validation does NOT claim gate 12
- [x] `moon ci` passes (10860 tests, 0 failed)