# Codebase Map: vb-yd5x

Bead: `vb-yd5x`  
Title: `validate/compile: Prove shared validated IR usage`  
State: State 2 MAP  
Workspace: `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25`

## Relevant crates/modules/files

- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_validate/src/shared.rs`
  - Central shared compiled-IR validation pipeline.
  - Exposes `ValidationPipeline`, `validate`, and `validate_with_contracts`.
  - Runs gates 7, 8, 9, 10, 11, 13, 14, 15 in order; gate 12 is contract-dependent and only in `validate_with_contracts`.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_validate/src/gates.rs`
  - Gate implementations re-exported by `shared.rs`.
  - Relevant for proving compile paths use shared validation rather than direct or duplicated checks.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_validate/src/lib.rs`
  - Defines `ValidationError`/`ValidationResult` and documents existing validation deduplication intent.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_compile/src/lib.rs`
  - Main compile facade. `YamlCompiler::compile` currently builds `WorkflowParts`, calls `vb_validate::shared::validate(&parts)`, then `CompiledWorkflow::try_from_parts(parts)`.
  - `compile_workflow_with_contracts` converts workflow back to parts and calls `vb_validate::shared::validate_with_contracts(&parts, contracts)`.
  - `validate_ir(parts)` already models the reusable pattern: shared validation first, core construction second.
  - `lower_steps_to_ir(...)` currently constructs `WorkflowParts` and directly calls `CompiledWorkflow::try_from_parts(parts)` without the shared validation call seen in `validate_ir`.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_compile/src/slot.rs`
  - Contains a duplicate-looking `validate_ir(parts)` helper and compiled artifact emit helpers. It shows the desired validation-before-core-construction sequence.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_compile/src/types.rs`
  - Contains another `YamlCompiler::compile` copy with shared validation before `CompiledWorkflow::try_from_parts`; likely legacy/split artifact to inspect for drift before editing.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_compile/src/api_validation.rs`
  - Contains another `validate_ir(parts)` copy with shared validation before core construction; likely generated/split artifact not currently declared in `lib.rs`, but useful as pattern/drift evidence.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_compile/src/compile.rs`
  - Contains public compile/lower API copy. Its `lower_steps_to_ir` directly calls `CompiledWorkflow::try_from_parts` and may be stale compared with `lib.rs` because it lacks fields visible in current `WorkflowParts`.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_core/src/workflow/mod.rs`
  - Canonical compiled workflow IR types.
  - `WorkflowParts` is explicitly untrusted compiler output.
  - `CompiledWorkflow::try_from_parts` runs core structural and budget validation, but not the `vb_validate::shared` gates.
  - `CompiledWorkflow::to_parts` is used by compile, CLI, storage, UI, IPC, and tests.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_core/src/compiled_workflow.rs`
  - Older/split compiled workflow copy; inspect before editing only if it is still wired through module exports.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/docs/compiled-ir.md`
  - Describes compiled IR as the runtime contract between cold compiler and hot engine; states `WorkflowParts` is untrusted compiler output and `CompiledWorkflow::try_from_parts` validates numeric references.
- Runtime/consumer touchpoints that deserialize or revalidate compiled artifacts:
  - `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballistics/src/run.rs`
  - `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballistics/src/storage.rs`
  - `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballistics/src/commands_verify.rs`
  - `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_storage/src/admission.rs`
  - `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_ipc/src/server/handlers.rs`

## Current patterns to reuse

- Preferred validation sequence:
  1. Build/obtain `WorkflowParts`.
  2. Run `vb_validate::shared::validate(&parts)` or `validate_with_contracts(&parts, contracts)`.
  3. Construct trusted `CompiledWorkflow` with `CompiledWorkflow::try_from_parts(parts)`.
- Existing examples:
  - `YamlCompiler::compile` in `vb_compile/src/lib.rs` lines 162-165.
  - `validate_ir(parts)` in `vb_compile/src/lib.rs` lines 701-703.
  - `compile_workflow_with_contracts` in `vb_compile/src/lib.rs` lines 219-224.
  - `commands_verify.rs` validates `compiled.to_parts()` using `vb_validate::shared::validate` as a verification phase.
- Keep error conversion style consistent: `map_err(|e| CompileErrors(vec![e.into()]))?` for validation/core errors.
- Keep hot runtime free of YAML/JSON/HTTP. Shared validation belongs on compile/admission/verification boundaries, not inside hot execution loops.
- Do not rely on `CompiledWorkflow::try_from_parts` as proof of shared gate usage; it checks core structural/budget invariants, while `vb_validate::shared` owns gates 7-15.

## Suspected touchpoints

- Primary likely implementation seam: `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_compile/src/lib.rs`
  - `lower_steps_to_ir(...)` should probably reuse `validate_ir(parts)` or call `vb_validate::shared::validate(&parts)` before `CompiledWorkflow::try_from_parts(parts)`.
  - This is the clearest compile path still bypassing the shared validation pipeline.
- Contract compile path:
  - `compile_workflow_with_contracts(...)` already uses `validate_with_contracts`; tests should prove this stays true, especially gate 12.
- Deserialization/admission paths:
  - `run.rs` and `storage.rs` load `WorkflowParts` and only call `CompiledWorkflow::try_from_parts`; decide in contract whether bead scope is compile-only or all persisted artifact loading.
  - `vb_storage/src/admission.rs` validates with `CompiledWorkflow::try_from_parts` and checksum, but does not call `vb_validate::shared::validate`. If bead title means shared validated IR usage beyond compiler output, this may be in scope.
- IPC verify path:
  - `vb_ipc/src/server/handlers.rs` enumerates gate functions directly instead of using `vb_validate::shared::ValidationPipeline`; this may be intentional for per-gate reporting, but it is a duplication risk.
- Split/stale files:
  - `vb_compile/src/slot.rs`, `types.rs`, `api_validation.rs`, and `compile.rs` contain overlapping functions. Before implementation, confirm which are compiled via `mod` declarations. Avoid editing inactive drift unless contract says to consolidate.

## Test locations to inspect later

- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_compile/src/tests/test_21.rs`
  - Existing `validate_ir_rejects_empty_parts` and `compile_workflow_with_contracts_rejects_orphan_contract` tests.
  - Good place for targeted tests proving `lower_steps_to_ir` or compile helpers reject parts only `vb_validate::shared` would reject.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_compile/src/tests.rs`
  - Large aggregate tests include comments about shared validation catching invalid together IR.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_validate/src/shared.rs`
  - Unit tests for pipeline defaults, disabled gates, short-circuit behavior.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballistics/tests/error_chain_integration.rs`
  - Asserts user-visible error text for shared validation failures, including `compiled workflow IR failed validation: ...`.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballistics/tests/cross_crate_adversarial.rs`
  - Cross-crate seam tests for `vb_compile -> vb_core` and deterministic compiled output.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballistics/tests/cli_integration.rs`
  - CLI compile/run compiled IR paths and compiled artifact digest/equality checks.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_core/src/workflow/tests.rs`
  - Core IR construction/to_parts invariants; useful to distinguish core structural validation from shared validation.

## Risks/dependencies

- Several apparent duplicate/split files may be inactive or stale. Implementation must identify the actual module graph before editing.
- `WorkflowParts` currently includes fields such as `symbols_count` and `step_names`; stale helper files lacking fields may not be compiled. Avoid assuming all search hits are live code.
- `vb_validate::shared::validate` intentionally skips gate 12 unless contracts are supplied. Tests must not claim full gate 7-15+12 coverage for paths using plain `validate`.
- `CompiledWorkflow::try_from_parts` must remain the core structural/budget gate, but it is not enough to prove shared validated IR usage.
- Deserialized compiled artifacts may need shared validation too, but expanding scope from compile to runtime/admission can touch storage and CLI behavior. Contract should decide exact scope.
- Existing error-chain tests are sensitive to error display strings. Preserve `CompileError::Validation` conversion and wording.
- Engineering rules prohibit `unwrap`, `expect`, `panic`, unsafe, unchecked indexing, casts, and arithmetic in new code/tests.
- Canonical gate remains `moon ci`; targeted compile/tests are useful but not final proof.

## Next-state notes for rust-contract

- Define the invariant precisely: every public compile/lowering path that turns `WorkflowParts` into `CompiledWorkflow` must pass through `vb_validate::shared` before core construction, except explicitly documented hot-runtime/load paths if out of scope.
- Contract should separate:
  - `WorkflowParts` = untrusted, not validated.
  - `vb_validate::shared::validate(_)/validate_with_contracts(_)` = cold shared gates.
  - `CompiledWorkflow::try_from_parts(_)` = core structural/budget validation.
- Candidate Given/When/Then:
  - Given parts that pass core `try_from_parts` but fail a shared gate, when passed through compile/lowering validation API, then the result is `CompileError::Validation` and no `CompiledWorkflow` is returned.
  - Given a workflow compiled with action contracts containing an orphan/missing contract, when using `compile_workflow_with_contracts`, then gate 12 rejects it.
  - Given a normal valid workflow, when compiled, then compile still succeeds and `to_parts` passes `vb_validate::shared::validate`.
- Require a test that fails if `lower_steps_to_ir` bypasses `vb_validate::shared::validate`.
- If contract includes persisted compiled IR loading/admission, require a separate boundary statement and tests for `run-compiled`/storage/admission paths.
- Recommend implementation reuse a small helper rather than duplicating validation sequence in multiple compile functions.
