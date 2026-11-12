# Test Plan: vb-qi37.5.1 — verifier idempotency contract model

## Summary

- Review repair status: this repaired plan explicitly addresses every rejection in `.beads/vb-qi37.5.1/test-plan-review.md`.
- Rejection 1 addressed: direct BDD scenarios are added for `validate_action_idempotency_contract` success, pure acceptance, `SideEffectingRetryUnsafe`, `SideEffectingAtLeastOnceExternal`, and `SideEffectingDeterministicPure`.
- Rejection 2 addressed: direct BDD scenarios are added for `collect_idempotency_contract_violations` empty input, all-legal input, single illegal input, multiple illegal input, deterministic order, and exact `IdempotencyContractErrors(Box<[...])` contents.
- Rejection 3 addressed: direct BDD scenarios are added for `is_statically_idempotent_contract` pure acceptance, side-effecting accepted shape, and each exact violation variant.
- Rejection 4 addressed: planned unit allocation is raised from 7 to 27 unit tests, exceeding the required floor of `5 × 4 public functions = 20`.
- Behaviors identified: 36
- Trophy allocation: 27 unit / 14 integration / 2 e2e / 6 static gates
- Proptest invariants: 10
- Fuzz targets: 2
- Kani harnesses: 5
- Mutation target: `cargo-mutants` kill rate >= 90% for touched crates and 100% for idempotency decision-table branches.
- Canonical command gate: `moon ci`

All tests must assert exact values: exact `Ok(())`, exact typed error enum variant, exact violation fields, exact boxed violation order, exact diagnostic reason category, or exact CLI/certificate proof status. No planned assertion may be only `is_ok()` or `is_err()`.

## 1. Behavior Inventory

1. Canonical model accepts only `vb_core::action::ActionContract`, `Idempotency`, `SideEffect`, and `RetrySafety` when verifier idempotency validation evaluates contracts.
2. `validate_workflow_idempotency_contracts` accepts a workflow with no `Do` nodes and an empty contract registry.
3. `validate_workflow_idempotency_contracts` accepts pure action contracts for every idempotency and retry-safety combination.
4. `validate_workflow_idempotency_contracts` accepts side-effecting `IdempotentExternal` contracts when retry safety is `Safe`.
5. `validate_workflow_idempotency_contracts` accepts side-effecting `IdempotentExternal` contracts when retry safety is `KeyRequired`.
6. `validate_workflow_idempotency_contracts` rejects side-effecting contracts when retry safety is `Unsafe`.
7. `validate_workflow_idempotency_contracts` rejects side-effecting contracts when idempotency is `AtLeastOnceExternal`.
8. `validate_workflow_idempotency_contracts` rejects side-effecting contracts when idempotency is `DeterministicPure`.
9. `validate_workflow_idempotency_contracts` accumulates all workflow-relevant idempotency violations in deterministic `Do` action traversal order.
10. `validate_workflow_idempotency_contracts` rejects a `Do` node whose action contract is missing.
11. `validate_workflow_idempotency_contracts` rejects a workflow-specific orphan contract.
12. `validate_workflow_idempotency_contracts` returns a completeness error before claiming idempotency proof when Gate 12 fails.
13. `validate_workflow_idempotency_contracts` does not mutate `WorkflowParts` or `ActionContract` inputs.
14. `validate_workflow_idempotency_contracts` performs no external I/O, network access, filesystem writes, parser work, or action dispatch.
15. `validate_action_idempotency_contract` accepts a pure action contract regardless of idempotency and retry safety.
16. `validate_action_idempotency_contract` accepts side-effecting `IdempotentExternal` contracts when retry safety is `Safe`.
17. `validate_action_idempotency_contract` accepts side-effecting `IdempotentExternal` contracts when retry safety is `KeyRequired`.
18. `validate_action_idempotency_contract` rejects side-effecting `RetrySafety::Unsafe` with `SideEffectingRetryUnsafe`.
19. `validate_action_idempotency_contract` rejects side-effecting `AtLeastOnceExternal` with `SideEffectingAtLeastOnceExternal`.
20. `validate_action_idempotency_contract` rejects side-effecting `DeterministicPure` with `SideEffectingDeterministicPure`.
21. `collect_idempotency_contract_violations` returns `Ok(())` for an empty contract slice.
22. `collect_idempotency_contract_violations` returns `Ok(())` for all-legal contracts.
23. `collect_idempotency_contract_violations` returns one exact boxed violation for one illegal contract.
24. `collect_idempotency_contract_violations` returns every exact boxed violation for multiple illegal contracts.
25. `collect_idempotency_contract_violations` preserves deterministic input traversal order for violations.
26. `is_statically_idempotent_contract` accepts pure contracts for every idempotency and retry-safety combination.
27. `is_statically_idempotent_contract` accepts side-effecting contracts iff idempotency is `IdempotentExternal` and retry safety is `Safe` or `KeyRequired`.
28. `is_statically_idempotent_contract` rejects side-effecting `RetrySafety::Unsafe` with `SideEffectingRetryUnsafe`.
29. `is_statically_idempotent_contract` rejects side-effecting `AtLeastOnceExternal` with `SideEffectingAtLeastOnceExternal`.
30. `is_statically_idempotent_contract` rejects side-effecting `DeterministicPure` with `SideEffectingDeterministicPure`.
31. Static key-required acceptance remains separate from runtime key-ingredient validation.
32. Runtime idempotency validation rejects empty key slots for key-required actions with `IdempotencyViolation::MissingKey(side_effect)`.
33. Runtime idempotency key validation rejects `Taint::Secret` and `Taint::DerivedFromSecret` with `IdempotencyViolation::SecretInKey(slot)`.
34. Static verifier never treats numeric `ActionTicket.idempotency_key == 0` as key absence.
35. CLI `verify` does not claim successful idempotency contract proof unless a real workflow-specific contract registry participates.
36. IPC certificate generation does not claim successful idempotency contract proof when a `Do` workflow has an empty registry.

## 2. Trophy Allocation

| Behaviors | Layer | Planned test count | Target location | Justification |
|---|---:|---:|---|---|
| 1 | Static/API | 1 | compile/API tests | Canonical public signatures prevent divergent enums. |
| 2-14 | Integration | 12 | `crates/vb_validate/tests/` | Real `WorkflowParts` plus real `ActionContract` values validate Gate 12 and verifier interaction. |
| 15-30 | Unit | 21 | idempotency verifier module unit tests | Direct public pure decision APIs need fast exhaustive exact assertions. |
| 31-34 | Unit + integration | 6 | `vb_core` action tests and cross-crate tests | Runtime key behavior is pure but must remain separate from static proof. |
| 35-36 | E2E + integration | 2 | CLI tests and IPC certificate tests | User-facing proof claims must be black-box observable. |
| Resource and panic constraints | Static | 6 gates | `moon ci`, clippy/source lints, `cargo-mutants`, fuzz, Kani, forbidden-token scans | Safety guarantees are structural and must run in CI. |

### Unit test density requirement

The contract exposes 4 public functions, so this plan requires at least 20 unit tests. Planned unit tests: 27.

Required unit test names:

1. `fn validate_action_returns_unit_for_pure_deterministic_safe_contract()`
2. `fn validate_action_returns_unit_for_pure_at_least_once_unsafe_contract()`
3. `fn validate_action_returns_unit_for_side_effecting_idempotent_external_safe_contract()`
4. `fn validate_action_returns_unit_for_side_effecting_idempotent_external_key_required_contract()`
5. `fn validate_action_returns_retry_unsafe_violation_with_all_fields_when_retry_is_unsafe()`
6. `fn validate_action_returns_at_least_once_violation_with_all_fields_when_idempotency_is_at_least_once()`
7. `fn validate_action_returns_deterministic_pure_violation_with_all_fields_when_side_effecting_declares_deterministic_pure()`
8. `fn collect_returns_unit_for_empty_contract_slice()`
9. `fn collect_returns_unit_for_all_legal_contracts()`
10. `fn collect_returns_one_boxed_retry_unsafe_violation_for_single_illegal_contract()`
11. `fn collect_returns_one_boxed_at_least_once_violation_for_single_illegal_contract()`
12. `fn collect_returns_one_boxed_deterministic_pure_violation_for_single_illegal_contract()`
13. `fn collect_returns_all_boxed_violations_in_input_order_for_multiple_illegal_contracts()`
14. `fn collect_returns_same_boxed_violations_when_called_twice_with_same_input()`
15. `fn is_static_returns_unit_for_pure_contract_for_all_retry_and_idempotency_values()`
16. `fn is_static_returns_unit_for_side_effecting_idempotent_external_safe_contract()`
17. `fn is_static_returns_unit_for_side_effecting_idempotent_external_key_required_contract()`
18. `fn is_static_returns_retry_unsafe_violation_with_all_fields_when_retry_is_unsafe()`
19. `fn is_static_returns_at_least_once_violation_with_all_fields_when_idempotency_is_at_least_once()`
20. `fn is_static_returns_deterministic_pure_violation_with_all_fields_when_side_effecting_declares_deterministic_pure()`
21. `fn runtime_returns_missing_key_when_key_required_action_has_empty_key_slots()`
22. `fn runtime_returns_secret_in_key_when_key_slot_taint_is_secret()`
23. `fn runtime_returns_secret_in_key_when_key_slot_taint_is_derived_from_secret()`
24. `fn runtime_returns_unit_when_key_required_action_has_non_empty_clean_key_slots()`
25. `fn static_verifier_ignores_zero_numeric_ticket_key_when_contract_is_key_required()`
26. `fn direct_decision_table_has_no_uncovered_enum_combination()`
27. `fn verifier_unit_functions_do_not_mutate_contract_values()`

## 3. BDD Scenarios

### Behavior: canonical model uses core action enums
Test function: `fn verifier_public_api_requires_core_action_contract_types()`
Given: code imports `vb_core::action::{ActionContract, Idempotency, SideEffect, RetrySafety}`.
When: it calls each public verifier function with `ActionContract` values.
Then: the code compiles only through the canonical core types and returns `Result<(), IdempotencyContractError>`, `Result<(), IdempotencyContractViolation>`, or `Result<(), IdempotencyContractErrors>` as specified.
Layer: static/API.

### Behavior: workflow accepts empty registry for empty workflow
Test function: `fn validate_workflow_returns_unit_when_workflow_has_no_do_nodes_and_registry_is_empty()`
Given: `WorkflowParts` has zero `CompiledNodeKind::Do` nodes and `action_contracts == []`.
When: `validate_workflow_idempotency_contracts(&parts, &[])` runs.
Then: result equals `Ok(())`.
Layer: integration.

### Behavior: workflow accepts pure action for all combinations
Test function: `fn validate_workflow_returns_unit_for_pure_action_for_every_retry_and_idempotency_combination()`
Given: one `Do` node for action `A` and one matching contract with `side_effect = SideEffect::None`.
When: workflow validation runs for every `Idempotency::{DeterministicPure, IdempotentExternal, AtLeastOnceExternal}` and every `RetrySafety::{Safe, KeyRequired, Unsafe}`.
Then: each result equals `Ok(())`.
Layer: integration plus unit matrix.

### Behavior: workflow accepts side-effecting idempotent external safe
Test function: `fn validate_workflow_returns_unit_for_side_effecting_idempotent_external_when_retry_safe()`
Given: action `A` contract is `{ side_effect: SideEffect::Writes, idempotency: Idempotency::IdempotentExternal, retry_safety: RetrySafety::Safe }`.
When: workflow validation runs with a matching `Do` node.
Then: result equals `Ok(())`.
Layer: integration.

### Behavior: workflow accepts side-effecting idempotent external key required
Test function: `fn validate_workflow_returns_unit_for_side_effecting_idempotent_external_when_key_required()`
Given: action `A` contract is `{ side_effect: SideEffect::Sends, idempotency: Idempotency::IdempotentExternal, retry_safety: RetrySafety::KeyRequired }`.
When: workflow validation runs with a matching `Do` node.
Then: result equals `Ok(())` and no runtime key slots are read.
Layer: integration.

### Behavior: workflow rejects side-effecting retry unsafe
Test function: `fn validate_workflow_returns_retry_unsafe_error_when_side_effecting_contract_is_retry_unsafe()`
Given: action `A` contract is `{ side_effect: SideEffect::Destroys, idempotency: Idempotency::IdempotentExternal, retry_safety: RetrySafety::Unsafe }`.
When: workflow validation runs with a matching `Do` node.
Then: result equals `Err(IdempotencyContractError::IdempotencyViolations(IdempotencyContractErrors(Box::from([IdempotencyContractViolation::SideEffectingRetryUnsafe { action: A, side_effect: SideEffect::Destroys, idempotency: Idempotency::IdempotentExternal, retry_safety: RetrySafety::Unsafe }]))))` and diagnostic reason equals `IDEMPOTENCY_RETRY_UNSAFE`.
Layer: integration.

### Behavior: workflow rejects side-effecting at-least-once external
Test function: `fn validate_workflow_returns_at_least_once_error_when_side_effecting_contract_is_at_least_once()`
Given: action `A` contract is `{ side_effect: SideEffect::Creates, idempotency: Idempotency::AtLeastOnceExternal, retry_safety: RetrySafety::Safe }`.
When: workflow validation runs with a matching `Do` node.
Then: result equals `Err(IdempotencyContractError::IdempotencyViolations(IdempotencyContractErrors(Box::from([IdempotencyContractViolation::SideEffectingAtLeastOnceExternal { action: A, side_effect: SideEffect::Creates, idempotency: Idempotency::AtLeastOnceExternal, retry_safety: RetrySafety::Safe }]))))` and diagnostic reason equals `IDEMPOTENCY_AT_LEAST_ONCE_EXTERNAL`.
Layer: integration.

### Behavior: workflow rejects side-effecting deterministic pure
Test function: `fn validate_workflow_returns_deterministic_pure_error_when_side_effecting_contract_declares_deterministic_pure()`
Given: action `A` contract is `{ side_effect: SideEffect::Writes, idempotency: Idempotency::DeterministicPure, retry_safety: RetrySafety::Safe }`.
When: workflow validation runs with a matching `Do` node.
Then: result equals `Err(IdempotencyContractError::IdempotencyViolations(IdempotencyContractErrors(Box::from([IdempotencyContractViolation::SideEffectingDeterministicPure { action: A, side_effect: SideEffect::Writes, idempotency: Idempotency::DeterministicPure, retry_safety: RetrySafety::Safe }]))))` and diagnostic reason equals `IDEMPOTENCY_SIDE_EFFECTING_DETERMINISTIC_PURE`.
Layer: integration.

### Behavior: workflow accumulates multiple idempotency violations
Test function: `fn validate_workflow_returns_all_idempotency_violations_in_do_action_order_when_multiple_contracts_are_illegal()`
Given: `Do` nodes appear in order `[A, B, C]`; contract `A` is retry unsafe, `B` is at-least-once external, and `C` is deterministic-pure side-effecting.
When: workflow validation runs.
Then: result equals `Err(IdempotencyContractError::IdempotencyViolations(IdempotencyContractErrors(Box::from([A_retry_unsafe, B_at_least_once, C_deterministic_pure]))))` where each element is the exact typed variant with its original fields.
Layer: integration.

### Behavior: workflow rejects missing contract
Test function: `fn validate_workflow_returns_action_contract_missing_when_do_node_has_no_matching_contract()`
Given: node index `0` is `CompiledNodeKind::Do { action: A, .. }` and the registry has no contract for `A`.
When: workflow validation runs.
Then: result equals `Err(IdempotencyContractError::ActionContractMissing { action_id: A, node_index: 0 })` and no idempotency success proof is emitted.
Layer: integration.

### Behavior: workflow rejects orphan contract
Test function: `fn validate_workflow_returns_action_contract_orphan_when_registry_contract_has_no_do_node()`
Given: workflow contains no `Do` node for action `B` and registry contains contract `B`.
When: workflow validation runs.
Then: result equals `Err(IdempotencyContractError::ActionContractOrphan { action_id: B })` and no idempotency success proof is emitted.
Layer: integration.

### Behavior: completeness failure precedes proof claim
Test function: `fn validate_workflow_returns_completeness_error_without_claiming_proof_when_gate_12_fails()`
Given: action `A` is missing its contract and unrelated contract `B` is idempotency-illegal.
When: workflow validation runs.
Then: result equals `Err(IdempotencyContractError::ActionContractMissing { action_id: A, node_index: 0 })`; certificate/proof state is absent or unavailable, never successful.
Layer: integration.

### Behavior: validate_action accepts pure contract directly
Test function: `fn validate_action_returns_unit_for_pure_deterministic_safe_contract()`
Given: an `ActionContract` with `side_effect = SideEffect::None`, `idempotency = Idempotency::DeterministicPure`, `retry_safety = RetrySafety::Safe`.
When: `validate_action_idempotency_contract(&contract)` runs.
Then: result equals `Ok(())`.
Layer: unit.

### Behavior: validate_action accepts pure unsafe at-least-once directly
Test function: `fn validate_action_returns_unit_for_pure_at_least_once_unsafe_contract()`
Given: an `ActionContract` with `side_effect = SideEffect::None`, `idempotency = Idempotency::AtLeastOnceExternal`, `retry_safety = RetrySafety::Unsafe`.
When: `validate_action_idempotency_contract(&contract)` runs.
Then: result equals `Ok(())`.
Layer: unit.

### Behavior: validate_action accepts side-effecting idempotent external safe directly
Test function: `fn validate_action_returns_unit_for_side_effecting_idempotent_external_safe_contract()`
Given: contract `{ action: A, side_effect: SideEffect::Writes, idempotency: Idempotency::IdempotentExternal, retry_safety: RetrySafety::Safe }`.
When: `validate_action_idempotency_contract(&contract)` runs.
Then: result equals `Ok(())`.
Layer: unit.

### Behavior: validate_action accepts side-effecting idempotent external key required directly
Test function: `fn validate_action_returns_unit_for_side_effecting_idempotent_external_key_required_contract()`
Given: contract `{ action: A, side_effect: SideEffect::Sends, idempotency: Idempotency::IdempotentExternal, retry_safety: RetrySafety::KeyRequired }`.
When: `validate_action_idempotency_contract(&contract)` runs.
Then: result equals `Ok(())`.
Layer: unit.

### Behavior: validate_action returns retry unsafe violation directly
Test function: `fn validate_action_returns_retry_unsafe_violation_with_all_fields_when_retry_is_unsafe()`
Given: contract `{ action: A, side_effect: SideEffect::Destroys, idempotency: Idempotency::IdempotentExternal, retry_safety: RetrySafety::Unsafe }`.
When: `validate_action_idempotency_contract(&contract)` runs.
Then: result equals `Err(IdempotencyContractViolation::SideEffectingRetryUnsafe { action: A, side_effect: SideEffect::Destroys, idempotency: Idempotency::IdempotentExternal, retry_safety: RetrySafety::Unsafe })`.
Layer: unit.

### Behavior: validate_action returns at-least-once violation directly
Test function: `fn validate_action_returns_at_least_once_violation_with_all_fields_when_idempotency_is_at_least_once()`
Given: contract `{ action: A, side_effect: SideEffect::Creates, idempotency: Idempotency::AtLeastOnceExternal, retry_safety: RetrySafety::Safe }`.
When: `validate_action_idempotency_contract(&contract)` runs.
Then: result equals `Err(IdempotencyContractViolation::SideEffectingAtLeastOnceExternal { action: A, side_effect: SideEffect::Creates, idempotency: Idempotency::AtLeastOnceExternal, retry_safety: RetrySafety::Safe })`.
Layer: unit.

### Behavior: validate_action returns deterministic-pure violation directly
Test function: `fn validate_action_returns_deterministic_pure_violation_with_all_fields_when_side_effecting_declares_deterministic_pure()`
Given: contract `{ action: A, side_effect: SideEffect::Writes, idempotency: Idempotency::DeterministicPure, retry_safety: RetrySafety::KeyRequired }`.
When: `validate_action_idempotency_contract(&contract)` runs.
Then: result equals `Err(IdempotencyContractViolation::SideEffectingDeterministicPure { action: A, side_effect: SideEffect::Writes, idempotency: Idempotency::DeterministicPure, retry_safety: RetrySafety::KeyRequired })`.
Layer: unit.

### Behavior: collect accepts empty input directly
Test function: `fn collect_returns_unit_for_empty_contract_slice()`
Given: `action_contracts == []`.
When: `collect_idempotency_contract_violations(&[])` runs.
Then: result equals `Ok(())`.
Layer: unit.

### Behavior: collect accepts all legal contracts directly
Test function: `fn collect_returns_unit_for_all_legal_contracts()`
Given: `[pure_unsafe_A, side_effecting_idempotent_safe_B, side_effecting_idempotent_key_required_C]`.
When: `collect_idempotency_contract_violations(&contracts)` runs.
Then: result equals `Ok(())`.
Layer: unit.

### Behavior: collect returns one boxed retry-unsafe violation directly
Test function: `fn collect_returns_one_boxed_retry_unsafe_violation_for_single_illegal_contract()`
Given: `[contract_A]` where `contract_A = { action: A, side_effect: SideEffect::Destroys, idempotency: Idempotency::IdempotentExternal, retry_safety: RetrySafety::Unsafe }`.
When: `collect_idempotency_contract_violations(&contracts)` runs.
Then: result equals `Err(IdempotencyContractErrors(Box::from([IdempotencyContractViolation::SideEffectingRetryUnsafe { action: A, side_effect: SideEffect::Destroys, idempotency: Idempotency::IdempotentExternal, retry_safety: RetrySafety::Unsafe }])))`.
Layer: unit.

### Behavior: collect returns one boxed at-least-once violation directly
Test function: `fn collect_returns_one_boxed_at_least_once_violation_for_single_illegal_contract()`
Given: `[contract_A]` where `contract_A = { action: A, side_effect: SideEffect::Creates, idempotency: Idempotency::AtLeastOnceExternal, retry_safety: RetrySafety::Safe }`.
When: `collect_idempotency_contract_violations(&contracts)` runs.
Then: result equals `Err(IdempotencyContractErrors(Box::from([IdempotencyContractViolation::SideEffectingAtLeastOnceExternal { action: A, side_effect: SideEffect::Creates, idempotency: Idempotency::AtLeastOnceExternal, retry_safety: RetrySafety::Safe }])))`.
Layer: unit.

### Behavior: collect returns one boxed deterministic-pure violation directly
Test function: `fn collect_returns_one_boxed_deterministic_pure_violation_for_single_illegal_contract()`
Given: `[contract_A]` where `contract_A = { action: A, side_effect: SideEffect::Writes, idempotency: Idempotency::DeterministicPure, retry_safety: RetrySafety::Safe }`.
When: `collect_idempotency_contract_violations(&contracts)` runs.
Then: result equals `Err(IdempotencyContractErrors(Box::from([IdempotencyContractViolation::SideEffectingDeterministicPure { action: A, side_effect: SideEffect::Writes, idempotency: Idempotency::DeterministicPure, retry_safety: RetrySafety::Safe }])))`.
Layer: unit.

### Behavior: collect returns multiple boxed violations in order directly
Test function: `fn collect_returns_all_boxed_violations_in_input_order_for_multiple_illegal_contracts()`
Given: contracts are `[A_retry_unsafe, legal_B, C_at_least_once, D_deterministic_pure]` in that input order.
When: `collect_idempotency_contract_violations(&contracts)` runs.
Then: result equals `Err(IdempotencyContractErrors(Box::from([A_retry_unsafe_violation, C_at_least_once_violation, D_deterministic_pure_violation])))` with no element for legal `B`.
Layer: unit.

### Behavior: collect diagnostics are deterministic directly
Test function: `fn collect_returns_same_boxed_violations_when_called_twice_with_same_input()`
Given: the same contract slice containing at least three illegal contracts.
When: `collect_idempotency_contract_violations(&contracts)` runs twice.
Then: both returned `Err(IdempotencyContractErrors(...))` values have identical length, identical variant sequence, identical action IDs, and identical enum fields.
Layer: unit.

### Behavior: is_static accepts pure contracts directly
Test function: `fn is_static_returns_unit_for_pure_contract_for_all_retry_and_idempotency_values()`
Given: a contract with `side_effect = SideEffect::None`.
When: `is_statically_idempotent_contract(&contract)` runs for all idempotency and retry-safety variants.
Then: each result equals `Ok(())`.
Layer: unit.

### Behavior: is_static accepts side-effecting idempotent external safe directly
Test function: `fn is_static_returns_unit_for_side_effecting_idempotent_external_safe_contract()`
Given: contract `{ action: A, side_effect: SideEffect::Writes, idempotency: Idempotency::IdempotentExternal, retry_safety: RetrySafety::Safe }`.
When: `is_statically_idempotent_contract(&contract)` runs.
Then: result equals `Ok(())`.
Layer: unit.

### Behavior: is_static accepts side-effecting idempotent external key required directly
Test function: `fn is_static_returns_unit_for_side_effecting_idempotent_external_key_required_contract()`
Given: contract `{ action: A, side_effect: SideEffect::Sends, idempotency: Idempotency::IdempotentExternal, retry_safety: RetrySafety::KeyRequired }`.
When: `is_statically_idempotent_contract(&contract)` runs.
Then: result equals `Ok(())`.
Layer: unit.

### Behavior: is_static returns retry unsafe violation directly
Test function: `fn is_static_returns_retry_unsafe_violation_with_all_fields_when_retry_is_unsafe()`
Given: contract `{ action: A, side_effect: SideEffect::Destroys, idempotency: Idempotency::IdempotentExternal, retry_safety: RetrySafety::Unsafe }`.
When: `is_statically_idempotent_contract(&contract)` runs.
Then: result equals `Err(IdempotencyContractViolation::SideEffectingRetryUnsafe { action: A, side_effect: SideEffect::Destroys, idempotency: Idempotency::IdempotentExternal, retry_safety: RetrySafety::Unsafe })`.
Layer: unit.

### Behavior: is_static returns at-least-once violation directly
Test function: `fn is_static_returns_at_least_once_violation_with_all_fields_when_idempotency_is_at_least_once()`
Given: contract `{ action: A, side_effect: SideEffect::Creates, idempotency: Idempotency::AtLeastOnceExternal, retry_safety: RetrySafety::KeyRequired }`.
When: `is_statically_idempotent_contract(&contract)` runs.
Then: result equals `Err(IdempotencyContractViolation::SideEffectingAtLeastOnceExternal { action: A, side_effect: SideEffect::Creates, idempotency: Idempotency::AtLeastOnceExternal, retry_safety: RetrySafety::KeyRequired })`.
Layer: unit.

### Behavior: is_static returns deterministic-pure violation directly
Test function: `fn is_static_returns_deterministic_pure_violation_with_all_fields_when_side_effecting_declares_deterministic_pure()`
Given: contract `{ action: A, side_effect: SideEffect::Writes, idempotency: Idempotency::DeterministicPure, retry_safety: RetrySafety::Safe }`.
When: `is_statically_idempotent_contract(&contract)` runs.
Then: result equals `Err(IdempotencyContractViolation::SideEffectingDeterministicPure { action: A, side_effect: SideEffect::Writes, idempotency: Idempotency::DeterministicPure, retry_safety: RetrySafety::Safe })`.
Layer: unit.

### Behavior: runtime key-required empty slots rejects
Test function: `fn runtime_returns_missing_key_when_key_required_action_has_empty_key_slots()`
Given: key-required side-effecting contract and an empty key ingredient slice.
When: `verify_idempotency(&contract, &[], &frame)` runs.
Then: result equals `Err(IdempotencyViolation::MissingKey(contract.side_effect))`.
Layer: unit.

### Behavior: runtime secret taint rejects
Test function: `fn runtime_returns_secret_in_key_when_key_slot_taint_is_secret()`
Given: key slot `S` has `Taint::Secret`.
When: `validate_idempotency_key_ingredients(&[S], &frame)` runs.
Then: result equals `Err(IdempotencyViolation::SecretInKey(u32::from(S.get())))`.
Layer: unit.

### Behavior: runtime derived-secret taint rejects
Test function: `fn runtime_returns_secret_in_key_when_key_slot_taint_is_derived_from_secret()`
Given: key slot `S` has `Taint::DerivedFromSecret`.
When: `validate_idempotency_key_ingredients(&[S], &frame)` runs.
Then: result equals `Err(IdempotencyViolation::SecretInKey(u32::from(S.get())))`.
Layer: unit.

### Behavior: zero numeric ticket key is not static absence
Test function: `fn static_verifier_ignores_zero_numeric_ticket_key_when_contract_is_key_required()`
Given: a statically accepted key-required contract and an `ActionTicket { idempotency_key: 0, .. }` fixture.
When: static contract validation runs.
Then: static result equals `Ok(())`; runtime absence is tested only by empty key ingredients.
Layer: unit/integration.

### Behavior: workflow validation does not mutate inputs
Test function: `fn validate_workflow_leaves_parts_and_contracts_equal_to_original_after_validation()`
Given: cloned `WorkflowParts` and cloned `ActionContract` slice.
When: workflow validation runs for both legal and illegal cases.
Then: post-call `parts == original_parts` and `contracts == original_contracts`.
Layer: integration/property.

### Behavior: verifier has no external effects
Test function: `fn verifier_performs_no_external_io_when_validating_typed_contract_values()`
Given: typed `WorkflowParts` and `ActionContract` inputs.
When: validation APIs run under a harness with no filesystem, network, parser, or action-dispatch capabilities.
Then: the only observable output is the returned typed `Result`; no files, sockets, JSON, YAML, HTTP, or dispatched action side effects occur.
Layer: static/adversarial integration.

### Behavior: CLI verify has honest oracle without contracts
Test function: `fn cli_verify_output_does_not_claim_idempotency_contract_proof_when_no_contract_registry_is_supplied()`
Given: a workflow source containing at least one `Do` node and no supplied contract registry.
When: `velvet-ballistics verify <workflow>` runs.
Then: process output contains stable status `idempotency_contract_proof=unavailable` or exits with the typed diagnostic category `ACTION_CONTRACT_MISSING`; it must not contain `idempotency_contract_proof=ok`, `verified_idempotent=true`, or any green proof claim.
Layer: e2e.

### Behavior: IPC certificate has honest oracle with empty registry
Test function: `fn ipc_certificate_marks_idempotency_contract_proof_unavailable_when_registry_is_empty_for_do_workflow()`
Given: certificate generation for a workflow with `Do` nodes and `action_contracts == []`.
When: certificate generation runs.
Then: certificate field `idempotency_contract_proof.status` equals `Unavailable` or `Failed(ActionContractMissing { action_id: A, node_index: N })`; it must not equal `Proven`.
Layer: integration/e2e.

## 4. Proptest Invariants

### Proptest: `validate_action_idempotency_contract`
Invariant: result equals `Ok(())` iff the direct single-contract decision table accepts the contract; otherwise it equals the exact typed violation with original fields.
Strategy: generate bounded valid `ActionContract` with all enum combinations.
Anti-invariant: no side-effecting invalid shape returns `Ok(())`.

### Proptest: `is_statically_idempotent_contract`
Invariant: same finite decision table as `validate_action_idempotency_contract`.
Strategy: generate all idempotency, side-effect, and retry-safety combinations.
Anti-invariant: no pure contract returns any violation.

### Proptest: pure action acceptance
Invariant: any `side_effect == SideEffect::None` returns exactly `Ok(())` from direct and workflow APIs.
Strategy: all `Idempotency` and `RetrySafety` variants with bounded action IDs.
Anti-invariant: pure contracts never produce `SideEffectingRetryUnsafe`, `SideEffectingAtLeastOnceExternal`, or `SideEffectingDeterministicPure`.

### Proptest: side-effecting accepted shape
Invariant: `side_effect != None` accepts iff `idempotency == IdempotentExternal` and `retry_safety in {Safe, KeyRequired}`.
Strategy: full enum cross product.
Anti-invariant: any accepted side-effecting contract outside that shape fails the property.

### Proptest: retry unsafe rejection
Invariant: any `side_effect != None && retry_safety == Unsafe` returns `SideEffectingRetryUnsafe` with original fields under the precedence rule.
Strategy: side-effecting variants and all idempotency variants.
Anti-invariant: retry-unsafe side-effecting contracts never return `Ok(())`.

### Proptest: at-least-once rejection
Invariant: any `side_effect != None && idempotency == AtLeastOnceExternal && retry_safety != Unsafe` returns `SideEffectingAtLeastOnceExternal` with original fields.
Strategy: side-effecting variants and `Safe | KeyRequired`.
Anti-invariant: side-effecting at-least-once contracts never return `Ok(())`.

### Proptest: deterministic-pure rejection
Invariant: any `side_effect != None && idempotency == DeterministicPure && retry_safety != Unsafe` returns `SideEffectingDeterministicPure` with original fields.
Strategy: side-effecting variants and `Safe | KeyRequired`.
Anti-invariant: side-effecting deterministic-pure contracts never return `Ok(())`.

### Proptest: collection order
Invariant: `collect_idempotency_contract_violations` returns violation action IDs in input slice order with legal contracts omitted.
Strategy: bounded `Vec<ActionContract>` length 0..128, unique action IDs.
Anti-invariant: output order must not depend on hash map or address order.

### Proptest: workflow completeness relation
Invariant: workflow validation can return `Ok(())` only when `Do` action ID set equals workflow-specific contract ID set and all relevant contracts are legal.
Strategy: generated bounded `WorkflowParts` with `Do` and non-`Do` nodes plus registries.
Anti-invariant: missing or orphan IDs never return `Ok(())`.

### Proptest: no mutation
Invariant: validating generated bounded inputs leaves all input values equal to pre-call clones.
Strategy: valid structural workflows and registries with mixed legal/illegal contracts.
Anti-invariant: no node, slot, constant, contract, action ID, or registry order changes.

## 5. Fuzz Targets

### Fuzz Target: typed verifier gate over arbitrary bounded IR
Input type: arbitrary bounded Rust struct converted to `WorkflowParts` plus `Vec<ActionContract>`.
Risk: panic, unchecked indexing, runaway allocation, nondeterministic diagnostics, false completeness success.
Corpus seeds: empty workflow; one Do/missing contract; orphan contract; pure unsafe; side-effecting unsafe; at-least-once; deterministic-pure side effect; 128 illegal contracts; duplicate Do nodes for same action; non-Do nodes interleaved around Do nodes.

### Fuzz Target: CLI verifier proof boundary
Input type: workflow source bytes for the existing parser/verification fuzz boundary plus optional future contract-registry fixture bytes outside runtime core.
Risk: malformed source causes proof claim without contracts; diagnostic rendering panic; JSON/YAML/HTTP parser logic leaks into core.
Corpus seeds: valid no-Do workflow; valid Do workflow without registry; malformed action IDs; duplicate actions; large bounded node list; unsupported trigger; future registry mode with side-effecting unsafe action.

## 6. Kani Harnesses

### Kani Harness: finite idempotency decision table
Property: for the bounded enum domain, every combination maps to exactly one of `Ok(())`, `SideEffectingRetryUnsafe`, `SideEffectingAtLeastOnceExternal`, or `SideEffectingDeterministicPure`.
Bound: full enum cross product.
Rationale: formal exhaustive proof prevents uncovered legality combinations.

### Kani Harness: accepted side-effecting iff idempotent external and safe/key-required
Property: `side_effect != None && result == Ok(())` implies `idempotency == IdempotentExternal && retry_safety != Unsafe`.
Bound: full enum cross product.
Rationale: central deploy-safety invariant.

### Kani Harness: pure action always accepted
Property: for all idempotency and retry-safety values, `SideEffect::None` returns exactly `Ok(())`.
Bound: full enum cross product.
Rationale: preserves existing runtime compatibility.

### Kani Harness: accumulation bounded by contract length
Property: for arrays length <= 16, returned violation count <= input contract count and no index outside the array is read.
Bound: 16 contracts.
Rationale: proves bounded traversal and diagnostic growth.

### Kani Harness: completeness has no false success
Property: for bounded Do ID arrays and contract ID arrays length <= 16, workflow validation cannot return `Ok(())` if any Do action ID is absent from contracts or any contract ID is absent from Do actions.
Bound: 16 Do IDs and 16 contract IDs.
Rationale: Gate 12 must block false idempotency certificates.

## 7. Mutation Testing Checkpoints

Threshold: `cargo-mutants` kill rate >= 90% overall for touched crates and 100% for idempotency decision-table branches.

- Mutating pure acceptance to rejection is killed by `validate_action_returns_unit_for_pure_deterministic_safe_contract`, `validate_action_returns_unit_for_pure_at_least_once_unsafe_contract`, and `is_static_returns_unit_for_pure_contract_for_all_retry_and_idempotency_values`.
- Mutating `RetrySafety::Unsafe` rejection to acceptance is killed by direct validate-action, direct is-static, collect, and workflow retry-unsafe tests.
- Mutating `AtLeastOnceExternal` rejection to acceptance is killed by direct validate-action, direct is-static, collect, and workflow at-least-once tests.
- Mutating `DeterministicPure` side-effect rejection to acceptance is killed by direct validate-action, direct is-static, collect, and workflow deterministic-pure tests.
- Mutating `KeyRequired` acceptance to rejection is killed by direct validate-action, direct is-static, and workflow key-required tests.
- Mutating collection to stop after the first violation is killed by `collect_returns_all_boxed_violations_in_input_order_for_multiple_illegal_contracts` and workflow accumulation tests.
- Mutating collection order to sorting/hash iteration is killed by `collect_returns_same_boxed_violations_when_called_twice_with_same_input` and workflow deterministic-order tests.
- Mutating missing-contract detection to success is killed by `validate_workflow_returns_action_contract_missing_when_do_node_has_no_matching_contract`.
- Mutating orphan detection to success is killed by `validate_workflow_returns_action_contract_orphan_when_registry_contract_has_no_do_node`.
- Mutating completeness failure to still emit proof is killed by `validate_workflow_returns_completeness_error_without_claiming_proof_when_gate_12_fails`, CLI no-registry, and IPC empty-registry tests.
- Mutating `Secret | DerivedFromSecret` to only `Secret` is killed by the two secret-taint runtime tests.
- Mutating empty key slots to success is killed by `runtime_returns_missing_key_when_key_required_action_has_empty_key_slots`.
- Mutating CLI/certificate output to claim proof without registry is killed by the CLI and IPC proof-unavailable scenarios.

## 8. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|---|---|---|---|
| direct pure deterministic safe | `None, DeterministicPure, Safe` | `Ok(())` | unit |
| direct pure at-least-once unsafe | `None, AtLeastOnceExternal, Unsafe` | `Ok(())` | unit |
| direct side-effecting idempotent safe | `Writes, IdempotentExternal, Safe` | `Ok(())` | unit |
| direct side-effecting idempotent key required | `Sends, IdempotentExternal, KeyRequired` | `Ok(())` | unit |
| direct side-effecting retry unsafe | `Destroys, IdempotentExternal, Unsafe` | `Err(SideEffectingRetryUnsafe { action, side_effect: Destroys, idempotency: IdempotentExternal, retry_safety: Unsafe })` | unit |
| direct side-effecting at least once | `Creates, AtLeastOnceExternal, Safe` | `Err(SideEffectingAtLeastOnceExternal { action, side_effect: Creates, idempotency: AtLeastOnceExternal, retry_safety: Safe })` | unit |
| direct side-effecting deterministic pure | `Writes, DeterministicPure, Safe` | `Err(SideEffectingDeterministicPure { action, side_effect: Writes, idempotency: DeterministicPure, retry_safety: Safe })` | unit |
| collect empty | `[]` | `Ok(())` | unit |
| collect legal | legal contract slice | `Ok(())` | unit |
| collect single illegal | one illegal contract | `Err(IdempotencyContractErrors(Box::from([exact_violation])))` | unit |
| collect mixed multiple | legal and illegal contracts | `Err(IdempotencyContractErrors(Box::from([illegal_only_in_input_order])))` | unit |
| workflow empty | no Do, no contracts | `Ok(())` | integration |
| workflow pure all combinations | matching pure contract | `Ok(())` | integration |
| workflow missing | Do action without contract | `Err(ActionContractMissing { action_id, node_index })` | integration |
| workflow orphan | contract without Do action | `Err(ActionContractOrphan { action_id })` | integration |
| workflow multiple violations | A unsafe, B at-least-once, C deterministic-pure | `Err(IdempotencyViolations(IdempotencyContractErrors(Box::from([A, B, C]))))` | integration |
| runtime empty key | key-required action, `key_slots == []` | `Err(IdempotencyViolation::MissingKey(side_effect))` | unit |
| runtime clean key | key-required action, non-empty clean slots | `Ok(())` | unit |
| runtime secret key | `Taint::Secret` | `Err(IdempotencyViolation::SecretInKey(slot))` | unit |
| runtime derived secret key | `Taint::DerivedFromSecret` | `Err(IdempotencyViolation::SecretInKey(slot))` | unit |
| zero ticket key | `ActionTicket.idempotency_key == 0` | static verifier `Ok(())` | unit/integration |
| CLI no registry | Do workflow, no contract source | proof status `unavailable` or `ACTION_CONTRACT_MISSING`, never proof success | e2e |
| IPC empty registry | Do workflow, empty contracts | `Unavailable` or `Failed(ActionContractMissing)`, never `Proven` | integration/e2e |

## 9. Static Resource, Panic, and Command Gates

Required gates after tests are implemented:

1. `moon ci` passes.
2. `moon run :nightly-feature-gate` or `just nightly-feature-gate` passes.
3. Static source checks confirm production code contains no `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing, unchecked slicing, unchecked casts, or unchecked arithmetic.
4. `cargo-mutants` scoped to touched crates reports >= 90% overall kill and 100% decision-table branch kill.
5. `cargo fuzz run verifier_gates` with the listed corpus seeds has no crash, no OOM, and no false proof claim.
6. `cargo kani` proves all listed harnesses.
7. Resource check: tests must include a bounded large case with up to 128 contracts and assert returned violation count <= contract count.
8. Panic check: fuzz/static gates must treat any panic in verifier APIs as a failure.

## Open Questions

1. If implementation chooses a different precedence for contracts that are both `RetrySafety::Unsafe` and invalid idempotency, the code must document it before tests are written; tests must then assert that exact precedence. This plan's concrete single-violation examples avoid ambiguous overlaps.
2. `RandomInKey` and `TimeInKey` remain runtime contract gaps until provenance metadata exists; future tests must assert exact variants when that metadata is introduced.
3. CLI registry input is not specified yet; current acceptance oracle is honest unavailable/failure status, not successful proof.
