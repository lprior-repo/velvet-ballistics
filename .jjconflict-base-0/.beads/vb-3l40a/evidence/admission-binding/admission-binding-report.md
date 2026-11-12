# vb-3l40a Admission/Workflow Binding Report

## Summary
Implemented focused fix for admission/workflow contract validation. The `validate_submit_admission_with_contracts` method now accepts a borrowed `CompiledWorkflow` and validates action contracts against actual workflow Do action IDs using registry semantics.

## API Change
```rust
// Before
pub(crate) fn validate_submit_admission_with_contracts(
    &self,
    run: RunId,
    digest: WorkflowDigest,
    caps: CapabilitySet,
    action_contracts: &[ActionContract],
) -> RuntimeResult<()>;

// After
pub(crate) fn validate_submit_admission_with_contracts(
    &self,
    run: RunId,
    workflow: &CompiledWorkflow,
    caps: CapabilitySet,
    action_contracts: &[ActionContract],
) -> RuntimeResult<()>;
```

## Validation Semantics
Validation uses registry semantics (index and ID matching):
1. Each contract's ID must match its index in the contracts array (registry position)
2. No duplicate action IDs
3. Well-formed contract names (non-empty)
4. Granted capabilities must cover all contract requirements
5. Each workflow Do action must have a corresponding contract at its registry index

This prevents:
- Fabricated contract IDs (e.g., ActionId 999 for a workflow with only Do 0)
- Reordered contracts that pass admission but fail at drive time
- Sparse/unbound contracts that resolve to wrong actions

## Files Changed
- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` - Updated validation method
- `crates/vb_runtime/src/runtime.rs` - Updated both submit methods to pass `&workflow`
- `crates/vb_runtime/src/shard/lifecycle_tests/contract_validation_tests.rs` - 8 new tests
- `crates/vb_runtime/src/shard/lifecycle_tests/chunk_001.rs` - Updated existing tests
- `crates/vb_runtime/src/runtime_tests.rs` - Fixed test helpers for non-Do workflows

## Test Results
```
test result: ok. 1899 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### New Tests (8)
1. `validate_contracts_accepts_valid_do_contract` - Positive: valid Do contract accepted
2. `validate_contracts_rejects_fabricated_action_id` - Negative: ID 999 vs Do 0 rejected
3. `validate_contracts_rejects_duplicate_action_ids` - Negative: duplicate IDs rejected
4. `validate_contracts_rejects_missing_capability` - Negative: missing cap rejected
5. `validate_contracts_accepts_with_capability` - Positive: satisfied cap accepted
6. `validate_contracts_rejection_is_non_mutating` - Non-mutating rejection verified
7. `validate_contracts_accepts_reused_action_ids` - Positive: reused action IDs accepted
8. Updated existing capability coverage tests in chunk_001.rs

## Design Notes
- Helper `workflow_contains_do_action` removed (replaced by inline validation)
- Validation preserves legitimate reused action IDs (same action in multiple Do nodes)
- Non-Do workflows use non-contracts submit path (no validation needed)
- All validation occurs before enqueue (fail before journal mutation)
