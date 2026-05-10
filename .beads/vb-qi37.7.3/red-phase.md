STATUS: BLOCKED

# Red Phase Evidence: vb-qi37.7.3

## Files Changed

- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_core/tests/vb_qi37_7_3_red.rs`
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_validate/tests/vb_qi37_7_3_red.rs`
- `/home/lewis/src/Velvet-ballistics/.beads/vb-qi37.7.3/red-phase.md`

No production implementation files were edited.

## Commands Run

### `rtk cargo test -p vb_core validate_symbol_references_returns -- --nocapture`

Result: compile failed for the intended red reason.

Summary:

```text
error[E0432]: unresolved imports `vb_core::workflow::validate_resource_references`, `vb_core::workflow::validate_symbol_references`
 --> crates/vb_core/tests/vb_qi37_7_3_red.rs:5:53
  |
5 |     ResourceContract, WorkflowError, WorkflowParts, validate_resource_references,
  |                                                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ no `validate_resource_references` in `workflow`
6 |     validate_symbol_references,
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^ no `validate_symbol_references` in `workflow`
```

### `rtk cargo test -p vb_validate validate_action_references_returns -- --nocapture`

Result: compile failed for the intended red reason.

Summary:

```text
error[E0432]: unresolved import `vb_validate::shared::validate_action_references`
 --> crates/vb_validate/tests/vb_qi37_7_3_red.rs:5:37
  |
5 | use vb_validate::shared::{validate, validate_action_references, validate_with_contracts};
  |                                     ^^^^^^^^^^^^^^^^^^^^^^^^^^ no `validate_action_references` in `shared`
```

### `cargo nextest run -p vb_core -p vb_validate vb_qi37_7_3_red`

Result: compile failed before test execution for the same missing contracted public APIs.

Summary:

```text
error[E0432]: unresolved import `vb_validate::shared::validate_action_references`
error[E0432]: unresolved imports `vb_core::workflow::validate_resource_references`, `vb_core::workflow::validate_symbol_references`
error: command `... cargo test --no-run --message-format json-render-diagnostics --package vb_core --package vb_validate` exited with code 101
```

## Intended Failing Tests

Added core tests directly target:

- `validate_symbol_references_returns_unit_when_all_symbol_carriers_are_in_bounds`
- `validate_symbol_references_returns_symbol_out_of_bounds_when_accessor_field_equals_symbols_count`
- `zero_symbols_rejects_accessor_symbol_zero`
- `validate_resource_references_returns_unit_when_declared_and_actual_resources_are_within_limits`
- `validate_resource_references_returns_resource_contract_too_large_when_declared_max_steps_exceeds_hard_limit`
- `validate_resource_references_returns_resource_contract_exceeded_when_node_count_exceeds_max_steps`
- pipeline/core admission symbol carrier tests for `ConstValue::Symbol` and `CompiledNodeKind::BuildObject`

Added validator tests directly target:

- `validate_action_references_returns_unit_when_unique_do_actions_equal_unique_contract_ids`
- `validate_action_references_returns_missing_contract_when_do_action_has_no_contract`
- `validate_action_references_returns_orphan_contract_when_contract_id_is_unreferenced`
- `validate_with_contracts_returns_missing_contract_when_do_action_has_no_contract`
- `validate_returns_success_for_do_action_without_contract_when_non_action_gates_pass`
- `orphan_action_contract_reports_first_orphan_in_supplied_contract_order`

## Missing API Contract Blocking Compilation

The approved contract requires these public surfaces:

```rust
pub fn validate_symbol_references(parts: &WorkflowParts) -> Result<(), WorkflowError>;
pub fn validate_resource_references(parts: &WorkflowParts) -> Result<(), WorkflowError>;
pub fn validate_action_references(parts: &WorkflowParts, action_contracts: &[ActionContract]) -> Result<(), ValidationError>;
```

Current code does not expose them at:

- `vb_core::workflow::validate_symbol_references`
- `vb_core::workflow::validate_resource_references`
- `vb_validate::shared::validate_action_references`

This is an approved red-phase compile blocker, not a weakened test.

## Non-Tautology Proof

- The direct helper imports prove the tests cannot pass unless the contracted public APIs exist.
- Error-path tests assert exact enum variants and salient fields: `SymbolOutOfBounds { symbol }`, `ResourceContractTooLarge { resource }`, `ResourceContractExceeded { resource }`, `ActionContractMissing { action_id, node_index }`, and `ActionContractOrphan { action_id }`.
- Stub implementations returning only `Ok(())` fail all negative tests.
- Stub implementations returning generic or wrong variants fail exact `assert_eq!` checks.
- Deleting symbol traversal for accessor, symbol constant, or build-object carriers fails the corresponding carrier-specific tests.
- Deleting resource hard-limit or actual-usage checks fails the direct resource helper tests.
- Replacing action-contract set equality with subset/superset checks fails missing/orphan/duplicate action scenarios.
