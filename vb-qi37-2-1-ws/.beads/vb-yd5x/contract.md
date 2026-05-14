# Contract Specification: vb-yd5x

## Context

- Bead: `vb-yd5x`
- Title: `validate/compile: Prove shared validated IR usage`
- Scope owner: `vb_compile` cold compile/lowering boundary.
- Primary invariant source: `vb_validate::shared` gates 7, 8, 9, 10, 11, 13, 14, and 15 for plain compiled IR; gate 12 only when action contracts are supplied.
- Trusted-core boundary: `vb_core::workflow::CompiledWorkflow::try_from_parts` remains the constructor for core structural and budget invariants, but it is not proof that shared validation gates ran.

## Domain Terms

- `WorkflowParts`: untrusted compiler/lowering output. It can be serialized, cloned from `CompiledWorkflow::to_parts`, or assembled by lowering helpers, but is not trusted until validated.
- `Shared validation pipeline`: `vb_validate::shared::validate` or `vb_validate::shared::validate_with_contracts`, using `ValidationPipeline::default` unless a caller explicitly opts into a narrower pipeline outside this bead.
- `Core construction`: `CompiledWorkflow::try_from_parts`, which validates numeric references and resource budgets before creating a trusted immutable `CompiledWorkflow`.
- `Plain validation`: shared gates excluding gate 12 because action contracts are unavailable.
- `Contract validation`: shared gates including gate 12 with caller-provided `ActionContract` values.
- `Compile/lowering boundary`: any public `vb_compile` API that accepts YAML or step-level IR and returns `Result<CompiledWorkflow, CompileErrors>`.

## Scope

### In Scope

- Prove every live public `vb_compile` path that turns newly built `WorkflowParts` into `CompiledWorkflow` runs shared validation before core construction.
- Require `lower_steps_to_ir` to use the same validation-before-construction sequence as `validate_ir` and `YamlCompiler::compile`.
- Preserve `compile_workflow_with_contracts` behavior: compile first, convert to parts, run `validate_with_contracts`, then run idempotency gates.
- Preserve typed error propagation into `CompileError::Validation` for shared gate failures and `CompileError::Workflow` for core construction failures.
- Add proof-oriented tests/scenarios that would fail if `lower_steps_to_ir` or other live compile paths bypass `vb_validate::shared`.

### Out of Scope

- Hot runtime execution loops.
- Introducing JSON, YAML, or HTTP parsing into runtime core.
- Redesigning `WorkflowParts`, `CompiledWorkflow`, gate implementations, or action contract semantics.
- Consolidating inactive split/stale files unless they are proven live in the module graph.
- Expanding persisted artifact loading/admission behavior in `run.rs`, `storage.rs`, `vb_storage::admission`, or IPC per-gate reporting. Those may need separate beads if product scope requires shared validation on deserialized artifacts.
- Performance claims or benchmark changes.

## Assumptions

- `crates/vb_compile/src/lib.rs` is the live compile facade for this bead unless module graph inspection proves otherwise.
- `CompiledWorkflow::try_from_parts` must continue to run after shared validation because it owns core structural/budget checks.
- `vb_validate::shared::validate` intentionally skips gate 12; tests must not claim gate 12 coverage for plain compile paths.
- Error display wording is user-visible and should remain compatible with existing integration tests.

## Open Questions

- Should future work require shared validation on all deserialized compiled artifacts before admission/run, or is this bead intentionally compile-only?
- Should stale overlapping files in `vb_compile/src/{slot.rs,types.rs,api_validation.rs,compile.rs}` be deleted/consolidated in a separate drift bead after live module graph proof?
- Should IPC per-gate reporting keep direct gate enumeration, or should it wrap `ValidationPipeline` while preserving per-gate diagnostics?

## Invariants

1. `WorkflowParts` is always untrusted at compile/lowering boundaries.
2. A public `vb_compile` API must not return `Ok(CompiledWorkflow)` from newly assembled `WorkflowParts` unless `vb_validate::shared` succeeded first.
3. Plain compile/lowering paths must run shared gates 7, 8, 9, 10, 11, 13, 14, and 15 before `CompiledWorkflow::try_from_parts`.
4. Contract-aware compile paths must run `validate_with_contracts` before returning `Ok(CompiledWorkflow)` to the caller.
5. Core construction remains mandatory after shared validation; shared validation does not replace `CompiledWorkflow::try_from_parts`.
6. Shared validation failures must be represented as `CompileError::Validation` inside `CompileErrors`.
7. Core construction failures must be represented as `CompileError::Workflow` inside `CompileErrors`.
8. No hot-runtime loop may gain YAML, JSON, HTTP, or validation-heavy compile concerns as part of this bead.
9. Validation order for plain paths is shared validation first, core construction second. Reversing the order violates this contract.
10. Gate 12 is only claimed for APIs receiving action contracts.

## Preconditions

### `compile_workflow` / `YamlCompiler::compile`

- Input is a byte slice owned by the caller and bounded by `YamlLimits`.
- YAML parsing and cold AST validation are allowed only in cold compile code.
- The compiler must produce `WorkflowParts` before shared compiled-IR validation.

### `lower_steps_to_ir`

- Caller supplies owned node/expression/accessor/constant vectors and scalar metadata.
- The function treats all supplied vectors and metadata as untrusted.
- Entry is determined by the lowering API contract (`StepIdx::new(0)` in the current shape).
- No caller may rely on `lower_steps_to_ir` to skip shared gates for speed.

### `validate_ir`

- Caller supplies owned `WorkflowParts`.
- The function treats `WorkflowParts` as untrusted, even if they came from a prior `to_parts` call.

### `compile_workflow_with_contracts`

- Caller supplies YAML source plus the complete intended action contract set.
- Contracts are cold deployment/admission data, not hot-runtime parsing data.

## Postconditions

### Success

- Returned value is a trusted `CompiledWorkflow` created by `CompiledWorkflow::try_from_parts`.
- Shared validation has succeeded before core construction for newly assembled `WorkflowParts`.
- `workflow.to_parts()` remains accepted by `vb_validate::shared::validate` for plain compile/lowering success cases.
- For contract-aware success, `validate_with_contracts(workflow.to_parts(), contracts)` succeeds and idempotency gates succeed.

### Failure

- If a shared gate rejects the IR, no `CompiledWorkflow` is returned.
- If a shared gate rejects the IR, the error is carried as `CompileError::Validation` inside `CompileErrors`.
- If core construction rejects the IR after shared validation succeeds, no `CompiledWorkflow` is returned and the error is carried as `CompileError::Workflow`.
- If gate 12 rejects contracts, no `CompiledWorkflow` is returned from `compile_workflow_with_contracts`.

## Typed Error Taxonomy

- `CompileError::Validation(vb_validate::ValidationError)`: shared validation pipeline failure. Includes gate errors such as `ExpressionStackExceeded`, `ExpressionStackMismatch`, `AccessorSlotOutOfRange`, `AccessorPathInvalid`, `SlotReferenceOutOfRange`, `LoopBodyStepOutOfRange`, `SlotDependencyCycle`, `NodeKindConstraintViolation`, `ActionContractMissing`, `ActionContractOrphan`, `SlotTypeInconsistency`, and `NonDeterministicPath`.
- `CompileError::Workflow(vb_core::workflow::WorkflowError)`: core structural/budget construction failure from `CompiledWorkflow::try_from_parts`.
- `CompileErrors(Vec<CompileError>)`: aggregate facade error. For this bead's validation/core conversion seam, a single semantic error is expected in the vector.
- No stringly typed validation failures are permitted at the seam; failures must preserve their enum variant through `Result<T, Error>`.

## Contract Signatures

These signatures define the behavior to preserve or require; implementations may factor helpers but must keep equivalent semantics.

```text
fn compile_workflow(source: &[u8]) -> Result<CompiledWorkflow, CompileErrors>
fn YamlCompiler::compile(&self, source: &[u8]) -> Result<CompiledWorkflow, CompileErrors>
fn lower_steps_to_ir(...) -> Result<CompiledWorkflow, CompileErrors>
fn validate_ir(parts: WorkflowParts) -> Result<CompiledWorkflow, CompileErrors>
fn compile_workflow_with_contracts(source: &[u8], contracts: &[ActionContract]) -> Result<CompiledWorkflow, CompileErrors>
fn vb_validate::shared::validate(parts: &WorkflowParts) -> Result<(), ValidationError>
fn vb_validate::shared::validate_with_contracts(parts: &WorkflowParts, contracts: &[ActionContract]) -> Result<(), ValidationError>
fn CompiledWorkflow::try_from_parts(parts: WorkflowParts) -> Result<CompiledWorkflow, WorkflowError>
```

## Acceptance Criteria

1. `lower_steps_to_ir` cannot directly call `CompiledWorkflow::try_from_parts(parts)` without first invoking the shared validation pipeline or a helper that does so.
2. `validate_ir` remains the canonical reusable validation-before-core helper, or an equivalent helper exists with the same ordering and typed errors.
3. `YamlCompiler::compile` and `compile_workflow` still run shared validation before core construction.
4. `compile_workflow_with_contracts` still runs `validate_with_contracts` and preserves gate 12 rejection.
5. At least one test proves a `WorkflowParts` value that would otherwise be core-constructible but violates a shared gate is rejected at the compile/lowering API boundary.
6. At least one test proves `lower_steps_to_ir` rejects a shared-gate violation, preventing regression to direct core construction.
7. At least one test proves contract-aware compilation rejects missing or orphan action contracts via `CompileError::Validation`.
8. No production code or tests introduce `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing/slicing, unchecked casts, or unchecked arithmetic.
9. No runtime-core JSON/YAML/HTTP dependency or parsing behavior is introduced.
10. `moon ci` is the canonical final gate for implementation completion.

## Martin Fowler Given/When/Then Scenarios

### Scenario 1: Valid YAML compiles through shared validation

Given a valid workflow YAML document that satisfies AST, shared IR gates, and core structural invariants
When `compile_workflow` is called
Then it returns `Ok(CompiledWorkflow)`
And `workflow.to_parts()` passes `vb_validate::shared::validate`
And no validation step is duplicated outside the shared pipeline.

### Scenario 2: Lowering rejects IR that violates a shared gate

Given step-level IR inputs that assemble into `WorkflowParts` accepted by core structural validation but rejected by one shared gate
When `lower_steps_to_ir` is called
Then it returns `Err(CompileErrors)`
And the contained error is `CompileError::Validation`
And no `CompiledWorkflow` is returned.

### Scenario 3: `validate_ir` preserves shared-before-core ordering

Given untrusted `WorkflowParts` with a shared gate violation
When `validate_ir` is called
Then `vb_validate::shared::validate` rejects the parts first
And the function returns `CompileError::Validation`
And `CompiledWorkflow::try_from_parts` is not the basis for accepting the value.

### Scenario 4: Core-only failure remains typed as workflow error

Given `WorkflowParts` that satisfy shared gates but violate a core structural or budget invariant
When a compile/lowering API validates and constructs the workflow
Then shared validation succeeds
And core construction fails
And the returned error is `CompileError::Workflow`.

### Scenario 5: Contract-aware compile rejects missing action contract

Given a workflow containing a `Do` node with an action id
And the supplied contract set lacks that action id
When `compile_workflow_with_contracts` is called
Then gate 12 rejects the workflow
And the returned error is `CompileError::Validation(ValidationError::ActionContractMissing)`.

### Scenario 6: Contract-aware compile rejects orphan contract

Given a workflow with no matching `Do` node for a supplied action contract
When `compile_workflow_with_contracts` is called
Then gate 12 rejects the contract set
And the returned error is `CompileError::Validation(ValidationError::ActionContractOrphan)`.

### Scenario 7: Hot runtime remains free of cold validation concerns

Given a compiled workflow already admitted for execution
When hot runtime execution proceeds
Then this bead adds no YAML, JSON, HTTP, or shared compile-validation parsing to the hot loop
And any persisted artifact validation expansion is deferred to a separate boundary decision.

## Contract Verification Test Plan

- `compile_workflow_returns_validated_compiled_workflow_for_valid_source`
- `lower_steps_to_ir_returns_validation_error_for_shared_gate_violation`
- `validate_ir_returns_validation_error_before_core_acceptance_for_shared_gate_violation`
- `compile_path_returns_workflow_error_for_core_only_structural_violation`
- `compile_workflow_with_contracts_rejects_missing_action_contract`
- `compile_workflow_with_contracts_rejects_orphan_action_contract`
- `compile_errors_preserve_validation_variant_for_shared_gate_failure`
- `compile_errors_preserve_workflow_variant_for_core_construction_failure`
- `plain_compile_paths_do_not_claim_gate_12_without_contracts`
- `runtime_core_imports_do_not_gain_yaml_json_http_from_this_change`

## Proof Obligations

- Static proof: live `vb_compile` APIs that construct `CompiledWorkflow` from new `WorkflowParts` must show the order `shared validate -> try_from_parts`.
- Regression proof: a targeted test must fail against the current `lower_steps_to_ir` bypass if shared validation is removed.
- Error proof: tests must inspect the typed variant, not only string output.
- Boundary proof: implementation review must distinguish live module files from stale split artifacts before editing.
- CI proof: final implementation state must pass `moon ci`; targeted tests alone are not sufficient.

## Risk Notes

- Some files contain overlapping compile helpers and may be stale. Editing inactive files can create false confidence without changing behavior.
- A test case that violates both shared and core invariants cannot prove ordering; choose fixtures that isolate one failure class at a time.
- Gate 12 requires contracts and must not be asserted for plain validation.
- Existing integration tests may depend on error wording such as `compiled workflow IR failed validation` or `validation gate failure`; preserve public display strings unless intentionally migrated.
- Expanding validation to persisted artifact loading could alter CLI/storage/runtime behavior and should be scoped separately.
