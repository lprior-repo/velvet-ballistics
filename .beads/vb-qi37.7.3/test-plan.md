# Test Plan: vb-qi37.7.3 — IR reference validation

Contract review prerequisite: `.beads/vb-qi37.7.3/contract-verification-review.md` is `STATUS: APPROVED`.

Scope: numeric compiled IR admission only. No production code, test code, proof code, bead status update, commit, push, or implementation is part of this artifact.

## Summary

- Behaviors identified: 27
- Testing Trophy allocation: 45 unit/component, 26 integration, 2 manual/E2E, 6 property groups, 0 mandatory parser fuzz targets under waiver `W-FUZZ-001`, 5 Kani harness groups, 9 Lean proof obligations, 8 static/manual gates.
- Mandatory mutation threshold: `>= 90%` killed overall for touched validation/admission modules and `100%` killed for critical mutants in Section 7.
- Canonical acceptance command after implementation: `moon ci`, plus verification lanes mapped below.
- Assertion rule: no planned assertion may be only `is_ok()` or `is_err()`; every failure asserts exact typed variant and salient fields.

## 1. Behavior Inventory

1. Core admission validates untrusted `WorkflowParts` before returning `CompiledWorkflow` when callers submit numeric IR.
2. Validators do not mutate borrowed `WorkflowParts`, action contracts, registries, globals, or external state when validation succeeds or fails.
3. `CompiledWorkflow::try_from_parts` returns `Result` and never panics when invalid references or resources are submitted.
4. `validate(parts)` runs non-action verifier gates and does not claim action-contract completeness when `Do` nodes exist.
5. `validate_with_contracts(parts, contracts)` requires action completeness when `Do` nodes exist.
6. Validators treat numeric IDs and resource fields as untrusted when symbols, slots, constants, handlers, actions, and resources are generated across boundaries.
7. Runtime core validation performs no YAML, JSON, HTTP, filesystem, network, plugin, or dynamic schema lookup when checking IR.
8. Core admission accepts symbol carriers when every `SymbolId` in accessor fields, symbol constants, and build-object field keys is `< symbols_count`.
9. Core admission rejects accessor field symbols when `symbol.get() >= symbols_count`.
10. Core admission rejects `ConstValue::Symbol` when `symbol.get() >= symbols_count`.
11. Core admission rejects build-object field keys when `symbol.get() >= symbols_count`.
12. Core and verifier reject any symbol carrier when `symbols_count == 0` and `SymbolId::new(0)` appears.
13. Core and verifier accept slot references only when every slot reference is `< slot_count` and artifact-owned.
14. Core and verifier reject slot references when a slot is out of range, wrong-kind for the use site, or cross-artifact.
15. Core and verifier accept constant references only when every constant index is in range, kind-correct, and artifact-owned; symbol-valued constants also satisfy symbol bounds.
16. Core and verifier reject constant references when a constant index is out of range, wrong-kind, or cross-artifact.
17. Core and verifier accept handler references only when every handler reference is in range, kind-correct, and artifact-owned.
18. Core and verifier reject handler references when a handler ID is out of range, wrong-kind, or cross-artifact.
19. `validate_with_contracts` succeeds when unique `Do.action` IDs equal unique supplied `ActionContract.id` values.
20. `validate_with_contracts` rejects missing action contracts with exact action ID and node index.
21. `validate_with_contracts` rejects orphan action contracts with exact action ID.
22. Resource validation rejects declared `ResourceContract` members above protocol hard limits with `WorkflowError::ResourceContractTooLarge { resource }`; verifier parity requires the explicit amendment named in Open Questions if still in scope.
23. Resource validation rejects actual IR usage above declared `ResourceContract` limits with `WorkflowError::ResourceContractExceeded { resource }`; verifier parity requires the explicit amendment named in Open Questions if still in scope.
24. Diagnostics for every new validation variant expose stable codes/rendering and exact enum assertions downstream.
25. `validate_symbol_references(parts)` succeeds only when every symbol carrier is below `symbols_count` and otherwise returns `WorkflowError::SymbolOutOfBounds { symbol }`.
26. `validate_resource_references(parts)` succeeds only when declared resources are within hard limits and actual usage is within declaration, otherwise returning `WorkflowError::ResourceContractTooLarge { resource }` or `WorkflowError::ResourceContractExceeded { resource }`.
27. `validate_action_references(parts, contracts)` succeeds only when unique `Do.action` IDs equal unique supplied `ActionContract.id` values, otherwise returning `ValidationError::ActionContractMissing { action_id, node_index }` or `ValidationError::ActionContractOrphan { action_id }`.

## 2. Trophy Allocation

| Behavior(s) | Layer(s) | Tool/command | Rationale |
|---|---|---|---|
| 1, 3, 8-18, 22-23, 25-26 | Unit/component | `cargo test -p vb_core vb_qi37_7_3` through `moon run :verify-fast` | Pure admission/reference predicates and direct core helpers belong in fast deterministic tests with exact values. |
| 4-5, 19-21, 24, 27 | Integration | `cargo test -p vb_validate vb_qi37_7_3` through `moon run :verify-fast` | Verifier pipeline behavior crosses gates, action contracts, diagnostics, and public error mapping; direct action-helper coverage prevents public API gaps. |
| 2 | Miri/static/component | `moon run :verify-deep` | No-mutation/no-global-state is Rust shell evidence, not Lean. |
| 6, 8-23 | Property | proptest under `moon run :verify-deep` | Generated numeric IDs/resources expose boundary and cross-product defects better than examples. |
| 7 | Static/manual | `moon run :verify-standard` plus scans in Section 9 | Runtime-core no-I/O/no-config lookup is source-governance evidence. |
| 8-23 | Kani | `moon run :verify-proof` | Bounded exhaustive checks prove off-by-one, index, and state-machine completeness. |
| INV-001..INV-008, POST-007 | Lean | `moon run :verify-proof` | Pure deterministic invariants are Lean-owned per approved contract. |
| Parser/codec fuzzing | Waived unless codec touched | `W-FUZZ-001`; void if decode/parser changes | This bead validates already-constructed in-memory `WorkflowParts`; no parser/codec boundary is in scope. |
| CLI/manual smoke | Manual/E2E | two fixture-based smoke checks | Few black-box checks prove user-visible diagnostics without dominating the trophy. |

Target ratio after implementation: integration/property-heavy around verifier boundary, with unit/component examples covering every error branch. Any deviation must be recorded in implementation evidence.

## 3. BDD Scenarios

### Behavior: untrusted parts are checked before core admission

- Test: `fn try_from_parts_rejects_untrusted_parts_before_workflow_is_returned()`
- Given: owned `WorkflowParts` containing one invalid accessor field symbol equal to `symbols_count`.
- When: `CompiledWorkflow::try_from_parts(parts)` is called.
- Then: result is `Err(WorkflowError::SymbolOutOfBounds { symbol })` with `symbol.get() == symbols_count`; no `CompiledWorkflow` value is observable.

### Behavior: validators do not mutate borrowed inputs

- Test: `fn validate_with_contracts_leaves_parts_and_contracts_unchanged_when_action_missing()`
- Given: cloned `WorkflowParts` with `Do(ActionId(7))` and empty `ActionContract` slice snapshot.
- When: `validate_with_contracts(&parts, &contracts)` is called.
- Then: result is `Err(ValidationError::ActionContractMissing { action_id: 7, node_index: 0 })`; `parts == original_parts`; `contracts == original_contracts`.

### Behavior: invalid IR returns `Result::Err` without panic

- Test: `fn try_from_parts_returns_result_error_when_slot_reference_is_out_of_range()`
- Given: owned `WorkflowParts` with `slot_count = 1` and one node/expression using slot index `1`.
- When: `CompiledWorkflow::try_from_parts(parts)` is called under normal test and Miri/cargo-careful lanes.
- Then: result is `Err(WorkflowError::SlotOutOfBounds { slot: SlotIdx(1) })`; no `CompiledWorkflow` value is observable and no panic output occurs.

### Behavior: default verifier skips action-contract completeness

- Test: `fn validate_returns_success_for_do_action_without_contract_when_non_action_gates_pass()`
- Given: `WorkflowParts` with valid symbols, slots, constants, handlers, resources, and a `Do(ActionId(7))`; no contracts are supplied because `validate` has no contract argument.
- When: `vb_validate::shared::validate(&parts)` is called.
- Then: result is exactly `Ok(())`, proving no action-complete claim is made by this API.

### Behavior: action-complete verifier requires supplied contracts

- Test: `fn validate_with_contracts_returns_missing_contract_when_do_action_has_no_contract()`
- Given: valid non-action `WorkflowParts` with `Do(ActionId(7))` at node index `0` and `contracts = []`.
- When: `vb_validate::shared::validate_with_contracts(&parts, &contracts)` is called.
- Then: result is `Err(ValidationError::ActionContractMissing { action_id: 7, node_index: 0 })`.

### Behavior: numeric IDs/resources are untrusted

- Test: `fn generated_numeric_ids_are_rejected_at_owner_bounds()`
- Given: generated parts with exactly one invalid ID among symbol, slot, constant, handler, action, or resource cases and all earlier gates made valid.
- When: the matching public validator is called.
- Then: result is the exact typed error named in this plan for the generated offending ID/resource and location.

### Behavior: runtime core validation performs no external lookup

- Test/gate: `given_runtime_core_validation_when_scanned_then_no_json_yaml_http_or_io_lookup_exists`
- Given: touched runtime-core validation files under `crates/vb_core/src/**` and `crates/vb_validate/src/**` excluding tests/support.
- When: static scan from Section 9 is executed.
- Then: no touched validation/admission source contains YAML/JSON/HTTP/filesystem/network/plugin lookup patterns.

### Behavior: in-bounds symbols are accepted

- Test: `fn symbol_references_are_accepted_when_every_carrier_is_below_symbols_count()`
- Given: `symbols_count = 3`, accessor field `SymbolId(0)`, symbol constant `SymbolId(1)`, and build-object field `SymbolId(2)`.
- When: `CompiledWorkflow::try_from_parts(parts)` and verifier validation are called on equivalent valid fixtures.
- Then: core returns `Ok(workflow)` with public counts matching submitted parts, and verifier returns `Ok(())`.

### Behavior: direct symbol-reference helper accepts all in-bounds carriers

- Test: `fn validate_symbol_references_returns_unit_when_all_symbol_carriers_are_in_bounds()`
- Given: `WorkflowParts` with `symbols_count = 3`, accessor field `SymbolId(0)`, symbol constant `SymbolId(1)`, build-object field `SymbolId(2)`, and otherwise valid slots/constants/nodes/resources.
- When: `vb_core::workflow::validate_symbol_references(&parts)` is called directly.
- Then: result is exactly `Ok(())`; `parts` remains byte-for-byte/value equal to the pre-call snapshot.

### Behavior: direct symbol-reference helper rejects an out-of-bounds carrier

- Test: `fn validate_symbol_references_returns_symbol_out_of_bounds_when_accessor_field_equals_symbols_count()`
- Given: `WorkflowParts` with `symbols_count = 1` and accessor index `0` containing `PathSegment::Field(SymbolId(1))`; all non-symbol validation inputs are otherwise valid.
- When: `vb_core::workflow::validate_symbol_references(&parts)` is called directly.
- Then: result is `Err(WorkflowError::SymbolOutOfBounds { symbol: SymbolId(1) })`; no generic validation error, string-only error, or later-gate resource/action error is accepted.

### Behavior: accessor field symbol out of bounds is rejected

- Test: `fn accessor_field_symbol_equal_to_symbols_count_returns_symbol_out_of_bounds()`
- Given: `symbols_count = 1` and accessor index `0` contains `PathSegment::Field(SymbolId(1))`.
- When: core admission and verifier validation run.
- Then: core/helper returns `Err(WorkflowError::SymbolOutOfBounds { symbol: SymbolId(1) })`; verifier-side coverage is blocked until the contract is amended to add `ValidationError::SymbolReferenceOutOfRange { symbol: usize, symbols_count: usize, context: String }` with stable code `CODE_SYMBOL_REFERENCE_OUT_OF_RANGE = 0x050D`, where `symbol == 1`, `symbols_count == 1`, and `context == "accessor 0 field 0"` for this fixture.

### Behavior: symbol constant out of bounds is rejected

- Test: `fn symbol_constant_equal_to_symbols_count_returns_symbol_out_of_bounds()`
- Given: `symbols_count = 2` and constant index `0` is `ConstValue::Symbol(SymbolId(2))`.
- When: core admission and verifier validation run.
- Then: core/helper returns `Err(WorkflowError::SymbolOutOfBounds { symbol: SymbolId(2) })`; verifier-side coverage requires `ValidationError::SymbolReferenceOutOfRange { symbol: 2, symbols_count: 2, context: "constant 0" }` after the `0x050D` contract amendment.

### Behavior: build-object field symbol out of bounds is rejected

- Test: `fn build_object_field_symbol_equal_to_symbols_count_returns_symbol_out_of_bounds()`
- Given: `symbols_count = 2` and build-object node index `0` has field key `SymbolId(2)`.
- When: core admission and verifier validation run.
- Then: core/helper returns `Err(WorkflowError::SymbolOutOfBounds { symbol: SymbolId(2) })`; verifier-side coverage requires `ValidationError::SymbolReferenceOutOfRange { symbol: 2, symbols_count: 2, context: "build_object node 0 field 0" }` after the `0x050D` contract amendment.

### Behavior: zero symbols rejects any symbol carrier

- Tests:
  - `fn zero_symbols_rejects_accessor_symbol_zero()`
  - `fn zero_symbols_rejects_constant_symbol_zero()`
  - `fn zero_symbols_rejects_build_object_symbol_zero()`
- Given: `symbols_count = 0` and one carrier contains `SymbolId(0)`.
- When: core admission/helper and verifier validation run.
- Then: core/helper returns `Err(WorkflowError::SymbolOutOfBounds { symbol: SymbolId(0) })`; verifier-side coverage requires `ValidationError::SymbolReferenceOutOfRange { symbol: 0, symbols_count: 0, context: "accessor 0 field 0" }`, `context: "constant 0"`, or `context: "build_object node 0 field 0"` matching the carrier test after the `0x050D` contract amendment.

### Behavior: slot references are owned and in range

- Test: `fn slot_reference_equal_to_slot_count_returns_precise_slot_error()`
- Given: `slot_count = 1` and one node/expression/action input uses slot `1`.
- When: core admission and verifier validation run.
- Then: core returns `Err(WorkflowError::SlotOutOfBounds { slot: SlotIdx(1) })`; verifier returns `Err(ValidationError::SlotReferenceOutOfRange { slot: 1, slot_count: 1, context: "node 0" })` for a node slot use or `context: "expression 0"` for an expression `LoadSlot` use.
- Boundary: `fn slot_reference_at_last_valid_slot_is_accepted()` uses slot `0` with `slot_count = 1` and expects exact success value.

### Behavior: slot wrong-kind/cross-artifact is rejected

- Test: `fn slot_reference_wrong_kind_returns_precise_slot_kind_error()`
- Given: a slot reference used in a position that requires a different slot/reference kind, or a constructed cross-artifact slot fixture if the model exposes ownership tags.
- When: verifier/core validation run.
- Then: because the current implementation exposes only bounds/cycle/type variants, this requires a contract amendment before implementation: add `ValidationError::SlotReferenceWrongKind { slot: usize, required: String, actual: String, context: String }` with stable code `CODE_SLOT_REFERENCE_WRONG_KIND = 0x050E`; assert all four fields exactly. If cross-artifact slot ownership is not representable in `WorkflowParts`, mark that subcase waived by contract amendment rather than using a generic error.

### Behavior: constant references are bounded, kind-correct, and artifact-owned

- Test: `fn constant_reference_equal_to_constants_len_returns_precise_constant_error()`
- Given: constants length `1` and a constant reference index `1`.
- When: core admission and verifier validation run.
- Then: core returns `Err(WorkflowError::ConstOutOfBounds { constant: ConstIdx(1) })`; verifier parity requires a contract amendment adding `ValidationError::ConstantReferenceOutOfRange { constant: usize, constant_count: usize, context: String }` with stable code `CODE_CONSTANT_REFERENCE_OUT_OF_RANGE = 0x050F`, asserted as `constant == 1`, `constant_count == 1`, and exact use-site context.
- Boundary: `fn constant_reference_at_last_valid_index_is_accepted()` expects success for index `0` with one constant.

### Behavior: constant wrong-kind is rejected

- Test: `fn constant_reference_wrong_kind_returns_precise_constant_kind_error()`
- Given: a use site requiring kind `K` and constant index `0` containing different kind `J`.
- When: validation runs.
- Then: this requires a contract amendment before implementation: add `ValidationError::ConstantReferenceWrongKind { constant: usize, required: String, actual: String, context: String }` with stable code `CODE_CONSTANT_REFERENCE_WRONG_KIND = 0x0510`; assert `constant == 0`, required kind `K`, actual kind `J`, and exact use-site context.

### Behavior: handler references are bounded, kind-correct, and artifact-owned

- Test: `fn handler_reference_equal_to_handler_count_returns_precise_handler_error()`
- Given: handler count/table length `1` and handler reference `1`.
- When: validation runs.
- Then: for `CompiledNodeKind::ErrorHandler { handler: StepIdx(1), .. }` with `node_count = 1`, core returns `Err(WorkflowError::StepOutOfBounds { step: StepIdx(1) })` and verifier returns `Err(ValidationError::LoopBodyStepOutOfRange { step: 1, node_count: 1, source_node: 0, label: "error_handler handler" })`.
- Boundary: `fn handler_reference_at_last_valid_index_is_accepted()` expects success for handler `0` with one handler.

### Behavior: handler wrong-kind/cross-artifact is rejected

- Test: `fn handler_reference_wrong_kind_returns_precise_handler_kind_error()`
- Given: handler reference points to a declared handler with the wrong kind for handler use, or to a cross-artifact handler if ownership tags exist.
- When: validation runs.
- Then: because current `WorkflowParts` models handlers as step references and has no handler kind/owner tag, this requires a contract amendment before implementation if kind/ownership remains in scope: add `ValidationError::HandlerReferenceWrongKind { handler: usize, required: String, actual: String, context: String }` with stable code `CODE_HANDLER_REFERENCE_WRONG_KIND = 0x0511`; otherwise amend ERR-008/INV-006 to reduce handler validation to step-range/structural handler reachability and assert the exact `StepOutOfBounds` / `LoopBodyStepOutOfRange` variants above.

### Behavior: matching action contracts satisfy action-complete validation

- Test: `fn validate_with_contracts_returns_unit_when_unique_do_actions_equal_unique_contract_ids()`
- Given: `Do(ActionId(7))`, `Do(ActionId(7))`, `Do(ActionId(9))`, and contracts with IDs `{7, 9}`.
- When: `validate_with_contracts(&parts, &contracts)` runs.
- Then: result is exactly `Ok(())`.

### Behavior: direct action-reference helper accepts exact action-contract set equality

- Test: `fn validate_action_references_returns_unit_when_unique_do_actions_equal_unique_contract_ids()`
- Given: `WorkflowParts` with `Do(ActionId(7))`, duplicate `Do(ActionId(7))`, and `Do(ActionId(9))`; supplied `ActionContract` slice contains exactly IDs `{7, 9}` in any deterministic order.
- When: `vb_validate::shared::validate_action_references(&parts, &contracts)` is called directly.
- Then: result is exactly `Ok(())`; duplicate `Do` IDs require only one matching contract and do not require duplicate contracts.

### Behavior: direct action-reference helper rejects a missing contract with node index

- Test: `fn validate_action_references_returns_missing_contract_when_do_action_has_no_contract()`
- Given: `WorkflowParts` with `Do(ActionId(7))` at node index `2` and `contracts = []`.
- When: `vb_validate::shared::validate_action_references(&parts, &contracts)` is called directly.
- Then: result is `Err(ValidationError::ActionContractMissing { action_id: 7, node_index: 2 })`; no orphan or generic gate error is accepted.

### Behavior: direct action-reference helper rejects an orphan contract with action ID

- Test: `fn validate_action_references_returns_orphan_contract_when_contract_id_is_unreferenced()`
- Given: `WorkflowParts` with no `Do` nodes and supplied `ActionContract` slice `[ActionContract { id: ActionId(9), .. }]`.
- When: `vb_validate::shared::validate_action_references(&parts, &contracts)` is called directly.
- Then: result is `Err(ValidationError::ActionContractOrphan { action_id: 9 })`; no missing-contract or generic gate error is accepted.

### Behavior: missing action contract is rejected with node index

- Test: `fn missing_action_contract_reports_first_missing_action_in_node_index_order()`
- Given: `Do(ActionId(7))` at node index `2`, `Do(ActionId(9))` at node index `4`, and no contracts.
- When: action-complete validation runs.
- Then: result is `Err(ValidationError::ActionContractMissing { action_id: 7, node_index: 2 })`.

### Behavior: orphan action contract is rejected with action ID

- Test: `fn orphan_action_contract_reports_first_orphan_in_supplied_contract_order()`
- Given: no `Do` nodes and supplied contracts `[ActionId(9), ActionId(11)]`.
- When: action-complete validation runs.
- Then: result is `Err(ValidationError::ActionContractOrphan { action_id: 9 })`.

### Behavior: declared resource above hard limit is rejected

- Tests, one per member:
  - `fn declared_max_steps_above_hard_limit_returns_resource_contract_too_large()`
  - `fn declared_max_slots_above_hard_limit_returns_resource_contract_too_large()`
  - `fn declared_max_constants_above_hard_limit_returns_resource_contract_too_large()`
  - `fn declared_max_accessors_above_hard_limit_returns_resource_contract_too_large()`
  - `fn declared_max_expressions_above_hard_limit_returns_resource_contract_too_large()`
  - `fn declared_max_expr_stack_above_hard_limit_returns_resource_contract_too_large()`
- Given: exactly one declared resource equals hard limit + 1 and all prior fields are valid.
- When: core/resource verifier validation runs.
- Then: core/direct helper error is `WorkflowError::ResourceContractTooLarge { resource }` where `resource` is exactly one of `"max_steps"`, `"max_slots"`, `"max_constants"`, `"max_accessors"`, `"max_expressions"`, or `"max_expr_stack"` matching the mutated member; verifier resource parity currently lacks a dedicated variant and therefore requires a contract amendment adding `ValidationError::ResourceContractTooLarge { resource: String }` with stable code `CODE_RESOURCE_CONTRACT_TOO_LARGE = 0x0512` if verifier-side resource parity remains required.
- Boundary: equality at hard limit is accepted when actual usage is within declaration.

### Behavior: direct resource-reference helper accepts declared and actual usage within limits

- Test: `fn validate_resource_references_returns_unit_when_declared_and_actual_resources_are_within_limits()`
- Given: `WorkflowParts` whose `ResourceContract` members are each at or below protocol hard limits and whose actual nodes, slots, constants, accessors, expressions, handlers, and expression stack usage are each `<=` the corresponding declared member.
- When: `vb_core::workflow::validate_resource_references(&parts)` is called directly.
- Then: result is exactly `Ok(())`; `parts` remains equal to its pre-call snapshot.

### Behavior: direct resource-reference helper rejects declared resource above hard limit

- Test: `fn validate_resource_references_returns_resource_contract_too_large_when_declared_max_steps_exceeds_hard_limit()`
- Given: `WorkflowParts` where exactly `resource_contract.max_steps` is `MAX_WORKFLOW_NODES + 1` and all actual usage is otherwise within declaration.
- When: `vb_core::workflow::validate_resource_references(&parts)` is called directly.
- Then: result is `Err(WorkflowError::ResourceContractTooLarge { resource: "max_steps" })`; no exceeded, symbol, slot, action, or generic error is accepted.

### Behavior: actual usage above declared resource is rejected

- Tests, one per member:
  - `fn node_count_above_max_steps_returns_resource_contract_exceeded()`
  - `fn slot_count_above_max_slots_returns_resource_contract_exceeded()`
  - `fn constants_len_above_max_constants_returns_resource_contract_exceeded()`
  - `fn accessors_len_above_max_accessors_returns_resource_contract_exceeded()`
  - `fn expressions_len_above_max_expressions_returns_resource_contract_exceeded()`
  - `fn expression_stack_above_max_expr_stack_returns_resource_contract_exceeded()`
- Given: exactly one actual usage is declaration + 1 and declared values are within hard limits.
- When: core/resource verifier validation runs.
- Then: core/direct helper error is `WorkflowError::ResourceContractExceeded { resource }` where `resource` is exactly one of `"max_steps"`, `"max_slots"`, `"max_constants"`, `"max_accessors"`, `"max_expressions"`, or `"max_expr_stack"` matching the mutated member; verifier resource parity currently lacks a dedicated variant and therefore requires a contract amendment adding `ValidationError::ResourceContractExceeded { resource: String }` with stable code `CODE_RESOURCE_CONTRACT_EXCEEDED = 0x0513` if verifier-side resource parity remains required.
- Boundary: actual usage equal to declaration is accepted.

### Behavior: direct resource-reference helper rejects actual usage above declared limit

- Test: `fn validate_resource_references_returns_resource_contract_exceeded_when_node_count_exceeds_max_steps()`
- Given: `WorkflowParts` whose hard-limit-valid `resource_contract.max_steps` is `1` and whose actual node count is `2`; all other resource members and references are valid.
- When: `vb_core::workflow::validate_resource_references(&parts)` is called directly.
- Then: result is `Err(WorkflowError::ResourceContractExceeded { resource: "max_steps" })`; no too-large, symbol, slot, action, or generic error is accepted.

### Behavior: diagnostic rendering is stable and exact

- Tests:
  - `fn diagnostic_for_symbol_reference_error_has_stable_code_and_location()`
  - `fn diagnostic_for_slot_reference_error_has_stable_code_and_location()`
  - `fn diagnostic_for_constant_reference_error_has_stable_code_and_location()`
  - `fn diagnostic_for_handler_reference_error_has_stable_code_and_location()`
  - `fn diagnostic_for_resource_too_large_has_stable_code_and_values()`
  - `fn diagnostic_for_resource_exceeded_has_stable_code_and_values()`
  - `fn diagnostic_for_action_contract_missing_has_stable_code_action_and_node()`
  - `fn diagnostic_for_action_contract_orphan_has_stable_code_and_action()`
- Given: one instance of each typed validation error.
- When: diagnostic rendering/code conversion runs.
- Then: stable code is exact and output includes salient fields; no generic `UNKNOWN`, string-only fallback, or internal error text appears.

### Behavior: validation is deterministic and bounded

- Test: `fn validation_returns_same_exact_result_for_repeated_runs_on_same_invalid_ir()`
- Given: invalid fixtures for symbol, slot, constant, handler, resource, and action.
- When: each public validator is run twice on the same inputs.
- Then: exact `Result` values compare equal and first-failure ordering is stable.

## 4. Proptest Invariants

### Proptest: symbol carrier bounds

- Invariant: validation succeeds iff every generated symbol carrier has `symbol < symbols_count`.
- Strategy: generate `symbols_count in 0..=32`, carrier kind `{accessor_field, const_symbol, build_object_field}`, valid unrelated structure, and exactly one target symbol.
- Anti-invariant: `symbols_count == 0` with any symbol carrier always yields exact symbol out-of-bounds error.

### Proptest: slot/constant/handler owner bounds and kind correctness

- Invariant: every generated slot/constant/handler reference is accepted iff it is in range, kind-correct, and belongs to the same generated artifact model.
- Strategy: bounded tables length `0..=16`, reference index `0..=17`, use-site kind enum, owner token enum `{same_artifact, other_artifact}` where representable.
- Anti-invariant: one out-of-range, wrong-kind, or cross-artifact target always yields the precise corresponding error variant.

### Proptest: action-contract bijection

- Invariant: action-complete validation returns `Ok(())` iff unique `Do.action` IDs equal unique supplied `ActionContract.id` values.
- Strategy: `0..=8` Do nodes, action IDs `0..=32`, `0..=8` contracts, generated duplicates included.
- Anti-invariant: remove one referenced contract to force missing; add one unreferenced contract to force orphan.

### Proptest: resource hard limits and coverage

- Invariant: validation succeeds iff `actual <= declared <= hard_limit` for each resource member.
- Strategy: six resource members generated in `0..=hard_limit+1` with one target violation at a time.
- Anti-invariant: `declared = hard_limit + 1` always yields too-large; `actual = declared + 1` always yields exceeded.

### Proptest: core/verifier parity

- Invariant: for equivalent invalid IR, core and verifier identify the same class of failure and same offending value/resource, even if enum types differ.
- Strategy: generate a single violation for symbol, slot, constant, handler, or resource while all earlier gates pass.
- Anti-invariant: changing the offending value to its upper valid boundary makes both validation surfaces pass that class of check.

### Proptest: determinism/no mutation

- Invariant: public validators are pure deterministic scans over input values; repeated calls return identical results and snapshots remain equal.
- Strategy: generated valid/invalid parts and action contract slices.
- Anti-invariant: any mutation, global counter effect, ordering nondeterminism, or panic fails the property.

## 5. Fuzz Targets

### Waiver: in-memory validator has no parser/codec boundary

- Waiver: `W-FUZZ-001` from `verification-layers.md` applies only while implementation does not add/modify serialized IR parsing, deserialization, CLI decoding, or artifact codec code.
- Required test-writer action: add a review/check test named `fn fuzz_waiver_remains_valid_when_no_parser_or_codec_boundary_is_touched()` that inspects implementation evidence/diff scope, not runtime behavior.
- Compensating evidence: proptest groups above plus Kani bounds checks below.

### Conditional fuzz target: serialized artifact decode admission

- Trigger: implementation touches `.vbir`, postcard, bincode, JSON/YAML, CLI fixture decoding, or any bytes/string-to-`WorkflowParts` path.
- Input type: arbitrary bytes for the touched decoder.
- Risk: panic, OOM, invalid artifact admission, unchecked index/cast, generic string-only errors.
- Corpus seeds: minimal valid artifact; invalid accessor symbol; invalid symbol constant; invalid build-object field; zero symbols with symbol zero; bad slot; bad constant; bad handler; declared resource hard limit + 1; actual resource over declared.
- Oracle: decoder returns typed decode error, or decoded parts admitted/rejected by exact validation errors; no panic/abort/OOM.

## 6. Kani Harnesses

### Kani: symbol off-by-one completeness

- Property: for each symbol carrier and `symbols_count <= 8`, validation accepts exactly symbols `< symbols_count` and rejects symbols `>= symbols_count`.
- Bound: carriers 3; counts `0..=8`; symbol IDs `0..=9`.
- Rationale: proves zero-count and equality-boundary behavior beyond sampled unit cases.

### Kani: reference index bounds

- Property: validation never indexes out of bounds and rejects invalid slot/constant/handler indices for vectors/tables length `0..=8`.
- Bound: slots/constants/handlers/nodes/expressions/accessors length `0..=8`.
- Rationale: repository forbids unchecked indexing; Kani proves harnessed traversal safety.

### Kani: action-contract set equality

- Property: for up to 4 `Do` nodes and 4 contracts, action-complete validation succeeds iff unique ID sets are equal; missing is reported before orphan.
- Bound: action IDs `0..=7`.
- Rationale: catches duplicate/set and precedence mistakes.

### Kani: resource comparison boundaries

- Property: equality at hard limit and declared limit is accepted; `declared > hard_limit` and `actual > declared` are rejected with the correct resource member.
- Bound: six members, counts `0..=8` with harness constants.
- Rationale: proves comparison operators are not off by one.

### Kani: no partial acceptance state machine

- Property: admission outcome is either full success with all predicates true or typed failure; no state admits a `CompiledWorkflow` after any predicate failure.
- Bound: reduced `WorkflowPartsModel` with booleans for each predicate class and bounded IDs/resources.
- Rationale: proves postcondition POST-009 as a bounded admission lattice.

## 7. Mutation Testing Checkpoints

Threshold: `cargo-mutants` must report `>= 90%` killed for touched `vb_core`/`vb_validate` validation modules. Critical mutants below must be killed; any survivor blocks acceptance regardless of aggregate rate.

- Remove call to reference validation before core admission -> killed by `try_from_parts_rejects_untrusted_parts_before_workflow_is_returned`.
- Replace `>= symbols_count` with `> symbols_count` -> killed by three `*_equal_to_symbols_count_*` symbol tests.
- Skip accessor symbol traversal -> killed by accessor carrier test.
- Skip constant symbol traversal -> killed by constant carrier test.
- Skip build-object field traversal -> killed by build-object carrier test.
- Remove/export-stub `validate_symbol_references` or make it always return success -> killed by direct `validate_symbol_references_returns_symbol_out_of_bounds_when_accessor_field_equals_symbols_count`.
- Allow `symbols_count == 0` with symbol zero -> killed by three zero-symbol tests.
- Remove slot validation or use wrong count -> killed by slot out-of-range and property tests.
- Remove constant bounds/kind check -> killed by constant out-of-range/wrong-kind tests.
- Remove handler bounds/kind check -> killed by handler out-of-range/wrong-kind tests.
- Replace action set equality with subset check -> killed by orphan action test.
- Replace action set equality with superset check -> killed by missing action test.
- Report orphan before missing -> killed by missing-before-orphan action scenario.
- Ignore duplicate `Do.action` handling -> killed by duplicate Do matching-contract scenario.
- Add Gate 12 to `validate` -> killed by default validate skip scenario.
- Remove Gate 12 from `validate_with_contracts` -> killed by missing contract scenario.
- Remove/export-stub `validate_action_references` or make it ignore supplied contracts -> killed by direct missing/orphan action-reference helper scenarios.
- Replace `declared > hard_limit` with `declared >= hard_limit` -> killed by equality-at-hard-limit resource tests.
- Replace `actual > declared` with `actual >= declared` -> killed by equality-at-declared-limit resource tests.
- Remove/export-stub `validate_resource_references` or make it skip resource contract checks -> killed by direct too-large/exceeded resource-reference helper scenarios.
- Swap resource names/fields -> killed by exact resource-member assertions.
- Collapse typed errors to generic strings -> killed by exact enum and diagnostic tests.
- Omit stable diagnostic code for new variant -> killed by diagnostic rendering tests.
- Mutate borrowed inputs or use global state -> killed by no-mutation snapshot/Miri tests.
- Introduce panic/unwrap/expect path -> killed by static scan plus invalid IR no-panic tests.

## 8. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|---|---|---|---|
| valid full IR | all references in range, resources covered | `Ok(CompiledWorkflow)` and `Ok(())` verifier with exact public counts | integration |
| direct symbol helper valid | all symbol carriers below `symbols_count` | `Ok(())` from `validate_symbol_references` | unit |
| direct symbol helper invalid | accessor field `SymbolId(symbols_count)` | `WorkflowError::SymbolOutOfBounds { symbol }` | unit/mutation |
| accessor symbol valid upper boundary | `symbol = symbols_count - 1` | success | unit/property |
| accessor symbol invalid equality | `symbol = symbols_count` | exact symbol error, accessor location | unit/property/Kani |
| constant symbol invalid equality | `symbol = symbols_count` | exact symbol error, constant location | unit/property/Kani |
| build-object symbol invalid equality | `symbol = symbols_count` | exact symbol error, build-object location | unit/property/Kani |
| zero symbols + accessor | `symbols_count = 0`, symbol `0` | exact symbol error | unit/property/Kani |
| slot valid upper boundary | slot `slot_count - 1` | success | unit/property |
| slot invalid equality | slot `slot_count` | precise slot error | unit/property/Kani |
| slot wrong kind | kind mismatch | precise slot kind error | unit/property |
| constant invalid equality | index `constants.len()` | precise constant error | unit/property/Kani |
| constant wrong kind | required/actual kind differ | precise constant kind error | unit/property |
| handler invalid equality | id/table index equals handler count | precise handler error | unit/property/Kani |
| handler wrong kind | required/actual kind differ | precise handler kind error | unit/property |
| action contracts match | unique Do IDs equal contract IDs | `Ok(())` from `validate_with_contracts` | integration/property/Kani |
| direct action helper contracts match | unique Do IDs equal contract IDs | `Ok(())` from `validate_action_references` | unit/property/Kani |
| duplicate Do IDs | two Do nodes same ID, one matching contract | `Ok(())` | unit/property |
| missing action contract | Do ID absent from contracts | `ActionContractMissing { action_id, node_index }` from `validate_with_contracts` and direct `validate_action_references` | unit/integration/mutation |
| orphan action contract | contract ID not in Do set | `ActionContractOrphan { action_id }` from `validate_with_contracts` and direct `validate_action_references` | unit/integration/mutation |
| default validate with missing action contract | valid non-action gates, Do without contract | `Ok(())` from `validate` | integration/mutation |
| declared hard limit equality | declared equals hard limit | success if actual <= declared | unit/property/Kani |
| direct resource helper valid | declared within hard limit and actual <= declared | `Ok(())` from `validate_resource_references` | unit |
| declared hard limit + 1 | each resource member | `ResourceContractTooLarge { resource }` from core and direct `validate_resource_references` | unit/property/Kani/mutation |
| actual equals declared | each resource member | success | unit/property/Kani |
| actual declared + 1 | each resource member | `ResourceContractExceeded { resource }` from core and direct `validate_resource_references` | unit/property/Kani/mutation |
| cross-artifact reference | foreign owner token/id where representable | precise ownership/reference error | integration/property |
| diagnostic rendering | each error variant | exact stable code + salient fields | unit/integration |
| static banned constructs | touched source | no unsafe/unwrap/expect/panic/todo/unimplemented/dbg/unchecked ops | static |
| runtime I/O scan | touched runtime validation source | no JSON/YAML/HTTP/fs/network lookup | static |
| CLI/manual invalid symbol | fixture invalid accessor symbol | non-zero exit + stable diagnostic code/fields | manual/E2E |
| CLI/manual invalid resource | fixture resource exceeded | non-zero exit + stable diagnostic code/fields | manual/E2E |

## 9. Contract and Proof Obligation Traceability

| Clause/obligation | Required tests/gates | Waiver |
|---|---|---|
| PRE-001 | `try_from_parts_rejects_untrusted_parts_before_workflow_is_returned`; `given_untrusted_workflow_parts_when_admitted_then_reference_integrity_is_checked`; `moon run :verify-fast` | none |
| PRE-002 | no-mutation snapshot tests; Miri/cargo-careful via `moon run :verify-deep` | Lean waiver only, compensated by Rust evidence |
| PRE-003 | invalid IR `Result::Err` no-panic tests; static banned-construct scan | Lean waiver only |
| PRE-004 / POST-004 / AC-006 | `validate_returns_success_for_do_action_without_contract_when_non_action_gates_pass`; mutation adding Gate 12 to `validate` | Lean/API compat waiver only as documented |
| PRE-005 | proptest generated numeric IDs/resources; Kani bounds | fuzz waived by W-FUZZ-001 if no parser/codec touched |
| PRE-006 / AC-008 | runtime-core no-I/O static scan | Lean waiver only |
| POST-001 / INV-001 / AC-001 / THM-INV-001 | direct `validate_symbol_references` success/failure scenarios; all symbol carrier examples; symbol proptest; Kani symbol harness; Lean theorem `all_symbol_refs_bounded_iff_valid` | none |
| POST-002 / INV-002 / AC-002 / THM-INV-002 | zero-symbol carrier tests; proptest anti-invariant; Lean theorem `zero_symbols_rejects_symbol_ref` | none |
| POST-003 / INV-003 / THM-INV-003 | direct `validate_action_references` success/missing/orphan scenarios; action set equality tests; duplicate Do test; proptest; Kani; Lean theorem `action_contract_bijection_exact` | none |
| POST-005 / INV-004 / ERR-006 / THM-INV-004 | slot bounds/kind/ownership tests; proptest; Kani; Lean theorem `all_slot_refs_owned_and_bounded` | none |
| POST-005/006 / INV-005 / ERR-007 / THM-INV-005 | constant bounds/kind tests and symbol constant tests; proptest; Kani; Lean theorem | none |
| POST-005/006 / INV-006 / ERR-008 / THM-INV-006 | handler bounds/kind/ownership tests; proptest; Kani; Lean theorem | none |
| POST-007 / THM-POST-007 | cross-artifact reference tests; action contract supplied-set ownership tests; Lean theorem `valid_references_are_artifact_or_contract_owned` | runtime shell exclusions per Lean contract |
| POST-008 / INV-007/008 / AC-007 / THM-INV-007/008 | direct `validate_resource_references` success/too-large/exceeded scenarios; resource hard-limit and coverage tests; proptest; Kani; Lean resource theorems | none |
| POST-009 | exact typed error/no partial workflow tests; mutation; static scan | Lean waiver only |
| INV-009 | determinism repeated-run tests; no-I/O/unbounded static scan; coverage | Lean waiver only |
| INV-010 | repository governance scans; no banned constructs/unchecked ops; Miri/cargo-careful | Lean waiver only |
| ERR-001 | direct `validate_symbol_references` out-of-bounds scenario; symbol out-of-bounds tests for each carrier and zero symbol | none |
| ERR-002 | direct `validate_resource_references` too-large scenario; one too-large test per resource member plus boundary equality | none |
| ERR-003 | direct `validate_resource_references` exceeded scenario; one exceeded test per resource member plus boundary equality | none |
| ERR-004 | direct `validate_action_references` missing scenario and pipeline missing tests asserting `action_id` and `node_index` | none |
| ERR-005 | direct `validate_action_references` orphan scenario and pipeline orphan tests asserting `action_id` | none |
| ERR-009 / AC-009 | diagnostic code/rendering tests for every new/existing reference error | Lean waiver only |
| AC-010 | `moon ci`, `moon run :verify-fast`, `moon run :verify-standard`, `moon run :verify-proof`, `moon run :verify-all` evidence | process evidence, not Lean |
| W-FUZZ-001 | waiver validity review plus proptest/Kani; conditional fuzz if codec touched | active only if no parser/codec touched |
| W-LOOM-001 | static scan confirms no concurrency primitives | active; no concurrency scope |
| W-PERF-001 | no performance claim review; deterministic bounded validation evidence | active |
| W-API-001 | compile/downstream tests; semver checks only if external public API changes | conditional |
| W-REL-001 | no release artifact review | active |

## 10. Red-Phase Expectations for Test Writer

Add tests first and prove red before implementation. Do not weaken assertions to compile around missing variants. If an expected verifier variant/code named in this plan is absent, implementation must either add exactly that variant/code or amend `contract.md` before green.

| Red test to add | Intended initial failure | Command to prove red |
|---|---|---|
| `validate_symbol_references_returns_unit_when_all_symbol_carriers_are_in_bounds` / `validate_symbol_references_returns_symbol_out_of_bounds_when_accessor_field_equals_symbols_count` | public helper absent/stubbed or fails to traverse accessor symbols | `cargo test -p vb_core validate_symbol_references_returns -- --nocapture` |
| `validate_resource_references_returns_unit_when_declared_and_actual_resources_are_within_limits` / too-large / exceeded direct helper tests | public helper absent/stubbed or resource helper not factored for direct call | `cargo test -p vb_core validate_resource_references_returns -- --nocapture` |
| `validate_action_references_returns_unit_when_unique_do_actions_equal_unique_contract_ids` / missing / orphan direct helper tests | public helper absent/stubbed or Gate 12 is not directly exposed | `cargo test -p vb_validate validate_action_references_returns -- --nocapture` |
| `accessor_field_symbol_equal_to_symbols_count_returns_symbol_out_of_bounds` | verifier currently misses accessor field symbol bounds or lacks precise verifier error | `cargo test -p vb_core -p vb_validate accessor_field_symbol_equal_to_symbols_count_returns_symbol_out_of_bounds -- --nocapture` |
| `symbol_constant_equal_to_symbols_count_returns_symbol_out_of_bounds` | verifier/core parity gap for `ConstValue::Symbol` | `cargo test -p vb_core -p vb_validate symbol_constant_equal_to_symbols_count_returns_symbol_out_of_bounds -- --nocapture` |
| `build_object_field_symbol_equal_to_symbols_count_returns_symbol_out_of_bounds` | build-object field carrier not traversed or not mapped | `cargo test -p vb_core -p vb_validate build_object_field_symbol_equal_to_symbols_count_returns_symbol_out_of_bounds -- --nocapture` |
| `zero_symbols_rejects_accessor_symbol_zero` / constant / build-object | zero-symbol edge accepted | `cargo test -p vb_core -p vb_validate zero_symbols_rejects -- --nocapture` |
| `slot_reference_equal_to_slot_count_returns_precise_slot_error` | slot gate absent/incomplete or generic error | `cargo test -p vb_core -p vb_validate slot_reference_equal_to_slot_count_returns_precise_slot_error -- --nocapture` |
| `constant_reference_equal_to_constants_len_returns_precise_constant_error` | constant gate absent/incomplete | `cargo test -p vb_core -p vb_validate constant_reference_equal_to_constants_len_returns_precise_constant_error -- --nocapture` |
| `handler_reference_equal_to_handler_count_returns_precise_handler_error` | handler gate absent/incomplete | `cargo test -p vb_core -p vb_validate handler_reference_equal_to_handler_count_returns_precise_handler_error -- --nocapture` |
| `validate_with_contracts_returns_missing_contract_when_do_action_has_no_contract` | Gate 12 not called by action-complete path or wrong payload | `cargo test -p vb_validate validate_with_contracts_returns_missing_contract_when_do_action_has_no_contract -- --nocapture` |
| `validate_returns_success_for_do_action_without_contract_when_non_action_gates_pass` | default validate incorrectly claims action completeness | `cargo test -p vb_validate validate_returns_success_for_do_action_without_contract_when_non_action_gates_pass -- --nocapture` |
| `orphan_action_contract_reports_first_orphan_in_supplied_contract_order` | orphan check missing or nondeterministic | `cargo test -p vb_validate orphan_action_contract_reports_first_orphan_in_supplied_contract_order -- --nocapture` |
| six `declared_*_above_hard_limit_returns_resource_contract_too_large` tests | missing hard-limit member check or wrong resource name | `cargo test -p vb_core -p vb_validate resource_contract_too_large -- --nocapture` |
| six `*_above_max_*_returns_resource_contract_exceeded` tests | missing actual-usage check or wrong resource name | `cargo test -p vb_core -p vb_validate resource_contract_exceeded -- --nocapture` |
| diagnostic stable-code tests | new variants lack code/renderer coverage | `cargo test -p vb_validate diagnostic_for_ -- --nocapture` |
| no-I/O static scan | implementation imports runtime JSON/YAML/HTTP/fs/network lookup | scan command in Section 11 |

Red proof rule: at least one targeted command above must fail for the intended reason before production changes. After implementation, the same command must pass with exact assertions unchanged.

## 11. Static, Manual, and Gauntlet Gates

Required after implementation:

1. `moon ci`
2. `moon run :verify-fast`
3. `moon run :verify-standard`
4. `moon run :verify-proof`
5. `moon run :verify-deep`
6. `moon run :verify-all`
7. Mutation: `cargo mutants --package vb_core --package vb_validate --minimum-test-timeout 60` or workspace-approved equivalent scoped to touched validation/admission files; require `>= 90%` killed and all Section 7 critical mutants killed.
8. Banned construct/unchecked operation scan from workspace root:
   - `rg --line-number --with-filename --glob 'crates/vb_core/src/**' --glob 'crates/vb_validate/src/**' --glob '!**/tests/**' --glob '!**/test_support/**' 'unsafe|\.unwrap\(|\.expect\(|panic!|todo!|unimplemented!|dbg!|\[[^\]]+\]| as (u8|u16|u32|u64|usize|i8|i16|i32|i64|isize)'`
   - Pass: no matches in touched runtime source; any pre-existing untouched match must be recorded with `file:line` evidence.
9. Runtime I/O/config dependency scan from workspace root:
   - `rg --line-number --with-filename --glob 'crates/vb_core/src/**' --glob 'crates/vb_validate/src/**' --glob '!**/tests/**' --glob '!**/test_support/**' 'std::fs|tokio::fs|std::net|tokio::net|reqwest|hyper|ureq|serde_yaml|serde_json|yaml|http|https|File::open|read_to_string|TcpStream|UdpSocket'`
   - Pass: no matches in touched validation/admission source.
10. Conditional fuzz command only if waiver voids because parser/codec boundary changed: run the project-standard fuzz target for the touched decoder for at least 60 seconds with corpus seeds from Section 5.
11. Manual/E2E smoke, if CLI compiled artifact validation path exists: invalid symbol fixture returns non-zero exit and exact stable diagnostic; invalid resource fixture returns non-zero exit and exact stable diagnostic.

## Open Questions

None blocking for the three rejected public helper gaps. The plan now names direct BDD scenarios for `validate_symbol_references`, `validate_resource_references`, and `validate_action_references`.

Required contract amendments if verifier parity remains in scope beyond existing variants:

- Add `ValidationError::SymbolReferenceOutOfRange { symbol: usize, symbols_count: usize, context: String }`, code `0x050D`.
- Add `ValidationError::SlotReferenceWrongKind { slot: usize, required: String, actual: String, context: String }`, code `0x050E`, or waive slot kind if not represented.
- Add `ValidationError::ConstantReferenceOutOfRange { constant: usize, constant_count: usize, context: String }`, code `0x050F`.
- Add `ValidationError::ConstantReferenceWrongKind { constant: usize, required: String, actual: String, context: String }`, code `0x0510`.
- Add `ValidationError::HandlerReferenceWrongKind { handler: usize, required: String, actual: String, context: String }`, code `0x0511`, or amend handler scope to existing `StepOutOfBounds` / `LoopBodyStepOutOfRange` semantics.
- Add `ValidationError::ResourceContractTooLarge { resource: String }`, code `0x0512`, and `ValidationError::ResourceContractExceeded { resource: String }`, code `0x0513`, if verifier resource parity is required instead of core/direct-helper-only resource validation.
