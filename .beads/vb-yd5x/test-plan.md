# Test Plan: vb-yd5x — validate/compile shared validated IR proof

## Summary

This State 4 retry repairs every rejection in `.beads/vb-yd5x/test-plan-review.md`.

- Public contract signatures counted from `contract.md:118-125`: 8.
- Required executable density: `8 * 5 = 40` executable tests minimum.
- Planned executable tests/checks: 62 named scenarios/checks, excluding proptest/fuzz/Kani variants.
- Trophy allocation: 19 unit / 33 integration / 3 e2e / 7 static checks.
- Proptest invariants: 8.
- Fuzz targets: 4.
- Kani harnesses: 5.
- Mutation threshold: `cargo-mutants` must kill at least 90% of mutants for changed `vb_compile` seam code and touched validation helper code.
- Canonical final gate: `moon ci`.

Hard rule: no planned assertion may be only `is_ok()` or `is_err()`. Every assertion below names an exact success value, exact public IR fact, exact exit code, exact artifact oracle, or exact typed error variant/payload.

## Latest Rejection Repair Map

| Rejection | Exact repair in this plan |
|---|---|
| Missing direct BDD scenarios for `CompiledWorkflow::try_from_parts` | Behaviors 53-56 and BDD section “Direct core constructor contract”. |
| `DepthLimit` boundary was non-concrete | Behavior 6 now requires `CompileError::DepthLimit { depth: 1, limit: 0 }`. |
| Scalar limit was conditional | Behavior 7 now requires `CompileError::ScalarLimit { actual: 7, limit: 0 }` for first key `version`. |
| Mapping limit was conditional | Behavior 8 now requires `CompileError::MappingLimit { actual: 4, limit: 0 }`. |
| Conditional `EntryOutOfBounds` lower test | Behavior 19 is removed; lower empty nodes has one exact oracle: `WorkflowError::EmptyNodes`. Direct `try_from_parts` covers `EntryOutOfBounds { entry: StepIdx::new(1) }`. |
| Deferred duplicate contract behavior | Behavior 32 now requires duplicate safe contracts for action 7 to be accepted: `Ok(CompiledWorkflow)` and `validate_with_contracts == Ok(())`. |
| CLI fixture/decode/digest vague | E2E section fixes exact source fixture path, exact blessed artifact path, and exact decode API `postcard::from_bytes::<vb_core::WorkflowParts>`. |
| Runtime allowlist vague | Static section names `crates/velvet_ballastics/tests/fixtures/static/runtime_import_allowlist.txt` with required empty contents for this bead. |
| Safety allowlist vague | Static section names `crates/velvet_ballastics/tests/fixtures/static/vb_yd5x_safety_allowlist.txt` with explicit allowed regex lines only. |

## 1. Behavior Inventory

1. `compile_workflow` returns a trusted `CompiledWorkflow` when source is the minimum valid YAML fixture.
2. `compile_workflow` returns `CompileError::EmptySource` when source is empty.
3. `compile_workflow` returns `CompileError::SourceTooLarge { actual: 1_048_577, limit: 1_048_576 }` when source exceeds default bytes by one.
4. `compile_workflow` returns `CompileError::Parse(_)` when YAML syntax is malformed but non-empty UTF-8.
5. `YamlCompiler::compile` returns a trusted `CompiledWorkflow` when limits exactly equal the minimum fixture requirements.
6. `YamlCompiler::compile` returns `CompileError::DepthLimit { depth: 1, limit: 0 }` when `max_depth` is zero for `MINIMAL_YAML`.
7. `YamlCompiler::compile` returns `CompileError::ScalarLimit { actual: 7, limit: 0 }` when `max_scalar_bytes` is zero for `MINIMAL_YAML`; the first offending scalar is mapping key `version`.
8. `YamlCompiler::compile` returns `CompileError::MappingLimit { actual: 4, limit: 0 }` when `max_mapping_entries` is zero for `MINIMAL_YAML`.
9. `YamlCompiler::compile` returns `CompileError::SequenceLimit { actual: 1, limit: 0 }` when `max_sequence_len` is zero for one-step `MINIMAL_YAML`.
10. `YamlCompiler::compile` returns `CompileError::SourceTooLarge { actual: MINIMAL_YAML.len(), limit: MINIMAL_YAML.len() - 1 }` when source limit is one below fixture length.
11. `YamlCompiler::compile` returns `CompileError::Validation(ValidationError::SlotReferenceOutOfRange { slot: 1, slot_count: 1, context: "Do.input" })` when compile-produced parts reference a missing input slot.
12. `YamlCompiler::compile` returns `CompileError::Workflow(WorkflowError::EmptyBranchTable)` when shared validation succeeds but core construction rejects an empty branch table.
13. `lower_steps_to_ir` returns a trusted `CompiledWorkflow` when supplied minimum valid vectors contain one `Finish` node and one constant.
14. `lower_steps_to_ir` returns `CompileError::Validation(ValidationError::SlotReferenceOutOfRange { slot: 1, slot_count: 1, context: "Do.input" })` when a `Do` node input equals `slot_count`.
15. `lower_steps_to_ir` returns `CompileError::Validation(ValidationError::LoopBodyStepOutOfRange { step: 2, node_count: 1, source_node: 0, label: "repeat.body" })` when loop body target exceeds node count.
16. `lower_steps_to_ir` returns `CompileError::Validation(ValidationError::AccessorSlotOutOfRange { accessor_index: 0, slot: 1, slot_count: 1 })` when accessor root slot equals `slot_count`.
17. `lower_steps_to_ir` returns `CompileError::Validation(ValidationError::ExpressionStackMismatch { expr_index: 0, declared: 0, computed: 1 })` when expression metadata understates stack depth.
18. `lower_steps_to_ir` returns `CompileError::Workflow(WorkflowError::EmptyNodes)` when vectors contain zero nodes.
19. `lower_steps_to_ir` returns `CompileError::Workflow(WorkflowError::NodeIdMismatch { expected: StepIdx::new(0), actual: StepIdx::new(1) })` when first node id is not table position zero.
20. `validate_ir` returns a trusted `CompiledWorkflow` when supplied minimum valid `WorkflowParts` from `compile_workflow(MINIMAL_YAML).to_parts()`.
21. `validate_ir` returns `CompileError::Validation(ValidationError::SlotReferenceOutOfRange { slot: 1, slot_count: 1, context: "Do.input" })` before core acceptance for shared-invalid/core-constructible parts.
22. `validate_ir` returns `CompileError::Workflow(WorkflowError::EmptyNodes)` for shared-valid/core-invalid empty-node parts.
23. `validate_ir` returns `CompileError::Workflow(WorkflowError::NodeIdMismatch { expected: StepIdx::new(0), actual: StepIdx::new(1) })` for shared-valid parts with wrong node id.
24. `validate_ir` returns `CompileError::Workflow(WorkflowError::ConstOutOfBounds { constant: ConstIdx::new(1) })` when `Finish` references constant 1 and constants length is 1.
25. `validate_ir` returns `CompileError::Workflow(WorkflowError::SlotOutOfBounds { slot: SlotIdx::new(1) })` when core slot validation rejects a slot after shared validation passes.
26. `compile_workflow_with_contracts` returns a trusted `CompiledWorkflow` when every `Do` node action id has exactly one matching safe pure `ActionContract`.
27. `compile_workflow_with_contracts` returns `CompileError::Validation(ValidationError::ActionContractMissing { action_id: 7, node_index: 0 })` when action id 7 has no contract.
28. `compile_workflow_with_contracts` returns `CompileError::Validation(ValidationError::ActionContractOrphan { action_id: 99 })` when contract 99 has no matching `Do` node.
29. `compile_workflow_with_contracts` returns `ActionContractMissing { action_id: 7, node_index: 0 }` when the contract slice is empty and workflow contains action 7.
30. `compile_workflow_with_contracts` returns `ActionContractOrphan { action_id: 99 }` when workflow contains no `Do` nodes and the contract slice contains one orphan.
31. `compile_workflow_with_contracts` accepts two duplicate safe pure contracts for action id 7 because current `check_idempotency_gates` only rejects side-effect/retry/idempotency violations and gate 12 compares action/contract set membership.
32. `vb_validate::shared::validate` returns `Ok(())` for otherwise-valid plain parts containing `Do { action: ActionId::new(7) }` without contracts; plain mode must not claim gate 12.
33. `vb_validate::shared::validate_with_contracts` returns `ActionContractMissing { action_id: 7, node_index: 0 }` for the same parts with no contract.
34. `vb_validate::shared::validate_with_contracts` returns `ActionContractOrphan { action_id: 99 }` when contract set contains 99 and parts use no action 99.
35. `CompileError::from(ValidationError::ExpressionStackExceeded { declared: 65, limit: 64 })` preserves the exact validation variant and payload.
36. `CompileError::from(ValidationError::ExpressionStackMismatch { expr_index: 0, declared: 0, computed: 1 })` preserves the exact variant and payload.
37. `CompileError::from(ValidationError::AccessorSlotOutOfRange { accessor_index: 0, slot: 1, slot_count: 1 })` preserves the exact variant and payload.
38. `CompileError::from(ValidationError::AccessorPathInvalid { accessor_index: 0, segment_index: 0 })` preserves the exact variant and payload.
39. `CompileError::from(ValidationError::SlotReferenceOutOfRange { slot: 1, slot_count: 1, context: "Do.input" })` preserves the exact variant and payload.
40. `CompileError::from(ValidationError::LoopBodyStepOutOfRange { step: 2, node_count: 1, source_node: 0, label: "repeat.body" })` preserves the exact variant and payload.
41. `CompileError::from(ValidationError::SlotDependencyCycle { slot: 0, chain: "0->0" })` preserves the exact variant and payload.
42. `CompileError::from(ValidationError::NodeKindConstraintViolation { node_index: 0, detail: "empty branch table" })` preserves the exact variant and payload.
43. `CompileError::from(ValidationError::ActionContractMissing { action_id: 7, node_index: 0 })` preserves the exact variant and payload.
44. `CompileError::from(ValidationError::ActionContractOrphan { action_id: 99 })` preserves the exact variant and payload.
45. `CompileError::from(ValidationError::SlotTypeInconsistency { slot: 0 })` preserves the exact variant and payload.
46. `CompileError::from(ValidationError::NonDeterministicPath { from_node: 0, to_node: 1 })` preserves the exact variant and payload.
47. `CompileError::from(WorkflowError::EmptyBranchTable)` preserves `CompileError::Workflow(WorkflowError::EmptyBranchTable)`.
48. `CompileError::from(WorkflowError::ResourceContractExceeded { resource: "nodes" })` preserves the exact workflow variant and payload.
49. `CompileError::from(WorkflowError::ResourceContractTooLarge { resource: "nodes" })` preserves the exact workflow variant and payload.
50. Each isolated validation/core seam failure returns `CompileErrors` length exactly `1`.
51. Successful outputs from `compile_workflow`, `YamlCompiler::compile`, `lower_steps_to_ir`, and `validate_ir` each have independent tests proving `to_parts()` passes `vb_validate::shared::validate` with `Ok(())`.
52. CLI compile emits a postcard artifact from `crates/velvet_ballastics/tests/fixtures/compile/minimal_valid.velvet.yaml` whose bytes exactly equal `crates/velvet_ballastics/tests/fixtures/compile/minimal_valid.postcard` and whose decoded parts pass shared validation.
53. `CompiledWorkflow::try_from_parts` returns a trusted workflow when supplied direct minimum valid core `WorkflowParts`.
54. `CompiledWorkflow::try_from_parts` returns `WorkflowError::EmptyNodes` when supplied parts contain zero nodes.
55. `CompiledWorkflow::try_from_parts` returns `WorkflowError::EntryOutOfBounds { entry: StepIdx::new(1) }` when entry is one and node table length is one.
56. `CompiledWorkflow::try_from_parts` returns `WorkflowError::NodeIdMismatch { expected: StepIdx::new(0), actual: StepIdx::new(1) }` when first node id is one.
57. CLI compile cleans up all temporary files and returns exit code `0` for valid input.
58. CLI compile returns exit code `2` (`CliExitCode::CompileFailed`) and a typed diagnostic containing `validation gate failure` plus `SLOT_REFERENCE_OUT_OF_RANGE` for shared-gate-invalid fixture.
59. Static runtime boundary checks find zero unallowlisted YAML/JSON/HTTP/compile-validation imports in hot runtime crates using the named empty allowlist.
60. Static module graph checks prove tests hit `crates/vb_compile/src/lib.rs`, not inactive split files `slot.rs`, `types.rs`, `api_validation.rs`, or `compile.rs`.
61. Static safety checks find zero forbidden constructs in changed production/test files using the named allowlist.
62. `moon ci` passes as final implementation completion gate.

## 2. Trophy Allocation

| Behaviors | Layer | Tool | Rationale |
|---|---|---|---|
| 1-12 | Integration | `cargo test -p vb_compile` / `cargo nextest run -p vb_compile` | YAML/compiler boundary crosses parser, AST validation, shared validation, and core construction. |
| 13-25 | Integration | `cargo test -p vb_compile` | Public lowering and `validate_ir` seam must use real `WorkflowParts`, real validator, and real core constructor. |
| 26-34 | Integration + Unit | `cargo test -p vb_compile -p vb_validate` | Contract-aware path needs full compile path; plain/contract validator distinction is pure validator behavior. |
| 35-50 | Unit | `#[test]` exact enum matches | Conversion/cardinality behavior is local and deterministic. |
| 51 | Integration | Four separate tests or named `rstest` cases | One assertion target per API; no test-body loops. |
| 52, 57-58 | E2E | `assert_cmd` invoking `velvet-ballastics` binary | User-visible command/artifact behavior. |
| 53-56 | Unit/Integration | `cargo test -p vb_core` or cross-crate integration test | Direct public core constructor contract; exact `WorkflowError` variants. |
| 59-62 | Static | `rg`, source assertions, `cargo metadata`, `moon ci` | Boundary/safety/final-gate properties are best enforced as executable static gates. |

## 3. BDD Scenarios

### Shared fixtures every test must define explicitly

- `MINIMAL_YAML`: `b"version: velvet-ballastics/v1\nname: minimal\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n"`.
- Expected `MINIMAL_YAML` public IR facts: `workflow.name() == "minimal"`, `parts.entry == StepIdx::new(0)`, `parts.nodes.len() == 1`, `parts.expressions.len() == 0`, `parts.accessors.len() == 0`, `parts.constants.len() == 1`, `parts.slot_count == 0`, `parts.symbols_count == 0`, and `vb_validate::shared::validate(&parts) == Ok(())`.
- `MINIMAL_DO_YAML`: valid schema fixture with one `do` primitive compiling to `ActionId::new(7)` and one reachable finish.
- `VALID_ACTION_CONTRACT_7`: `ActionContract { id: ActionId::new(7), side_effect: SideEffect::None, retry_safety: RetrySafety::Safe, idempotency: Idempotency::DeterministicPure }`.
- Duplicate safe contract fixture: `[VALID_ACTION_CONTRACT_7, VALID_ACTION_CONTRACT_7]`.
- No test body may loop across APIs. Use separate named tests or named `rstest` cases.

### Compile facade and YAML compiler

- `fn compile_workflow_returns_minimal_workflow_when_source_is_minimum_valid_yaml()` — Given `MINIMAL_YAML`; When `compile_workflow(MINIMAL_YAML)`; Then exact `MINIMAL_YAML` public IR facts and `validate(&to_parts()) == Ok(())`.
- `fn compile_workflow_returns_empty_source_when_source_is_empty()` — Given `b""`; When compile; Then `Err(CompileErrors(vec![CompileError::EmptySource]))`.
- `fn compile_workflow_returns_source_too_large_when_source_exceeds_default_limit_by_one()` — Given UTF-8 bytes len `1_048_577`; Then `SourceTooLarge { actual: 1_048_577, limit: 1_048_576 }`.
- `fn yaml_compiler_compile_returns_parse_error_when_yaml_syntax_is_malformed()` — Given `b"version: [unterminated"`; Then `CompileErrors.0.len() == 1` and sole variant is `CompileError::Parse(_)`.
- `fn yaml_compiler_compile_returns_minimal_workflow_when_limits_equal_fixture_requirements()` — Given exact sufficient limits; Then exact `MINIMAL_YAML` public IR facts.
- `fn yaml_compiler_compile_returns_depth_limit_when_max_depth_is_zero()` — Given `YamlLimits { max_depth: 0, ..Default::default() }`; Then `Err(CompileErrors(vec![CompileError::DepthLimit { depth: 1, limit: 0 }]))`.
- `fn yaml_compiler_compile_returns_scalar_limit_when_max_scalar_bytes_is_zero()` — Given `YamlLimits { max_scalar_bytes: 0, ..Default::default() }`; Then `Err(CompileErrors(vec![CompileError::ScalarLimit { actual: 7, limit: 0 }]))`.
- `fn yaml_compiler_compile_returns_mapping_limit_when_max_mapping_entries_is_zero()` — Given `YamlLimits { max_mapping_entries: 0, ..Default::default() }`; Then `Err(CompileErrors(vec![CompileError::MappingLimit { actual: 4, limit: 0 }]))`.
- `fn yaml_compiler_compile_returns_sequence_limit_when_max_sequence_len_is_zero()` — Given `YamlLimits { max_sequence_len: 0, ..Default::default() }`; Then `Err(CompileErrors(vec![CompileError::SequenceLimit { actual: 1, limit: 0 }]))`.
- `fn yaml_compiler_compile_returns_source_too_large_when_limit_is_one_below_fixture_len()` — Given `max_source_bytes = MINIMAL_YAML.len() - 1`; Then `SourceTooLarge { actual: MINIMAL_YAML.len(), limit: MINIMAL_YAML.len() - 1 }`.
- `fn yaml_compiler_compile_returns_slot_reference_out_of_range_when_generated_parts_reference_missing_slot()` — Given public compile/lowering fixture with `Do.input == SlotIdx::new(1)` and `slot_count == 1`; Then exact `CompileError::Validation(ValidationError::SlotReferenceOutOfRange { slot: 1, slot_count: 1, context: "Do.input" })`.
- `fn yaml_compiler_compile_returns_empty_branch_table_when_shared_validation_passes_but_core_rejects_branch_table()` — Given parts/source with `CompiledNodeKind::ChooseSlot { branches: [], otherwise: None }`; Then exact `CompileError::Workflow(WorkflowError::EmptyBranchTable)`.

### Lowering and `validate_ir` seams

- `fn lower_steps_to_ir_returns_minimal_workflow_when_vectors_are_minimum_valid()` — Given one `Finish { result: ConstIdx::new(0) }` node and one `ConstValue::I64(0)`; Then exact counts, entry `0`, and shared validation `Ok(())`.
- `fn lower_steps_to_ir_returns_slot_reference_out_of_range_when_do_input_exceeds_slot_count()` — Given `Do.input == SlotIdx::new(1)` and `slot_count == 1`; Then exact `SlotReferenceOutOfRange { slot: 1, slot_count: 1, context: "Do.input" }`.
- `fn lower_steps_to_ir_returns_loop_body_step_out_of_range_when_repeat_body_exceeds_nodes()` — Then exact `LoopBodyStepOutOfRange { step: 2, node_count: 1, source_node: 0, label: "repeat.body" }`.
- `fn lower_steps_to_ir_returns_accessor_slot_out_of_range_when_accessor_root_exceeds_slot_count()` — Then exact `AccessorSlotOutOfRange { accessor_index: 0, slot: 1, slot_count: 1 }`.
- `fn lower_steps_to_ir_returns_expression_stack_mismatch_when_declared_stack_is_too_small()` — Then exact `ExpressionStackMismatch { expr_index: 0, declared: 0, computed: 1 }`.
- `fn lower_steps_to_ir_returns_empty_nodes_when_node_vector_is_empty()` — Then exact `CompileError::Workflow(WorkflowError::EmptyNodes)`.
- `fn lower_steps_to_ir_returns_node_id_mismatch_when_first_node_id_is_one()` — Then exact `CompileError::Workflow(WorkflowError::NodeIdMismatch { expected: StepIdx::new(0), actual: StepIdx::new(1) })`.
- `fn validate_ir_returns_minimal_workflow_when_parts_are_valid()` — Given `compile_workflow(MINIMAL_YAML).to_parts()`; Then returned `to_parts()` has exact same public fields and shared validation `Ok(())`.
- `fn validate_ir_returns_slot_reference_out_of_range_before_core_acceptance_when_do_input_exceeds_slot_count()` — Then exact `SlotReferenceOutOfRange { slot: 1, slot_count: 1, context: "Do.input" }`.
- `fn validate_ir_returns_empty_nodes_when_parts_have_no_nodes()` — Then exact `WorkflowError::EmptyNodes`.
- `fn validate_ir_returns_node_id_mismatch_when_node_id_does_not_match_position()` — Then exact `NodeIdMismatch { expected: StepIdx::new(0), actual: StepIdx::new(1) }`.
- `fn validate_ir_returns_const_out_of_bounds_when_finish_references_missing_const()` — Then exact `ConstOutOfBounds { constant: ConstIdx::new(1) }`.
- `fn validate_ir_returns_slot_out_of_bounds_when_core_slot_reference_exceeds_slot_count()` — Then exact `SlotOutOfBounds { slot: SlotIdx::new(1) }`.

### Contract-aware compile and gate 12 mode

- `fn compile_workflow_with_contracts_returns_workflow_when_action_contract_matches()` — Given `MINIMAL_DO_YAML` and `[VALID_ACTION_CONTRACT_7]`; Then workflow parts are shared-valid and `validate_with_contracts(&parts, &contracts) == Ok(())`.
- `fn compile_workflow_with_contracts_returns_action_contract_missing_when_action_seven_has_no_contract()` — Then exact `ActionContractMissing { action_id: 7, node_index: 0 }`.
- `fn compile_workflow_with_contracts_returns_action_contract_orphan_when_contract_ninety_nine_has_no_action()` — Then exact `ActionContractOrphan { action_id: 99 }`.
- `fn compile_workflow_with_contracts_returns_action_contract_missing_when_contract_slice_is_empty()` — Then exact missing variant above.
- `fn compile_workflow_with_contracts_returns_action_contract_orphan_when_workflow_has_no_actions()` — Then exact orphan variant above.
- `fn compile_workflow_with_contracts_accepts_duplicate_safe_pure_contracts_for_same_action_id()` — Given duplicate safe pure contracts for action 7; Then `Ok(CompiledWorkflow)` with exact action workflow IR facts and `validate_with_contracts(&parts, &contracts) == Ok(())`.
- `fn shared_validate_returns_ok_for_action_without_contract_when_plain_mode_is_used()` — Then `vb_validate::shared::validate(&parts) == Ok(())` exactly.
- `fn validate_with_contracts_returns_action_contract_missing_when_action_has_no_contract()` — Then exact missing variant above.
- `fn validate_with_contracts_returns_action_contract_orphan_when_contract_has_no_action()` — Then exact orphan variant above.

### Error preservation unit tests

Each listed test constructs the exact variant and asserts conversion/result payload equality via pattern matching, not display strings:

`validation_error_preserves_expression_stack_exceeded`, `validation_error_preserves_expression_stack_mismatch`, `validation_error_preserves_accessor_slot_out_of_range`, `validation_error_preserves_accessor_path_invalid`, `validation_error_preserves_slot_reference_out_of_range`, `validation_error_preserves_loop_body_step_out_of_range`, `validation_error_preserves_slot_dependency_cycle`, `validation_error_preserves_node_kind_constraint_violation`, `validation_error_preserves_action_contract_missing`, `validation_error_preserves_action_contract_orphan`, `validation_error_preserves_slot_type_inconsistency`, `validation_error_preserves_non_deterministic_path`, `workflow_error_preserves_empty_branch_table`, `workflow_error_preserves_resource_contract_exceeded`, `workflow_error_preserves_resource_contract_too_large`, `compile_errors_contains_exactly_one_error_for_isolated_validation_and_core_failures`.

### Direct core constructor contract: `CompiledWorkflow::try_from_parts`

- `fn try_from_parts_returns_workflow_when_parts_are_minimum_valid()` — Given direct `WorkflowParts` with name `"core_minimal"`, digest `[0; 32]`, one `Finish { result: ConstIdx::new(0) }` node at `StepIdx::new(0)`, constants `[ConstValue::I64(0)]`, `entry: StepIdx::new(0)`, `slot_count: 0`, `symbols_count: 0`, `ResourceContract::DEFAULT`, and step name `"done"`; When `CompiledWorkflow::try_from_parts(parts)`; Then workflow name is `"core_minimal"`, entry is `StepIdx::new(0)`, node_count is `1`, constant `ConstIdx::new(0)` equals `Some(&ConstValue::I64(0))`, and `workflow.to_parts().nodes.len() == 1`.
- `fn try_from_parts_returns_empty_nodes_when_node_table_is_empty()` — Given same fields but `nodes: Box::new([])` and `entry: StepIdx::new(0)`; Then `Err(WorkflowError::EmptyNodes)`.
- `fn try_from_parts_returns_entry_out_of_bounds_when_entry_is_one_and_one_node_exists()` — Given one valid node at index 0 but `entry: StepIdx::new(1)`; Then `Err(WorkflowError::EntryOutOfBounds { entry: StepIdx::new(1) })`.
- `fn try_from_parts_returns_node_id_mismatch_when_first_node_id_is_one()` — Given one node stored at index 0 with `id: StepIdx::new(1)` and `entry: StepIdx::new(0)`; Then `Err(WorkflowError::NodeIdMismatch { expected: StepIdx::new(0), actual: StepIdx::new(1) })`.

### CLI/E2E acceptance

- Source fixture path is exact: `crates/velvet_ballastics/tests/fixtures/compile/minimal_valid.velvet.yaml`.
- Blessed artifact fixture path is exact: `crates/velvet_ballastics/tests/fixtures/compile/minimal_valid.postcard`.
- Invalid fixture path is exact: `crates/velvet_ballastics/tests/fixtures/compile/shared_slot_reference_out_of_range.velvet.yaml`.
- CLI command shape is exact: `velvet-ballastics compile <fixture> --emit postcard --out <tempdir>/minimal.postcard`.
- Decode API is exact: `postcard::from_bytes::<vb_core::WorkflowParts>(&artifact_bytes)`.
- Success oracle: exit code `0`, stdout contains `compiled postcard written to <tempdir>/minimal.postcard`, output bytes equal the blessed fixture byte-for-byte, decoded `WorkflowParts` has exact `MINIMAL_YAML` public IR facts, and `vb_validate::shared::validate(&decoded_parts) == Ok(())`.
- Cleanup oracle: after test tempdir drop, `std::path::Path::exists(tempdir_path) == false`; before drop, only `minimal.postcard` may exist in the tempdir.
- Failure oracle: invalid fixture exits with code `2`, stderr contains `compile error: validation gate failure`, stderr contains `SLOT_REFERENCE_OUT_OF_RANGE`, and no output artifact exists.

### Executable static checks

- `fn static_runtime_import_scan_finds_no_yaml_json_http_or_compile_validation_in_hot_loop()` executes: `rg -n "saphyr|serde_yaml|serde_json|reqwest|hyper|http::|vb_validate::shared|compile_workflow|YamlCompiler" crates/vb_runtime crates/vb_engine crates/vb_core/src/runtime crates/vb_core/src/run.rs crates/vb_core/src/storage.rs`. Allowlist path: `crates/velvet_ballastics/tests/fixtures/static/runtime_import_allowlist.txt`. Required contents for this bead: empty file. Expected unallowlisted output: zero lines.
- `fn static_vb_compile_module_graph_targets_live_lib_facade()` checks `crates/vb_compile/src/lib.rs` declares exactly these live module names for this seam: `ast`, `control_flow`, `expression`, `expression_bytecode`, `references`, `schema`, `strict_yaml`, `type_taint`; and contains no `mod slot;`, `mod types;`, `mod api_validation;`, or `mod compile;`.
- `fn static_safety_scan_finds_no_forbidden_constructs_in_bead_diff()` scans changed production and test files for `unsafe`, `.unwrap(`, `.expect(`, `panic!`, `todo!`, `unimplemented!`, `dbg!`, unchecked `[` indexing/slicing, unchecked ` as ` casts, and unchecked arithmetic introduced for this bead. Allowlist path: `crates/velvet_ballastics/tests/fixtures/static/vb_yd5x_safety_allowlist.txt`. Required allowed lines: `#![forbid(unsafe_code)]`, `#[allow(clippy::too_many_arguments)]`, `#[allow(clippy::too_many_lines)]`, documentation text containing the forbidden words. Expected unallowlisted output: zero lines.

## 4. Proptest Invariants

### Proptest: `validate_ir(parts)` valid round-trip
Invariant: any bounded valid `WorkflowParts` passing `vb_validate::shared::validate` and `CompiledWorkflow::try_from_parts` returns a workflow whose `to_parts()` equals original public fields. Strategy: `1..=16` nodes with matching ids, valid entry, bounded constants/accessors/expressions, deterministic paths, consistent counts. Anti-invariant: `Do.input >= slot_count` always returns exact `SlotReferenceOutOfRange`.

### Proptest: `lower_steps_to_ir` generated valid vectors
Invariant: valid generated vectors lower to shared-valid `CompiledWorkflow` with preserved counts. Strategy: same bounds as above through public lowering parameters. Anti-invariant: empty node vector returns exact `WorkflowError::EmptyNodes`.

### Proptest: `CompiledWorkflow::try_from_parts` core constructor
Invariant: valid generated parts with matching node ids, in-bounds entry/references, and adequate `ResourceContract` produce a workflow with equal public `to_parts()` fields. Strategy: `1..=16` nodes, entry in `0..nodes.len()`, constants/accessors/expressions within bounds. Anti-invariant: entry exactly `nodes.len()` returns `WorkflowError::EntryOutOfBounds { entry }`; first node id one with table index zero returns exact `NodeIdMismatch`.

### Proptest: gate 9 slot references
Invariant: all generated slot references in nodes/accessors/expressions must be `< slot_count` for success. Strategy: `slot_count in 0..=16`; valid refs `0..slot_count`; invalid refs exactly `slot_count`. Anti-invariant: invalid refs return exact `SlotReferenceOutOfRange` or `AccessorSlotOutOfRange`.

### Proptest: gate 12 action contracts
Invariant: `validate_with_contracts` succeeds iff action-id set equals contract-id set. Strategy: action ids and contract ids over `0..=8`, sizes `0..=8`. Anti-invariant: action minus contracts returns exact `ActionContractMissing`; contracts minus actions returns exact `ActionContractOrphan`.

### Proptest: plain validation excludes gate 12
Invariant: for otherwise-valid parts, `validate(&parts) == Ok(())` regardless of missing action contracts. Strategy: valid parts with `Do` action ids `0..=8`. Anti-invariant: any `ActionContractMissing` or `ActionContractOrphan` from plain validate is a failure.

### Proptest: validation/workflow error conversion
Invariant: every constructible shared-gate `ValidationError` and reachable core `WorkflowError` variant is preserved inside the corresponding `CompileError` wrapper with identical payload. Strategy: enumerate variants and generate small payload values. Anti-invariant: string-only erasure or wrong wrapper fails.

### Proptest: YAML limit arithmetic
Invariant: exact boundary `limit == actual` succeeds for source bytes, `limit == actual - 1` returns the corresponding limit variant, and zero limits return the concrete variants above. Strategy: generated valid YAML fixture sizes around `0`, `1`, fixture length, and `DEFAULT_MAX_SOURCE_BYTES` within memory-safe bounds. Anti-invariant: overflow/underflow while computing boundaries is forbidden.

## 5. Fuzz Targets

### Fuzz Target: `compile_workflow(source: &[u8])`
Input type: arbitrary bytes. Risk: parser crash, limit bypass, invalid YAML accepted as shared-valid, typed error erasure. Corpus seeds: `b""`, whitespace-only, `MINIMAL_YAML`, `MINIMAL_DO_YAML`, malformed YAML, duplicate keys, YAML alias/anchor, invalid UTF-8, deeply nested maps, one-byte-over default limit. Oracle: no crash; success implies exact shared validation `Ok(())`; failure has `CompileErrors.0.len() == 1` and a concrete `CompileError` variant.

### Fuzz Target: `YamlCompiler::compile(source)` with generated `YamlLimits`
Input type: arbitrary bytes plus bounded `YamlLimits`. Risk: arithmetic overflow, off-by-one limit checks, inconsistent facade/instance behavior. Corpus seeds: exact sufficient limits, one-below source limit, zero depth/scalar/mapping/sequence limits. Oracle: exact limit variants named above or success with shared-valid parts.

### Fuzz Target: CLI postcard artifact decode
Input type: arbitrary bytes. Risk: decoder crash, malformed artifact accepted as proof, checksum/length errors hidden. Corpus seeds: `minimal_valid.postcard`, truncated artifact, random bytes, mutated slot reference, corrupted digest/checksum. Oracle: `postcard::from_bytes::<vb_core::WorkflowParts>` either returns a decode error or yields parts explicitly checked with `vb_validate::shared::validate`.

### Fuzz Target: `validate_ir(parts)` arbitrary parts
Input type: arbitrary `WorkflowParts` generated by `arbitrary`/proptest bridge within small bounds. Risk: panics in shared validation or core constructor on hostile references/counts. Corpus seeds: empty parts, one valid finish node, out-of-range slot, out-of-range const, mismatched node id, expression stack mismatch, gate 12 action fixture. Oracle: no crash; result is either trusted workflow with shared-valid parts or exact `CompileError::Validation`/`CompileError::Workflow` variant.

## 6. Kani Harnesses

- `validation-before-core order state machine`: prove no modeled public seam transitions `BuiltParts -> CoreConstructed` unless it first visits `SharedValidated`; bound four states, all boolean outcomes.
- `compile error classification lattice`: prove shared failure maps only to `CompileError::Validation`, core failure after shared success maps only to `CompileError::Workflow`, success maps only to `Ok`; bound all shared/core boolean combinations.
- `try_from_parts core validation lattice`: prove empty nodes, out-of-bounds entry, and node-id mismatch are mutually classified to exact `WorkflowError` variants within modeled bounds; bound nodes `0..=2`, entry `0..=3`, first id `0..=2`.
- `gate 12 mode lattice`: prove plain mode cannot emit `ActionContractMissing`/`ActionContractOrphan`; contract mode emits them exactly when sets differ; bound ids `0..=3`, set size `0..=3`.
- `YAML limit arithmetic`: prove computing `actual`, `limit`, `limit + 1`, and `actual - 1` never overflows/underflows within configured bounds; bound source lengths `0..=1_048_577`, scalar lengths `0..=65_537`.

## 7. Mutation Testing Checkpoints

Minimum threshold: `cargo-mutants` or repository-approved equivalent must report >=90% killed mutants for changed `vb_compile` seam code and touched validation helper code. Survivors in validation order, typed errors, gate 12, static boundary, CLI artifact proof, or panic/resource checks block acceptance unless proven equivalent.

Critical mutants and killing tests:

- Delete `vb_validate::shared::validate` in `lower_steps_to_ir` -> killed by `lower_steps_to_ir_returns_slot_reference_out_of_range_when_do_input_exceeds_slot_count`.
- Replace shared validation with no-op `Ok(())` -> killed by slot/accessor/expression/loop validation scenarios.
- Reverse order to `try_from_parts` before shared validate -> killed by `validate_ir_returns_slot_reference_out_of_range_before_core_acceptance_when_do_input_exceeds_slot_count` and Kani order harness.
- Convert `ValidationError::SlotReferenceOutOfRange` to `WorkflowError::SlotOutOfBounds` -> killed by exact slot reference tests.
- Drop context payload from `SlotReferenceOutOfRange` -> killed by context equality assertion.
- Convert any `WorkflowError` into `CompileError::Validation` -> killed by workflow preservation tests.
- Remove core `CompiledWorkflow::try_from_parts` after shared validation -> killed by `WorkflowError::EmptyBranchTable`, `NodeIdMismatch`, and `ConstOutOfBounds` scenarios.
- Return multiple errors for isolated failures -> killed by `compile_errors_contains_exactly_one_error_for_isolated_validation_and_core_failures`.
- Use plain `validate` inside `compile_workflow_with_contracts` -> killed by missing/orphan contract scenarios.
- Add duplicate-contract rejection to current safe pure duplicate behavior -> killed by `compile_workflow_with_contracts_accepts_duplicate_safe_pure_contracts_for_same_action_id`.
- Claim gate 12 in plain validation -> killed by `shared_validate_returns_ok_for_action_without_contract_when_plain_mode_is_used`.
- Change `SourceTooLarge` comparison from `>` to `>=` -> killed by exact-sufficient and one-below/one-above source limit tests.
- Change depth/scalar/mapping/sequence concrete limits -> killed by exact zero-limit boundary tests with `depth: 1`, `actual: 7`, `actual: 4`, and `actual: 1`.
- Change `CompiledWorkflow::try_from_parts` empty-node/entry/node-id order or variants -> killed by direct core constructor BDD tests.
- Edit inactive `slot.rs`, `types.rs`, `api_validation.rs`, or `compile.rs` only -> killed by module graph static check plus live `lib.rs` public API tests.
- Add YAML/JSON/HTTP dependency to hot runtime -> killed by runtime import scan with empty allowlist.
- Add `unwrap`/`expect`/`panic` in new tests or code -> killed by safety static scan.
- Make CLI artifact test assert only file existence -> killed by required byte-for-byte blessed fixture and `postcard::from_bytes::<WorkflowParts>` decode oracle.

## 8. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|---|---|---|---|
| minimum YAML | valid minimum | `Ok(CompiledWorkflow)` with exact `MINIMAL_YAML` IR facts | integration |
| empty source | empty bytes | `Err(CompileErrors(vec![CompileError::EmptySource]))` | integration |
| malformed YAML | non-empty invalid YAML | sole `CompileError::Parse(_)` | integration/fuzz |
| source limit equal | source len == limit | `Ok` with exact IR facts | integration |
| source limit one below | source len == limit + 1 | `SourceTooLarge { actual, limit }` | integration/proptest |
| zero depth | `max_depth = 0` | `DepthLimit { depth: 1, limit: 0 }` | integration |
| zero scalar | `max_scalar_bytes = 0` | `ScalarLimit { actual: 7, limit: 0 }` | integration |
| zero mapping | `max_mapping_entries = 0` | `MappingLimit { actual: 4, limit: 0 }` | integration |
| zero sequence | one step with `max_sequence_len = 0` | `SequenceLimit { actual: 1, limit: 0 }` | integration |
| lower min vectors | one valid finish node | `Ok` with exact counts/entry | integration |
| lower empty nodes | empty node vector | `CompileError::Workflow(WorkflowError::EmptyNodes)` | integration |
| lower one-past slot | `Do.input == slot_count` | `SlotReferenceOutOfRange { slot, slot_count, context }` | integration |
| validate min parts | `to_parts()` from minimal compile | `Ok` and public fields equal original | integration |
| validate shared invalid | core-constructible shared-invalid parts | exact `CompileError::Validation` variant | integration |
| validate core invalid | shared-valid core-invalid parts | exact `CompileError::Workflow` variant | integration |
| contract exact match | action set == contract set | `Ok` and `validate_with_contracts == Ok(())` | integration |
| duplicate safe contracts | two safe pure contracts with id 7 | `Ok(CompiledWorkflow)` | integration |
| contract missing empty set | actions non-empty, contracts empty | `ActionContractMissing { action_id: 7, node_index: 0 }` | integration |
| contract orphan | contract 99 no action 99 | `ActionContractOrphan { action_id: 99 }` | integration |
| plain mode action no contract | action exists, no contracts supplied | `vb_validate::shared::validate == Ok(())` | unit |
| try_from_parts min | valid direct `WorkflowParts` | workflow name/entry/node_count/constant exact | unit/integration |
| try_from_parts empty | zero nodes | `WorkflowError::EmptyNodes` | unit/integration |
| try_from_parts bad entry | one node, entry one | `EntryOutOfBounds { entry: StepIdx::new(1) }` | unit/integration |
| try_from_parts bad id | node id one at index zero | `NodeIdMismatch { expected: StepIdx::new(0), actual: StepIdx::new(1) }` | unit/integration |
| every shared gate error | constructed enum variants | exact same payload in `CompileError::Validation` | unit/proptest |
| every reachable core error | constructed fixtures | exact same payload in `CompileError::Workflow` | unit/proptest |
| CLI valid compile | exact fixture path | exit 0, byte-equal blessed postcard, decoded shared-valid parts | e2e |
| CLI invalid compile | exact invalid fixture | exit 2, typed gate diagnostic text, no artifact | e2e |
| runtime imports | source/dependency diff | zero unallowlisted hits; allowlist file empty | static |
| module graph | `lib.rs` declarations | live facade covered; stale split files not counted | static |
| safety scan | changed files | zero unallowlisted forbidden constructs | static |

## 9. Required Command Gates and Oracles

Targeted commands before handoff:

1. `cargo test -p vb_compile compile_workflow_returns_minimal_workflow_when_source_is_minimum_valid_yaml`.
2. `cargo test -p vb_compile yaml_compiler_compile_returns_depth_limit_when_max_depth_is_zero` -> exact `depth: 1`.
3. `cargo test -p vb_compile yaml_compiler_compile_returns_scalar_limit_when_max_scalar_bytes_is_zero` -> exact `actual: 7`.
4. `cargo test -p vb_compile yaml_compiler_compile_returns_mapping_limit_when_max_mapping_entries_is_zero` -> exact `actual: 4`.
5. `cargo test -p vb_compile lower_steps_to_ir_returns_slot_reference_out_of_range_when_do_input_exceeds_slot_count`.
6. `cargo test -p vb_compile validate_ir_returns_slot_reference_out_of_range_before_core_acceptance_when_do_input_exceeds_slot_count`.
7. `cargo test -p vb_compile compile_workflow_with_contracts_accepts_duplicate_safe_pure_contracts_for_same_action_id`.
8. `cargo test -p vb_core try_from_parts_returns_entry_out_of_bounds_when_entry_is_one_and_one_node_exists`.
9. CLI integration test `cli_compile_emits_blessed_artifact_when_source_is_minimal_valid` using byte equality with `crates/velvet_ballastics/tests/fixtures/compile/minimal_valid.postcard` and decode API `postcard::from_bytes::<vb_core::WorkflowParts>`.
10. Static runtime import command named above with empty allowlist `crates/velvet_ballastics/tests/fixtures/static/runtime_import_allowlist.txt`.
11. Static safety scan with allowlist `crates/velvet_ballastics/tests/fixtures/static/vb_yd5x_safety_allowlist.txt`.
12. `cargo mutants` scoped to changed `vb_compile` seam code with >=90% kill rate.
13. `moon ci` final canonical gate.

## Open Questions

None. This repaired plan has no conditional expected values, no deferred variants, direct BDD coverage for every counted public signature, named CLI fixture/decode/artifact oracles, and named deterministic static allowlists.
