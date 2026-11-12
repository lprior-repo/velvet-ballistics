# Martin Fowler Test Plan: vb-qi37.5.1

## Happy Path Tests

- `validate_workflow_returns_unit_when_workflow_has_no_do_nodes_and_registry_is_empty`
- `validate_workflow_returns_unit_for_pure_action_for_every_retry_and_idempotency_combination`
- `validate_workflow_returns_unit_for_side_effecting_idempotent_external_when_retry_safe`
- `validate_workflow_returns_unit_for_side_effecting_idempotent_external_when_key_required`
- `validate_action_returns_unit_for_pure_deterministic_safe_contract`
- `validate_action_returns_unit_for_pure_at_least_once_unsafe_contract`
- `validate_action_returns_unit_for_side_effecting_idempotent_external_safe_contract`
- `validate_action_returns_unit_for_side_effecting_idempotent_external_key_required_contract`
- `is_static_returns_unit_for_pure_contract_for_all_retry_and_idempotency_values`
- `is_static_returns_unit_for_side_effecting_idempotent_external_safe_contract`
- `is_static_returns_unit_for_side_effecting_idempotent_external_key_required_contract`
- `runtime_returns_unit_when_key_required_action_has_non_empty_clean_key_slots`
- `collect_returns_unit_for_empty_contract_slice`
- `collect_returns_unit_for_all_legal_contracts`

## Error Path Tests

- `validate_workflow_returns_retry_unsafe_error_when_side_effecting_contract_is_retry_unsafe`
- `validate_workflow_returns_at_least_once_error_when_side_effecting_contract_is_at_least_once`
- `validate_workflow_returns_deterministic_pure_error_when_side_effecting_contract_declares_deterministic_pure`
- `validate_workflow_returns_action_contract_missing_when_do_node_has_no_matching_contract`
- `validate_workflow_returns_action_contract_orphan_when_registry_contract_has_no_do_node`
- `validate_action_returns_retry_unsafe_violation_with_all_fields_when_retry_is_unsafe`
- `validate_action_returns_at_least_once_violation_with_all_fields_when_idempotency_is_at_least_once`
- `validate_action_returns_deterministic_pure_violation_with_all_fields_when_side_effecting_declares_deterministic_pure`
- `collect_returns_one_boxed_retry_unsafe_violation_for_single_illegal_contract`
- `collect_returns_one_boxed_at_least_once_violation_for_single_illegal_contract`
- `collect_returns_one_boxed_deterministic_pure_violation_for_single_illegal_contract`
- `is_static_returns_retry_unsafe_violation_with_all_fields_when_retry_is_unsafe`
- `is_static_returns_at_least_once_violation_with_all_fields_when_idempotency_is_at_least_once`
- `is_static_returns_deterministic_pure_violation_with_all_fields_when_side_effecting_declares_deterministic_pure`
- `runtime_returns_missing_key_when_key_required_action_has_empty_key_slots`
- `runtime_returns_secret_in_key_when_key_slot_taint_is_secret`
- `runtime_returns_secret_in_key_when_key_slot_taint_is_derived_from_secret`

## Edge Case Tests

- `validate_workflow_returns_all_idempotency_violations_in_do_action_order_when_multiple_contracts_are_illegal`
- `collect_returns_all_boxed_violations_in_input_order_for_multiple_illegal_contracts`
- `collect_returns_same_boxed_violations_when_called_twice_with_same_input`
- `static_verifier_ignores_zero_numeric_ticket_key_when_contract_is_key_required`
- `direct_decision_table_has_no_uncovered_enum_combination`
- `validate_workflow_leaves_parts_and_contracts_equal_to_original_after_validation`
- `verifier_performs_no_external_io_when_validating_typed_contract_values`

## Contract Verification Tests

- `verifier_public_api_requires_core_action_contract_types`
- `verifier_unit_functions_do_not_mutate_contract_values`
- `proptest_pure_action_acceptance_holds_for_representative_action_ids`
- `proptest_retry_unsafe_side_effecting_contracts_report_original_action`

## Given-When-Then Scenarios

### Scenario 1: Pure workflow with empty registry
Given: a compiled workflow with no `Do` nodes and an empty contract registry
When: `validate_workflow_idempotency_contracts` runs
Then: result equals `Ok(())`

### Scenario 2: Pure action passes regardless of retry/idempotency
Given: a contract with `side_effect = SideEffect::None` and any `Idempotency` and `RetrySafety`
When: `validate_action_idempotency_contract` runs
Then: result equals `Ok(())`

### Scenario 3: Side-effecting idempotent external with Safe retry passes
Given: a contract with `side_effect = SideEffect::Writes`, `idempotency = IdempotentExternal`, `retry_safety = Safe`
When: `validate_action_idempotency_contract` runs
Then: result equals `Ok(())`

### Scenario 4: Side-effecting idempotent external with KeyRequired passes statically
Given: a contract with `side_effect = SideEffect::Sends`, `idempotency = IdempotentExternal`, `retry_safety = KeyRequired`
When: `validate_action_idempotency_contract` runs
Then: result equals `Ok(())`

### Scenario 5: Side-effecting retry unsafe is rejected
Given: a contract with `side_effect = SideEffect::Destroys`, `idempotency = IdempotentExternal`, `retry_safety = Unsafe`
When: `validate_action_idempotency_contract` runs
Then: result equals `Err(IdempotencyContractViolation::SideEffectingRetryUnsafe { action, side_effect: Destroys, idempotency: IdempotentExternal, retry_safety: Unsafe })`

### Scenario 6: Side-effecting at-least-once is rejected
Given: a contract with `side_effect = SideEffect::Creates`, `idempotency = AtLeastOnceExternal`, `retry_safety = Safe`
When: `validate_action_idempotency_contract` runs
Then: result equals `Err(IdempotencyContractViolation::SideEffectingAtLeastOnceExternal { action, side_effect: Creates, idempotency: AtLeastOnceExternal, retry_safety: Safe })`

### Scenario 7: Side-effecting deterministic pure is rejected
Given: a contract with `side_effect = SideEffect::Writes`, `idempotency = DeterministicPure`, `retry_safety = Safe`
When: `validate_action_idempotency_contract` runs
Then: result equals `Err(IdempotencyContractViolation::SideEffectingDeterministicPure { action, side_effect: Writes, idempotency: DeterministicPure, retry_safety: Safe })`

### Scenario 8: Missing contract blocks workflow proof
Given: a workflow with `Do` node for action `A` and no matching contract in registry
When: `validate_workflow_idempotency_contracts` runs
Then: result equals `Err(IdempotencyContractError::ActionContractMissing { action_id: A, node_index: N })`

### Scenario 9: Orphan contract blocks workflow proof
Given: a contract for action `B` and no `Do` node for action `B`
When: `validate_workflow_idempotency_contracts` runs
Then: result equals `Err(IdempotencyContractError::ActionContractOrphan { action_id: B })`

### Scenario 10: Multiple violations accumulate in deterministic order
Given: `Do` nodes in order `[A, B, C]` with contracts that are retry-unsafe, at-least-once, and deterministic-pure respectively
When: `validate_workflow_idempotency_contracts` runs
Then: result equals `Err(IdempotencyContractError::IdempotencyViolations(...))` with violations in order `[A, B, C]`

### Scenario 11: Key-required action requires non-empty clean key slots at runtime
Given: a key-required contract and an empty key ingredient slice
When: `verify_idempotency` runs
Then: result equals `Err(IdempotencyViolation::MissingKey(side_effect))`

### Scenario 12: Secret-tainted key slot is rejected
Given: a key slot with `Taint::Secret`
When: `validate_idempotency_key_ingredients` runs
Then: result equals `Err(IdempotencyViolation::SecretInKey(slot))`

### Scenario 13: Zero numeric ticket key is not treated as missing key
Given: a statically accepted key-required contract and `ActionTicket { idempotency_key: 0, .. }`
When: static contract validation runs
Then: result equals `Ok(())`
