# Test Plan: vb-qi37.7.3 — IR reference validation

## Review Repair Statement

This repaired plan explicitly addresses every rejection in `.beads/vb-qi37.7.3/test-plan-review.md`:

- Direct BDD scenarios are added for `validate_symbol_references`, `validate_resource_references`, and `validate_action_references`, and their expected error enums match the contract signatures exactly.
- Unit density is raised to **36 mandatory unit tests** for the 6 public contract functions, exceeding the required minimum of 30.
- All prior conditional escape hatches are removed; this plan pins one expected enum, one diagnostic code set, and one static-gate evidence shape.
- Verifier symbol parity is now the fixed contract: `vb_validate` must validate accessor, constant, and build-object symbol references.
- Exact error variants, fields, deterministic first-failure ordering, resource strings, diagnostic codes, mutation targets, executable static gates, and CLI evidence are pinned.
- Latest rejection repaired: direct `validate_symbol_references` and `validate_resource_references` scenarios now assert `WorkflowError` variants, while pipeline `validate`/`validate_with_contracts` scenarios assert mapped `ValidationError` variants; static/Holzmann gates now include executable commands, target paths, file:line evidence rules, and failure criteria.

## Summary

- Behaviors identified: 20
- Trophy allocation: 36 unit / 28 integration / 2 E2E / 8 static gate checks
- Proptest invariants: 6
- Fuzz targets: 2
- Kani harnesses: 4
- Mutation threshold: `cargo-mutants` must kill **>= 90%** of mutants in touched validation modules, with the critical mutants listed in section 7 killed by named tests.
- Canonical acceptance gate: `moon ci`.

## Fixed Error and Diagnostic Contract

Tests must assert these exact variants and fields. No assertion may use only `is_ok()` or `is_err()`.

### Core admission and direct helper errors

- `WorkflowError::SymbolOutOfBounds { symbol }`
  - Returned by `CompiledWorkflow::try_from_parts(parts)` and direct `validate_symbol_references(&parts)`.
  - `symbol` equals the offending `SymbolId`.
- `WorkflowError::ResourceContractTooLarge { resource }`
  - Returned by `CompiledWorkflow::try_from_parts(parts)` and direct `validate_resource_references(&parts)`.
  - `resource` is exactly one of: `"max_steps"`, `"max_slots"`, `"max_constants"`, `"max_accessors"`, `"max_expressions"`, `"max_expr_stack"`.
- `WorkflowError::ResourceContractExceeded { resource }`
  - Returned by `CompiledWorkflow::try_from_parts(parts)` and direct `validate_resource_references(&parts)`.
  - `resource` is exactly one of: `"max_steps"`, `"max_slots"`, `"max_constants"`, `"max_accessors"`, `"max_expressions"`, `"max_expr_stack"`.

### Verifier errors required by this bead

The pipeline functions `vb_validate::shared::validate(parts)` and `vb_validate::shared::validate_with_contracts(parts, action_contracts)` must map direct helper/core failures into these exact public verifier variants and diagnostic rendering:

- `ValidationError::SymbolReferenceOutOfRange { symbol: usize, source: SymbolReferenceSource, source_index: usize }`
  - `source` is exactly `SymbolReferenceSource::AccessorField`, `SymbolReferenceSource::ConstantPool`, or `SymbolReferenceSource::BuildObjectField`.
  - Diagnostic code: `E050D`.
- `ValidationError::ResourceContractTooLarge { resource: &'static str, declared: usize, hard_limit: usize }`
  - Diagnostic code: `E050E`.
- `ValidationError::ResourceContractExceeded { resource: &'static str, actual: usize, declared: usize }`
  - Diagnostic code: `E050F`.
- Existing `ValidationError::ActionContractMissing { action_id: usize, node_index: usize }`
  - Diagnostic code remains `E0509`.
- Existing `ValidationError::ActionContractOrphan { action_id: usize }`
  - Diagnostic code remains `E050A`.

### Deterministic first-failure ordering

Validation must short-circuit in this order:

1. Existing non-action gates in ascending gate order.
2. Symbol reference validation carrier order: accessors, constants, build-object fields; within each carrier, slice iteration order.
3. Resource validation order: `max_steps`, `max_slots`, `max_constants`, `max_accessors`, `max_expressions`, `max_expr_stack`.
4. Action validation order: missing contracts in node-index order, then orphan contracts in supplied-contract slice order.

## 1. Behavior Inventory

1. `validate_symbol_references` returns `Ok(())` when all accessor, constant, and build-object symbols are `< symbols_count`.
2. `validate_symbol_references` rejects accessor field symbols with `WorkflowError::SymbolOutOfBounds` when `symbol.get() >= symbols_count`.
3. `validate_symbol_references` rejects symbol constants with `WorkflowError::SymbolOutOfBounds` when `symbol.get() >= symbols_count`.
4. `validate_symbol_references` rejects build-object field keys with `WorkflowError::SymbolOutOfBounds` when `symbol.get() >= symbols_count`.
5. `validate_symbol_references` rejects every symbol carrier with `WorkflowError::SymbolOutOfBounds` when `symbols_count == 0`.
6. `CompiledWorkflow::try_from_parts` accepts a valid artifact and preserves public workflow counts when all symbol/resource checks pass.
7. `CompiledWorkflow::try_from_parts` rejects all three symbol carriers with `WorkflowError::SymbolOutOfBounds` when out of bounds.
8. `validate_resource_references` returns `Ok(())` when declared resources are within hard limits and cover actual usage.
9. `validate_resource_references` rejects each declared resource member above its hard limit with `WorkflowError::ResourceContractTooLarge`.
10. `validate_resource_references` rejects each actual usage count above its declaration with `WorkflowError::ResourceContractExceeded`.
11. `CompiledWorkflow::try_from_parts` rejects declared resource members above hard limits with `WorkflowError::ResourceContractTooLarge`.
12. `CompiledWorkflow::try_from_parts` rejects actual resource usage above declarations with `WorkflowError::ResourceContractExceeded`.
13. `validate_action_references` returns `Ok(())` when unique `Do.action` IDs equal unique `ActionContract.id` values.
14. `validate_action_references` rejects the first missing contract in node-index order with `ValidationError::ActionContractMissing`.
15. `validate_action_references` rejects the first orphan contract in supplied-contract order with `ValidationError::ActionContractOrphan`.
16. `validate(parts)` skips action-contract completeness but includes symbol and resource reference validation.
17. `validate_with_contracts(parts, contracts)` includes symbol, resource, and action reference validation.
18. Diagnostic rendering preserves stable exact codes and fields for all new and existing reference-validation errors.
19. Validation failure is atomic and does not mutate borrowed parts/contracts or return a partially accepted workflow.
20. Validation remains deterministic, bounded, panic-free, and free of runtime JSON/YAML/HTTP/filesystem/network lookup in the runtime core.

## 2. Trophy Allocation and Density

| Public contract surface | Mandatory unit tests | Integration tests | Why |
|---|---:|---:|---|
| `validate_symbol_references` | 8 | 3 | Pure bounded scan over symbol carriers; needs carrier and zero-bound exhaustiveness. |
| `validate_resource_references` | 14 | 4 | Pure bounded resource comparison; every member and boundary must be pinned. |
| `validate_action_references` | 6 | 4 | Set-bijection with ordering and duplicate behavior. |
| `validate` | 2 | 5 | Public pipeline behavior; default skips Gate 12 but includes symbol/resource checks. |
| `validate_with_contracts` | 2 | 6 | Public pipeline with Gate 12; proves non-action gates run before action gate. |
| `CompiledWorkflow::try_from_parts` | 4 | 6 | Core admission consumes owned artifact; preserve counts and exact core errors. |
| Diagnostics/static/CLI | 0 | 2 E2E + 8 static | Black-box user and source-policy evidence. |

Total mandatory unit density: **36 unit tests / 6 public functions = 6x**, above the required `>= 5x` density.

## 3. BDD Scenarios

### Behavior 1: `validate_symbol_references` accepts all in-bounds symbol carriers

Test: `fn validate_symbol_references_returns_unit_when_all_symbol_carriers_are_in_bounds()`

Given: `WorkflowParts` with `symbols_count = 3`, accessor field `SymbolId::new(0)`, constant `ConstValue::Symbol(SymbolId::new(1))`, and build-object field key `SymbolId::new(2)`.
When: `validate_symbol_references(&parts)` is called.
Then: the result is exactly `Ok(())`.

### Behavior 2: `validate_symbol_references` rejects accessor field out of bounds

Test: `fn validate_symbol_references_returns_symbol_out_of_bounds_when_accessor_field_equals_symbols_count()`

Given: `WorkflowParts` with `symbols_count = 1` and accessor index `0` containing `PathSegment::Field(SymbolId::new(1))`.
When: `validate_symbol_references(&parts)` is called.
Then: result is `Err(WorkflowError::SymbolOutOfBounds { symbol: SymbolId::new(1) })`.

Boundary test: `fn validate_symbol_references_accepts_accessor_field_at_upper_valid_boundary()` expects `Ok(())` for `SymbolId::new(0)` with `symbols_count = 1`.

### Behavior 3: `validate_symbol_references` rejects symbol constant out of bounds

Test: `fn validate_symbol_references_returns_symbol_out_of_bounds_when_symbol_constant_equals_symbols_count()`

Given: `WorkflowParts` with `symbols_count = 2` and constant index `0` equal to `ConstValue::Symbol(SymbolId::new(2))`.
When: `validate_symbol_references(&parts)` is called.
Then: result is `Err(WorkflowError::SymbolOutOfBounds { symbol: SymbolId::new(2) })`.

Boundary test: `fn validate_symbol_references_accepts_symbol_constant_at_upper_valid_boundary()` expects `Ok(())` for `SymbolId::new(1)` with `symbols_count = 2`.

### Behavior 4: `validate_symbol_references` rejects build-object field key out of bounds

Test: `fn validate_symbol_references_returns_symbol_out_of_bounds_when_build_object_field_equals_symbols_count()`

Given: `WorkflowParts` with `symbols_count = 2` and build-object node index `0` containing field key `SymbolId::new(2)`.
When: `validate_symbol_references(&parts)` is called.
Then: result is `Err(WorkflowError::SymbolOutOfBounds { symbol: SymbolId::new(2) })`.

Boundary test: `fn validate_symbol_references_accepts_build_object_field_at_upper_valid_boundary()` expects `Ok(())` for `SymbolId::new(1)` with `symbols_count = 2`.

### Behavior 5: zero symbols rejects every symbol carrier

Tests:
- `fn validate_symbol_references_rejects_accessor_field_when_symbols_count_is_zero()` expects `Err(WorkflowError::SymbolOutOfBounds { symbol: SymbolId::new(0) })`.
- `fn validate_symbol_references_rejects_symbol_constant_when_symbols_count_is_zero()` expects `Err(WorkflowError::SymbolOutOfBounds { symbol: SymbolId::new(0) })`.
- `fn validate_symbol_references_rejects_build_object_field_when_symbols_count_is_zero()` expects `Err(WorkflowError::SymbolOutOfBounds { symbol: SymbolId::new(0) })`.

### Behavior 6: core admission accepts valid references and preserves public counts

Test: `fn core_admission_preserves_workflow_counts_when_reference_validation_passes()`

Given: owned `WorkflowParts` with one entry node, `slot_count = 1`, one constant, one accessor, one expression, `symbols_count = 3`, and resource contract exactly matching the counts.
When: `CompiledWorkflow::try_from_parts(parts)` is called.
Then: it returns `Ok(workflow)` and public observations equal: `workflow.nodes().len() == 1`, `workflow.slot_count() == 1`, `workflow.constants().len() == 1`, `workflow.accessors().len() == 1`, `workflow.expressions().len() == 1`, and `workflow.symbols_count() == 3`. The implementation must expose any missing public read-only count accessor required for these exact assertions before tests are written; `Ok(Default::default())` must fail this test.

### Behavior 7: core admission rejects symbol carriers with exact core variant

Tests:
- `fn core_admission_returns_symbol_out_of_bounds_when_accessor_field_equals_symbols_count()` expects `Err(WorkflowError::SymbolOutOfBounds { symbol: SymbolId::new(1) })`.
- `fn core_admission_returns_symbol_out_of_bounds_when_symbol_constant_equals_symbols_count()` expects `Err(WorkflowError::SymbolOutOfBounds { symbol: SymbolId::new(2) })`.
- `fn core_admission_returns_symbol_out_of_bounds_when_build_object_field_equals_symbols_count()` expects `Err(WorkflowError::SymbolOutOfBounds { symbol: SymbolId::new(2) })`.
- `fn core_admission_returns_symbol_out_of_bounds_when_symbol_carrier_exists_with_zero_symbols()` uses one test per carrier and expects `symbol == SymbolId::new(0)`.

### Behavior 8: `validate_resource_references` accepts covered resources

Test: `fn validate_resource_references_returns_unit_when_declared_resources_cover_actual_usage()`

Given: `WorkflowParts` where actual counts equal declared counts for `max_steps`, `max_slots`, `max_constants`, `max_accessors`, `max_expressions`, and every expression `max_stack == max_expr_stack`, with all declarations at or below hard limits.
When: `validate_resource_references(&parts)` is called.
Then: result is exactly `Ok(())`.

### Behavior 9: `validate_resource_references` rejects every declared hard-limit violation

Tests, one per resource member:

- `fn validate_resource_references_returns_too_large_when_max_steps_exceeds_hard_limit()` expects `Err(WorkflowError::ResourceContractTooLarge { resource: "max_steps" })` and the fixture sets declared `MAX_STEPS_PER_WORKFLOW + 1` against hard limit `MAX_STEPS_PER_WORKFLOW`.
- `fn validate_resource_references_returns_too_large_when_max_slots_exceeds_hard_limit()` expects `Err(WorkflowError::ResourceContractTooLarge { resource: "max_slots" })`.
- `fn validate_resource_references_returns_too_large_when_max_constants_exceeds_hard_limit()` expects `Err(WorkflowError::ResourceContractTooLarge { resource: "max_constants" })`.
- `fn validate_resource_references_returns_too_large_when_max_accessors_exceeds_hard_limit()` expects `Err(WorkflowError::ResourceContractTooLarge { resource: "max_accessors" })`.
- `fn validate_resource_references_returns_too_large_when_max_expressions_exceeds_hard_limit()` expects `Err(WorkflowError::ResourceContractTooLarge { resource: "max_expressions" })`.
- `fn validate_resource_references_returns_too_large_when_max_expr_stack_exceeds_hard_limit()` expects `Err(WorkflowError::ResourceContractTooLarge { resource: "max_expr_stack" })`.

Boundary tests: one per member at exactly the hard limit must return `Ok(())` when actual usage is valid.

### Behavior 10: `validate_resource_references` rejects actual usage over declared contract

Tests, one per resource member:

- `fn validate_resource_references_returns_exceeded_when_node_count_exceeds_max_steps()` expects `Err(WorkflowError::ResourceContractExceeded { resource: "max_steps" })` with fixture actual `2`, declared `1`.
- `fn validate_resource_references_returns_exceeded_when_slot_count_exceeds_max_slots()` expects `Err(WorkflowError::ResourceContractExceeded { resource: "max_slots" })` with fixture actual `2`, declared `1`.
- `fn validate_resource_references_returns_exceeded_when_constants_len_exceeds_max_constants()` expects `Err(WorkflowError::ResourceContractExceeded { resource: "max_constants" })` with fixture actual `2`, declared `1`.
- `fn validate_resource_references_returns_exceeded_when_accessors_len_exceeds_max_accessors()` expects `Err(WorkflowError::ResourceContractExceeded { resource: "max_accessors" })` with fixture actual `2`, declared `1`.
- `fn validate_resource_references_returns_exceeded_when_expressions_len_exceeds_max_expressions()` expects `Err(WorkflowError::ResourceContractExceeded { resource: "max_expressions" })` with fixture actual `2`, declared `1`.
- `fn validate_resource_references_returns_exceeded_when_expression_max_stack_exceeds_max_expr_stack()` expects `Err(WorkflowError::ResourceContractExceeded { resource: "max_expr_stack" })` with fixture actual `3`, declared `2`.

Boundary tests: equality for each member must return `Ok(())`.

### Behavior 11: core admission rejects declared resources above hard limits

Tests:
- `fn core_admission_returns_resource_contract_too_large_when_max_steps_exceeds_hard_limit()` expects `Err(WorkflowError::ResourceContractTooLarge { resource: "max_steps" })`.
- Repeat for exact resource strings `"max_slots"`, `"max_constants"`, `"max_accessors"`, `"max_expressions"`, and `"max_expr_stack"`.

### Behavior 12: core admission rejects actual resource usage above declarations

Tests:
- `fn core_admission_returns_resource_contract_exceeded_when_node_count_exceeds_max_steps()` expects `Err(WorkflowError::ResourceContractExceeded { resource: "max_steps" })`.
- Repeat for exact resource strings `"max_slots"`, `"max_constants"`, `"max_accessors"`, `"max_expressions"`, and `"max_expr_stack"`.

### Behavior 13: `validate_action_references` accepts matching action contracts

Test: `fn validate_action_references_returns_unit_when_do_actions_match_contract_ids()`

Given: one `Do` node with `ActionId::new(7)` and one `ActionContract { id: ActionId::new(7), .. }`.
When: `validate_action_references(&parts, &contracts)` is called.
Then: result is exactly `Ok(())`.

Duplicate test: `fn validate_action_references_accepts_one_contract_for_duplicate_do_action_ids()` expects `Ok(())` for two `Do(7)` nodes and one contract `7`.

### Behavior 14: `validate_action_references` rejects missing contracts in node-index order

Test: `fn validate_action_references_returns_first_missing_contract_in_node_index_order()`

Given: `Do(7)` at node index `2`, `Do(9)` at node index `4`, and no supplied contracts.
When: `validate_action_references(&parts, &[])` is called.
Then: result is `Err(ValidationError::ActionContractMissing { action_id: 7, node_index: 2 })`.

Field swap test: `fn validate_action_references_preserves_action_id_and_node_index_fields_for_missing_contract()` expects `action_id == 7` and `node_index == 3` for `Do(7)` at node 3.

### Behavior 15: `validate_action_references` rejects orphan contracts in supplied order

Test: `fn validate_action_references_returns_first_orphan_contract_in_supplied_order()`

Given: no `Do` nodes and contracts `[ActionId::new(9), ActionId::new(11)]`.
When: `validate_action_references(&parts, &contracts)` is called.
Then: result is `Err(ValidationError::ActionContractOrphan { action_id: 9 })`.

Ordering test: `fn validate_action_references_reports_missing_before_orphan_when_both_exist()` expects `Err(ValidationError::ActionContractMissing { action_id: 7, node_index: 0 })` for parts with `Do(7)` and contracts `[9]`.

### Behavior 16: default `validate` skips action contracts but validates symbols/resources

Test: `fn shared_validate_returns_unit_for_do_node_without_contracts_when_non_action_gates_pass()`

Given: valid parts containing `Do(7)` and no contracts are provided because `validate` has no contract argument.
When: `vb_validate::shared::validate(&parts)` is called.
Then: result is exactly `Ok(())`.

Symbol test: `fn shared_validate_returns_symbol_reference_out_of_range_for_invalid_accessor_symbol()` expects `Err(ValidationError::SymbolReferenceOutOfRange { symbol: 1, source: SymbolReferenceSource::AccessorField, source_index: 0 })`.

Resource test: `fn shared_validate_returns_resource_contract_exceeded_for_node_count_over_contract()` expects `Err(ValidationError::ResourceContractExceeded { resource: "max_steps", actual: 2, declared: 1 })`.

### Behavior 17: `validate_with_contracts` includes non-action gates and action references

Test: `fn validate_with_contracts_returns_action_contract_missing_after_non_action_gates_pass()`

Given: parts with valid symbols/resources and `Do(7)` at node index `0`, and empty contracts.
When: `validate_with_contracts(&parts, &[])` is called.
Then: result is `Err(ValidationError::ActionContractMissing { action_id: 7, node_index: 0 })`.

Precedence tests:
- `fn validate_with_contracts_returns_symbol_error_before_action_error()` expects symbol error when both invalid symbol and missing action exist.
- `fn validate_with_contracts_returns_resource_error_before_action_error()` expects resource error when both invalid resource and missing action exist.

### Behavior 18: diagnostics preserve exact codes and messages

Tests:
- `fn diagnostic_from_error_returns_e050d_for_symbol_reference_out_of_range()` expects code `DiagnosticCode::new(0x050D)` and message contains `symbol 1`, `AccessorField`, and `source_index 0`.
- `fn diagnostic_from_error_returns_e050e_for_resource_contract_too_large()` expects code `0x050E`, resource `max_steps`, declared value, and hard limit in message.
- `fn diagnostic_from_error_returns_e050f_for_resource_contract_exceeded()` expects code `0x050F`, resource `max_steps`, actual value, and declared value in message.
- `fn diagnostic_from_error_returns_e0509_for_action_contract_missing()` expects code `0x0509`, action `7`, and node `3` in message.
- `fn diagnostic_from_error_returns_e050a_for_action_contract_orphan()` expects code `0x050A` and action `9` in message.

### Behavior 19: validation failure is atomic

Tests:
- `fn validate_symbol_references_does_not_mutate_parts_when_symbol_validation_fails()` compares `parts == original_parts` after exact symbol error.
- `fn validate_resource_references_does_not_mutate_parts_when_resource_validation_fails()` compares `parts == original_parts` after exact resource error.
- `fn validate_action_references_does_not_mutate_parts_or_contracts_when_action_validation_fails()` compares both inputs to snapshots after exact action error.
- `fn core_admission_returns_no_workflow_when_reference_validation_fails()` pattern-matches exact error and proves no `CompiledWorkflow` value exists in the `Err` branch.

### Behavior 20: bounded deterministic source policy and CLI oracle

Determinism test: `fn validation_returns_same_exact_error_on_repeated_runs_for_same_invalid_ir()` runs symbol, resource, and action invalid fixtures twice and compares exact `Result` values.

E2E CLI test 1: `fn cli_validate_reports_e050d_for_invalid_symbol_reference_fixture()`

Given: fixture `tests/fixtures/vb_qi37_7_3/invalid_symbol_accessor.vbir` containing accessor `SymbolId::new(1)` with `symbols_count = 1`.
When: `cargo run -p velvet-ballastics -- validate tests/fixtures/vb_qi37_7_3/invalid_symbol_accessor.vbir --json` is executed.
Then: process exit code is `1`; stderr or JSON error output contains `E050D`, `SymbolReferenceOutOfRange`, `symbol 1`, and `AccessorField`; output does not contain `UNKNOWN` or `internal error`.

E2E CLI test 2: `fn cli_run_compiled_reports_resource_error_for_invalid_resource_fixture()`

Given: fixture `tests/fixtures/vb_qi37_7_3/resource_max_steps_exceeded.vbir` with two nodes and `max_steps = 1`.
When: `cargo run -p velvet-ballastics -- run-compiled tests/fixtures/vb_qi37_7_3/resource_max_steps_exceeded.vbir --input-bin tests/fixtures/vb_qi37_7_3/input.bin --durability memory --json` is executed.
Then: process exit code is `1`; output contains `E050F`, `ResourceContractExceeded`, `max_steps`, `actual 2`, and `declared 1`.

## 4. Proptest Invariants

### Proptest: symbol bounds over all carriers

Invariant: For generated parts with every symbol carrier `< symbols_count`, direct `validate_symbol_references` returns `Ok(())`; for exactly one offending carrier `>= symbols_count`, direct `validate_symbol_references` returns `WorkflowError::SymbolOutOfBounds` with the generated `SymbolId`, while pipeline `validate` maps the same fixture to `ValidationError::SymbolReferenceOutOfRange` with the generated symbol/source/source_index.
Strategy: bounded `symbols_count in 0..=16`; carrier enum for accessor/constant/build-object; valid unrelated structure.
Anti-invariant: `symbols_count == 0` plus any generated symbol carrier always fails direct helper/core validation with exact `WorkflowError::SymbolOutOfBounds { symbol: SymbolId::new(0) }`; pipeline validation additionally reports the exact source context.

### Proptest: core and verifier symbol parity

Invariant: For any single invalid symbol carrier, direct `validate_symbol_references` and `CompiledWorkflow::try_from_parts` both return `WorkflowError::SymbolOutOfBounds` for the same numeric symbol; pipeline `validate` returns `ValidationError::SymbolReferenceOutOfRange` for that same symbol and source.
Strategy: generate one invalid carrier at a time, keep every non-target field valid.
Anti-invariant: Changing the same symbol to `symbols_count - 1` when `symbols_count > 0` makes both validations pass symbol checks.

### Proptest: resource coverage

Invariant: If `actual <= declared <= hard_limit` for every resource member, direct `validate_resource_references` returns `Ok(())`; if one member has `declared > hard_limit`, direct validation returns `WorkflowError::ResourceContractTooLarge { resource }`; if one member has `actual > declared`, direct validation returns `WorkflowError::ResourceContractExceeded { resource }`. The pipeline `validate` maps the same fixtures to `ValidationError::ResourceContractTooLarge { resource, declared, hard_limit }` or `ValidationError::ResourceContractExceeded { resource, actual, declared }`.
Strategy: generate small bounded counts for all resource members and derive declarations.
Anti-invariant: lower exactly one declaration by one with no earlier failures.

### Proptest: core and verifier resource parity

Invariant: For the same resource member violation, verifier error `ValidationError::{ResourceContractTooLarge|ResourceContractExceeded}` and core error `WorkflowError::{ResourceContractTooLarge|ResourceContractExceeded}` use the same exact `resource` string.
Strategy: generate one violation at a time for the six resource strings.
Anti-invariant: swapping resource names must fail exact equality.

### Proptest: action contract bijection

Invariant: `validate_action_references(parts, contracts) == Ok(())` iff unique `Do.action` IDs equal unique `ActionContract.id` values.
Strategy: bounded `0..=8` Do nodes, action IDs `0..=32`, contracts with duplicates eliminated unless testing duplicate-contract rejection policy separately.
Anti-invariant: remove the first referenced contract to force `ActionContractMissing`; add one unreferenced contract to force `ActionContractOrphan`.

### Proptest: determinism and no mutation

Invariant: Running any public validation function twice returns equal exact `Result` values and leaves borrowed inputs equal to snapshots.
Strategy: generated valid and invalid parts/contracts for symbol/resource/action cases.
Anti-invariant: no mutation is permitted; any inequality is failure.

## 5. Fuzz Targets

### Fuzz target: artifact decode to `WorkflowParts` plus admission

Input type: bytes for existing `.vbir`/postcard artifact decoder.
Risk: panic, OOM, unchecked index/cast, accidental invalid admission, generic string-only error.
Corpus seeds: minimal valid artifact; all three valid symbol carriers; accessor symbol equals count; constant symbol with zero symbols; build-object field equals count; `max_steps` hard-limit + 1; actual nodes 2 with `max_steps = 1`; expression max stack 3 with `max_expr_stack = 2`.
Oracle: decoder returns a typed decode error, or decoded `WorkflowParts` passed to `CompiledWorkflow::try_from_parts` returns exact `Ok(CompiledWorkflow)` or exact `WorkflowError`; no panic/abort/OOM.

### Fuzz target: action contract verifier boundary

Input type: arbitrary bounded struct of `WorkflowParts` action nodes plus `ActionContract` IDs.
Risk: set-equivalence bug, duplicate bug, ordering nondeterminism, panic on empty/large slices.
Corpus seeds: no Do/no contracts; one Do matching contract; one Do/no contracts; no Do/one contract; two Do nodes sharing one action; Do(7)+contract(9).
Oracle: exact `Ok(())`, `ActionContractMissing`, `ActionContractOrphan`, or earlier exact non-action `ValidationError`; no panic/OOM.

Fuzz acceptance is mandatory. If fuzz tooling is unavailable, bead acceptance is **BLOCKED**, not downgraded.

## 6. Kani Harnesses

### Kani: symbol boundary completeness
Property: for `symbols_count <= 8` and symbol IDs `<= 9`, each carrier is accepted iff `symbol < symbols_count`; otherwise exact symbol error is returned.
Bound: three carriers × `symbols_count 0..=8` × `symbol 0..=9`.

### Kani: resource off-by-one correctness
Property: equality at declared/hard limit is accepted; `declared > hard_limit` and `actual > declared` fail with exact resource member.
Bound: six resource members, counts `0..=8`, hard limits modelled as harness constants.

### Kani: action-contract bijection
Property: up to 4 Do nodes and 4 contracts succeed iff unique ID sets are equal; missing is reported before orphan.
Bound: action IDs `0..=7`, nodes/contracts `0..=4`.

### Kani: scan index bounds
Property: validation scans never index out of bounds for vectors length `0..=8` and never panic.
Bound: nodes/accessors/constants/expressions/contracts length `0..=8`.

Kani execution is mandatory for this bead when harnesses are added. If Kani cannot run in CI, acceptance is **BLOCKED** until a bead dependency records the infrastructure gap.

## 7. Mutation Testing Checkpoints

Minimum: `cargo-mutants --minimum-test-timeout 60 --package vb_core --package vb_validate` scoped to touched files must report **>= 90% killed**.

Critical mutants and required killing tests:

- Change `symbol >= symbols_count` to `symbol > symbols_count`: killed by the three `*_equals_symbols_count` symbol tests.
- Remove accessor symbol scan: killed by `validate_symbol_references_returns_symbol_out_of_bounds_when_accessor_field_equals_symbols_count`.
- Remove constant symbol scan: killed by constant out-of-bounds test.
- Remove build-object field scan: killed by build-object out-of-bounds test.
- Allow `symbols_count == 0`: killed by the three zero-symbol tests.
- Swap symbol source context values: killed by exact `SymbolReferenceSource` assertions.
- Hollow success into `Ok(Default::default())`: killed by `core_admission_preserves_workflow_counts_when_reference_validation_passes` exact count assertions.
- Change `declared > hard_limit` to `declared >= hard_limit`: killed by hard-limit equality boundary tests.
- Change `actual > declared` to `actual >= declared`: killed by actual-equals-declared boundary tests.
- Swap resource string `max_steps` with `max_slots`: killed by exact resource-string tests in verifier and core.
- Remove expression stack resource check: killed by `*_max_expr_stack_exceeds_*` tests.
- Remove Gate 12 from `validate_with_contracts`: killed by `validate_with_contracts_returns_action_contract_missing_after_non_action_gates_pass`.
- Add Gate 12 to `validate`: killed by `shared_validate_returns_unit_for_do_node_without_contracts_when_non_action_gates_pass`.
- Report orphan before missing: killed by `validate_action_references_reports_missing_before_orphan_when_both_exist`.
- Change missing `node_index` to `0`: killed by `validate_action_references_preserves_action_id_and_node_index_fields_for_missing_contract`.
- Swap missing `action_id` and `node_index`: killed by exact field assertion for `action_id == 7`, `node_index == 3`.
- Require duplicate contracts for duplicate Do references: killed by duplicate Do success test.
- Collapse typed errors to generic strings: killed by exact enum pattern and diagnostic-code tests.
- Mutate borrowed inputs: killed by atomicity snapshot tests.

Surviving critical mutants block acceptance. Non-critical survivors may be accepted only if total kill rate remains >=90% and each survivor is documented with a rationale in implementation evidence.

## 8. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|---|---|---|---|
| valid symbol carriers | symbols 0/1/2, count 3 | `Ok(())` from `validate_symbol_references` | unit |
| accessor equals count | accessor symbol 1, count 1 | direct helper: `Err(WorkflowError::SymbolOutOfBounds { symbol: SymbolId::new(1) })`; pipeline: `Err(SymbolReferenceOutOfRange { symbol: 1, AccessorField, 0 })` | unit/integration |
| constant equals count | constant symbol 2, count 2 | direct helper: `Err(WorkflowError::SymbolOutOfBounds { symbol: SymbolId::new(2) })`; pipeline: `Err(SymbolReferenceOutOfRange { symbol: 2, ConstantPool, 0 })` | unit/integration |
| build field equals count | field symbol 2, count 2 | direct helper: `Err(WorkflowError::SymbolOutOfBounds { symbol: SymbolId::new(2) })`; pipeline: `Err(SymbolReferenceOutOfRange { symbol: 2, BuildObjectField, 0 })` | unit/integration |
| zero symbols accessor | count 0, accessor symbol 0 | direct helper/core exact `WorkflowError::SymbolOutOfBounds`; pipeline exact symbol verifier error | unit/integration |
| zero symbols constant | count 0, constant symbol 0 | direct helper/core exact `WorkflowError::SymbolOutOfBounds`; pipeline exact symbol verifier error | unit/integration |
| zero symbols build object | count 0, field symbol 0 | direct helper/core exact `WorkflowError::SymbolOutOfBounds`; pipeline exact symbol verifier error | unit/integration |
| core symbol parity | invalid symbol carrier | `Err(WorkflowError::SymbolOutOfBounds { symbol })` | integration |
| resources equal declarations | actual == declared for all members | `Ok(())` | unit |
| declared hard limit + 1 | each resource member | direct helper: `Err(WorkflowError::ResourceContractTooLarge { resource })`; pipeline: `Err(ValidationError::ResourceContractTooLarge { resource, declared, hard_limit })` | unit/integration |
| actual exceeds declared | each resource member | direct helper: `Err(WorkflowError::ResourceContractExceeded { resource })`; pipeline: `Err(ValidationError::ResourceContractExceeded { resource, actual, declared })` | unit/integration |
| core resource too large | each resource member | `Err(WorkflowError::ResourceContractTooLarge { resource })` | integration |
| core resource exceeded | each resource member | `Err(WorkflowError::ResourceContractExceeded { resource })` | integration |
| action match | Do(7), contract(7) | `Ok(())` | unit/integration |
| duplicate Do actions | Do(7), Do(7), contract(7) | `Ok(())` | unit |
| missing action | Do(7), no contracts | `Err(ActionContractMissing { action_id: 7, node_index })` | unit/integration |
| orphan action | no Do, contract(9) | `Err(ActionContractOrphan { action_id: 9 })` | unit/integration |
| missing and orphan | Do(7), contract(9) | `Err(ActionContractMissing { action_id: 7, node_index: 0 })` | unit |
| default validate with Do | Do(7), no contract arg | `Ok(())` if non-action gates pass | integration |
| validate_with_contracts missing | Do(7), empty contracts | exact missing action error | integration |
| diagnostics | each reference error | exact E050D/E050E/E050F/E0509/E050A | unit |
| CLI invalid symbol | `.vbir` invalid accessor | exit 1 + E050D + exact fields | E2E |
| CLI invalid resource | `.vbir` max_steps exceeded | exit 1 + E050F + exact fields | E2E |
| source policy | implementation diff | no forbidden constructs/I/O | static |

## Static, Holzmann, and Resource Gates

These checks are concrete acceptance gates, not promises:

1. `moon ci` must pass.
2. `cargo test -p vb_core -p vb_validate vb_qi37_7_3 -- --nocapture` must pass targeted tests.
3. `cargo test -p velvet-ballastics cli_validate_reports_e050d_for_invalid_symbol_reference_fixture cli_run_compiled_reports_resource_error_for_invalid_resource_fixture -- --nocapture` must pass E2E tests.
4. `cargo mutants --package vb_core --package vb_validate --minimum-test-timeout 60` must report `>= 90%` kill rate.
5. `cargo fuzz run artifact_decode_admission -- -max_total_time=60` must complete with no crash.
6. `cargo fuzz run action_contract_verifier -- -max_total_time=60` must complete with no crash.
7. `cargo kani -p vb_core` and `cargo kani -p vb_validate` must pass the harnesses added for this bead.
8. Static forbidden-construct scan must be executed exactly from the workspace root and must produce `file:line:match` evidence for every violation:
   - Command: `rg --line-number --with-filename --glob 'crates/vb_core/src/**' --glob 'crates/vb_validate/src/**' --glob '!**/tests/**' --glob '!**/test_support/**' 'unsafe|\.unwrap\(|\.expect\(|panic!|todo!|unimplemented!|dbg!|\[[^\]]+\]| as (u8|u16|u32|u64|usize|i8|i16|i32|i64|isize)'`
   - Failure rule: exit code `0` with any matching line is a hard failure unless that exact `file:line` is pre-existing outside the bead diff and is listed in implementation evidence as untouched debt. Exit code `1` means no matches and passes. Exit code `>1` is a tool failure and blocks acceptance.
   - Target paths: only runtime-core source under `crates/vb_core/src/**` and `crates/vb_validate/src/**`, excluding test modules/support.
9. Static runtime I/O/config dependency scan must be executed exactly from the workspace root and must produce `file:line:match` evidence for every violation:
   - Command: `rg --line-number --with-filename --glob 'crates/vb_core/src/**' --glob 'crates/vb_validate/src/**' --glob '!**/tests/**' --glob '!**/test_support/**' 'std::fs|tokio::fs|std::net|tokio::net|reqwest|hyper|ureq|serde_yaml|serde_json|yaml|http|https|File::open|read_to_string|TcpStream|UdpSocket'`
   - Failure rule: any match in touched validation/admission code is a hard failure. Any pre-existing untouched match must be cited as `file:line` evidence and must not be in the bead diff. Exit code `1` is pass. Exit code `>1` blocks acceptance.
10. Static panic/resource evidence must include `RUST_BACKTRACE=1 cargo test -p vb_core -p vb_validate vb_qi37_7_3 -- --nocapture` completing with exit code `0`, no output line containing `panicked at`, no output line containing `thread '.*' panicked`, and no tempfile or fixture files left outside `tests/fixtures/vb_qi37_7_3`.

## Open Questions

None. The review-required choices are pinned in this plan.
