# State 2 Codebase Map: vb-nsnc

Bead: `vb-nsnc`
Title: `verifier/runtime: Define capability contract schema`
Workspace: `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25`
Status: State 2 artifact retry, write-capable, no production/test edits made.

## Relevant Files

- `crates/vb_core/src/capability.rs`: owns the core capability model. `Capability { name: Box<str>, action: ActionId }` and `CapabilitySet` already exist. Matching permits exact names and dotted hierarchical child requirements, but only when `ActionId` also matches. Empty grant names intentionally grant nothing.
- `crates/vb_core/src/action.rs`: `ActionContract` already carries `required_capabilities: Box<[Capability]>`. This is the canonical contract field downstream validation should inspect.
- `crates/vb_runtime/src/admission.rs`: `RunAdmission` stores granted capabilities. `check_capability(action, required, granted)` returns `AdmissionError::CapabilityDenied` with the denied requirement and grants.
- `crates/vb_runtime/src/engine/action.rs`: runtime execution checks `resolved.required_capabilities` against granted capabilities before dispatching a contracted action. The no-contract execution path cannot enforce requirements because it cannot enumerate them.
- `crates/vb_runtime/src/shard/lifecycle.rs`: translates admission errors into `RuntimeError::AdmissionCapabilityDenied`, but current `admit_run` only verifies artifact presence and records grants; capability checks are not currently performed at submit/admission for action contracts.
- `crates/vb_validate/src/gate_12_14_15.rs`: gate 12 currently verifies only that every `Do` node has a matching `ActionContract` and that every supplied contract is used. It does not validate the shape or semantics of `required_capabilities`.
- `crates/vb_validate/src/shared.rs`: `ValidationPipeline::validate_with_contracts` is the public path that runs gate 12 with `ActionContract` data after the non-contract gates. This is the likely integration point for capability contract schema checks if they belong with contract validation.
- `crates/vb_validate/src/schema.rs`, `schema_fields.rs`, `schema_doc.rs`, `schema_tests.rs`: existing workflow-document schema validators and tests for required fields, allowed fields, IDs, duplicate fields, trigger shape, and primitive constraints. These are patterns for field validation and exact error assertions, but they validate authoring document shape rather than compiled `ActionContract` instances.
- `crates/vb_validate/src/lib.rs`: defines `ValidationError`, including `ActionContractMissing` and `ActionContractOrphan`. New capability-contract schema failures probably need new `ValidationError` variants plus diagnostic mappings.
- `crates/vb_validate/src/diagnostic.rs`, `diag_render.rs`, `diag_convert.rs`: diagnostic mappings for validation errors. Any new validation error should be mapped here to preserve CLI/UI error rendering.
- `crates/vb_validate/src/gates.rs`: appears to contain an older or parallel implementation of gate 12 and comments mentioning gate 13 capabilities. Check this before editing to avoid updating one gate path while callers use another.
- `crates/velvet_ballistics/src/main.rs`: CLI renders `ActionContractDetail.required_capabilities` and maps validation errors. Any new validation error may need CLI text/reporting updates.
- `crates/vb_ui/src/registry/mod.rs`, `crates/vb_ui/src/verify/action_policy.rs`, `crates/vb_ui_snapshot/src/fixtures.rs`, `crates/vb_ui_model/src/lib.rs`: UI and snapshot consumers already display or model required capabilities; useful downstream consumers, but likely not first implementation touchpoints.
- `velvet-ballistics-MASTER.md`: authoritative contract. Relevant notes say capability model is partial, compile-time schema validation is needed, actions declare required capabilities, operators grant capabilities, and admission checks should deny missing grants. It also says capability checking occurs at admission time only, while current runtime still performs execution-time checks.

## Patterns To Reuse

- Reuse `CapabilitySet::grants` semantics from `vb_core/src/capability.rs` as the source of truth for grant matching. Do not invent colon matching or partial lexical prefixes; current matching is exact or dotted hierarchy plus matching `ActionId`.
- Reuse gate 12 structure from `gate_12_14_15.rs`: iterate contracts with bounded loops, return the first `ValidationError`, and keep tests small with local `make_contract` helpers.
- Reuse schema validation style from `schema_fields.rs` / `schema_tests.rs`: explicit helper functions, no panics, exact `ValidationError` assertions for invalid cases.
- Reuse `validate_with_contracts(parts, action_contracts)` in `shared.rs` as the cold-path compiled IR validation entry point rather than adding ad hoc validation in runtime hot paths.
- Reuse diagnostic mapping pattern in `diagnostic.rs`, `diag_render.rs`, and `diag_convert.rs` when adding new validation errors.
- Reuse existing no-unsafe/no-unwrap style and the repo preference for checked iteration and explicit errors.

## Suspected Touchpoints

- Add a contract-schema validator near gate 12, probably in `crates/vb_validate/src/gate_12_14_15.rs`, because `required_capabilities` belongs to `ActionContract` completeness/validity.
- Consider whether `crates/vb_validate/src/gates.rs` is still active or re-exported. If both gate implementations are live, keep semantics in sync or consolidate through the used function path.
- Extend `ValidationError` in `crates/vb_validate/src/lib.rs` for cases such as empty capability name, capability action mismatch with containing contract id, duplicate capability entries, invalid capability name grammar, and potentially too-long names if a bound exists in the master contract.
- Wire new error diagnostics in `diagnostic.rs`, `diag_render.rs`, `diag_convert.rs`, and CLI validation-error formatting in `crates/velvet_ballistics/src/main.rs` if compile errors or user-facing reporting require it.
- Add tests in `crates/vb_validate/src/gate_12_14_15.rs` for compiled contract validation and in `crates/vb_validate/src/schema_tests.rs` only if the bead requires authoring document schema shape. The known gap is compiled `ActionContract.required_capabilities`, not workflow YAML fields.
- If the contract requires admission-time enforcement, the runtime touchpoint is `vb_runtime/src/admission.rs` plus submit/lifecycle call sites that have access to compiled action contracts. Current `admit_run` lacks contract input, so rust-contract should decide whether this bead is schema-only or also changes admission API.

## Current Behavior Summary

- Core type exists: `Capability { name, action }`.
- Core matching exists: grant name must be non-empty; exact names match; `network` grants `network.github`; `net` does not grant `network.github`; action IDs must match.
- `ActionContract` carries `required_capabilities` already.
- Runtime contracted `Do` execution rejects missing capabilities before dispatch.
- Admission stores granted capabilities but does not validate them against action contracts at submit time.
- Validation gate 12 verifies action-contract presence/orphan status only and ignores malformed `required_capabilities`.

## Risks And Dependencies

- The master document says capability checking occurs at admission time only, but `vb_runtime/src/engine/action.rs` currently checks at execution time. Contract should explicitly decide whether to preserve execution defense-in-depth or move checks earlier.
- Capability name grammar is not yet obvious from code. Rust-contract must define it before implementation. Existing matching assumes dotted hierarchy, so a reasonable grammar may be lower-case ID segments separated by dots; do not assume colon names unless the master requires them.
- `Capability.action` duplicates the containing `ActionContract.id`. A schema validator should likely reject required capabilities whose `action` differs from the contract id, otherwise a contract can declare an unenforceable or misleading requirement.
- Empty capability names are ignored by grant matching and should likely be invalid in required capabilities to avoid declaring requirements that can never be granted.
- Duplicate required capabilities may be harmless but should be specified. If rejected, compare both name and action to avoid false positives.
- New validation errors require diagnostic and CLI mapping or tests may fail even if gate logic compiles.
- Runtime admission API changes could ripple through shard lifecycle, CLI submit paths, tests, and fixtures. Keep the first implementation minimal if the bead scope is only schema validity.

## Test Locations

- `crates/vb_core/src/capability.rs`: existing tests for exact, hierarchical, partial-prefix rejection, action mismatch, and empty names.
- `crates/vb_validate/src/gate_12_14_15.rs`: best place for new gate 12 capability contract schema tests alongside action contract completeness tests.
- `crates/vb_validate/src/shared.rs`: add/adjust pipeline tests if the new validator is wired through `validate_with_contracts`.
- `crates/vb_validate/src/schema_tests.rs`: only relevant if authoring document schema receives a new `capabilities` field or capability declaration syntax.
- `crates/vb_runtime/src/admission.rs`: existing unit tests for capability checking; extend only if admission-time validation/enforcement is in scope.
- `crates/vb_runtime/src/engine/action.rs` and `crates/vb_runtime/src/engine/tests.rs`: existing runtime capability-denial path; useful regression coverage if enforcement placement changes.
- `crates/velvet_ballistics/tests/cli_integration.rs`: likely integration test location if validation errors surface in CLI output.

## Next-State Notes For rust-contract

- Define the capability contract schema precisely before code: required name grammar, empty-name rejection, max length if any, allowed hierarchy separator, duplicate policy, and required relation between `Capability.action` and `ActionContract.id`.
- Decide whether the validator is part of gate 12 or a new named subcheck under action contract validation. The current `ValidationPipeline` only has `gate_12_action_contracts`, so adding a subcheck under gate 12 is probably minimal.
- Decide if schema validity is purely cold-path validation or if runtime admission must also change to check all required capabilities before accepting a run. The master says admission-time, but current code enforces at execution-time.
- Specify exact `ValidationError` variants and diagnostic text so implementation can update `lib.rs`, diagnostics, and CLI mappings without guesswork.
- Include Given/When/Then cases for: valid empty required list, valid dotted capability matching contract action id, empty name rejection, partial-prefix not relevant to schema but covered in core, action mismatch rejection, duplicate rejection if required, and orphan/missing contract behavior preserved.

STATUS: COMPLETE
