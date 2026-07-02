# Contract Specification: vb-f04l

## Context

- Feature: safe v1 primitive source lowering in `vb_compile`.
- Bead: `vb-f04l`, `compiler: Safe v1 primitive source lowering`.
- Source of truth read: `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-f04l --json`.
- State2 artifacts read: `codebase-map.md`, `delivery-scope.jsonl`, `baseline-report.md`, `STATE.md`.
- Primary target: `crates/vb_compile/src/lib.rs::compile_source(&vb_yaml::ast::WorkflowSource) -> Result<CompiledWorkflow, CompileErrors>`.
- Current gap: canonical `compile_source` lowers only `Set` and terminal `Finish`; canonical `ForEach`, `Together`, `Collect`, `Reduce`, `Repeat`, `Wait`, and `Ask` return `CompileError::UnsupportedStepPrimitive`.

## Domain Terms

- Canonical source AST: `vb_yaml::ast::WorkflowSource`, with ordered `StepAst` values and `StepPrimitive` variants.
- Numeric IR: `vb_core::CompiledNode` array containing `CompiledNodeKind` variants with numeric `StepIdx`, `SlotIdx`, `ConstIdx`, and `ActionId` references only.
- Dense node index: every emitted node has `id == StepIdx(position)` and every branch/body/done/join/resume target is inside the emitted node array.
- Dense slot index: `slot_count` is exactly one more than the maximum referenced slot, or zero when no slots are referenced.
- Lowering boundary: source AST to validated `CompiledWorkflow` through `WorkflowParts`, `vb_validate::shared::validate`, and `CompiledWorkflow::try_from_parts`.
- Legacy compiler AST: `vb_compile::ast::StepKindAst`; it may remain, but canonical v1 admission must not depend on low-level author input.

## Assumptions

- `vb_yaml` remains the canonical parser and AST validator for source YAML.
- Runtime crates remain YAML-free.
- Existing strict parser, schema, control-flow, type/taint, and validation tests must remain intact.
- This contract does not require generated Rust/codegen parity.
- `Wait` and `Ask` are included because State2 found canonical AST and runtime IR support, and dependency bead `vb-core-lower-control-primitives` says wait/ask as applicable.
- Nested body lowering may allocate additional synthetic IR nodes, but those nodes must be named, dense, validated, and traceable to source steps.

## Open Questions

- OQ-001: Exact source-expression-to-slot policy for `input`, `source`, `initial`, `event`, `timeout`, and `prompt` is not fully discovered in State2.
- OQ-002: Whether `ForEachJoin`, `TogetherBranch`, `CollectNext`, and `RepeatCheck` are required for canonical lowering, or intentionally optional runtime variants, needs proof/test review before implementation.
- OQ-003: Whether legacy `YamlCompiler::parse_ast` remains a supported low-level path or transitional compatibility is out of bead-local contract scope.

## Preconditions

- PRE-001: Caller provides a parsed, canonical `WorkflowSource` accepted by `vb_yaml`; `source.steps()` is non-empty.
- PRE-002: `WorkflowSource` uses `version == "velvet-ballistics/v1"` and canonical trigger forms supported by `vb_yaml`.
- PRE-003: Canonical compile scope excludes unsupported top-level declarations until the dedicated values/actions/reference bead extends it: non-empty `inputs`, `vars`, `secrets`, `examples`, and top-level `result` must produce explicit compile errors.
- PRE-004: Step IDs are unique in every lowered scope; duplicate IDs in the top-level source or nested bodies must be rejected before runtime validation.
- PRE-005: Unsupported step control fields in the current compile scope (`name`, `if`, `with`, `try_again`, `on_error`, `then`) must produce explicit compile errors unless a primitive-specific contract extension consumes them.
- PRE-006: Primitive source fields that name variables, outputs, branch labels, loop variables, prompts, events, and references must be non-empty after canonical parsing.
- PRE-007: Bounded primitive parameters must fit target IR widths: `Together` branch count fits `u16`; `Repeat.max_attempts` fits `u16`; all generated step and slot indexes fit `u16`; limits/pages/items fit `u32`.

## Postconditions

- POST-001: A valid canonical AST containing `ForEach`, `Together`, `Collect`, `Reduce`, `Repeat`, `Wait`, or `Ask` lowers to mathematically equivalent `CompiledNodeKind` IR instead of `UnsupportedStepPrimitive`.
- POST-002: `compile_source` returns only a `CompiledWorkflow` that has passed `vb_validate::shared::validate` and `CompiledWorkflow::try_from_parts`.
- POST-003: The emitted `WorkflowParts.name`, `digest`, `entry`, `resource_contract`, `step_names`, constants, expressions, accessors, and slots are deterministic functions of the canonical AST.
- POST-004: Every emitted node has a valid dense `StepIdx` and every `next`, `body`, `done`, `join`, branch, resume, exhausted, or handler target is inside the emitted node array.
- POST-005: Every emitted slot reference is less than `slot_count`; slot allocation is deterministic and has no gaps above the maximum referenced slot.
- POST-006: `ForEach` emits a bounded loop graph whose start, body, next/join/done nodes preserve source body execution and route completion to the next source step or terminal finish.
- POST-007: `Together` emits a bounded fan-out/fan-in graph whose branch entries are all reachable, whose join waits for exactly the emitted branch count, and whose completion routes once.
- POST-008: `Collect` emits a bounded paginated loop graph preserving source body execution, page limit, page size, collector slot, and completion route.
- POST-009: `Reduce` emits a bounded reduction graph preserving input slot, accumulator slot, initial value, body execution, and finish route.
- POST-010: `Repeat` emits a bounded retry graph preserving maximum attempts, attempt state, body route, check/finish behavior, and exhaustion route.
- POST-011: `Wait` emits exactly one legal wait IR shape: `WaitUntil` for deadline-only form or `WaitEvent` for event form with optional timeout; invalid event/timeout combinations are rejected.
- POST-012: `Ask` emits `Ask` and `AskResume` nodes with valid prompt, optional timeout, answer slot, and resume target.
- POST-013: Existing `Set` and terminal `Finish` behavior remains compatible, including named output lookup and last-step restriction.
- POST-014: Existing tests and legacy compiler files are not deleted to hide regressions.

## Invariants

- INV-001: Numeric node IDs are dense and position-aligned: for all node positions `i`, `nodes[i].id == StepIdx(i)`.
- INV-002: All target step references are in range and preserve a well-formed forward/body graph accepted by `vb_validate` gates.
- INV-003: All slot references are in range, and `slot_count` covers every input, output, accumulator, iterator, prompt, answer, timeout, collector, and result slot.
- INV-004: All primitive bounds are finite and checked before narrowing conversion or allocation.
- INV-005: Canonical lowering is deterministic: equal `WorkflowSource` values produce equal digest and equal IR parts.
- INV-006: No valid v1 primitive remains reachable as `UnsupportedStepPrimitive` unless explicitly excluded by this contract.
- INV-007: No untested primitive path is accepted: each newly accepted primitive has at least one positive and one negative executable scenario.
- INV-008: No implementation may use `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, unchecked indexing/slicing, unchecked casts, or unchecked arithmetic in production paths.
- INV-009: Runtime core remains free of YAML/JSON/HTTP dependencies; YAML stays in the cold compiler boundary.
- INV-010: Existing legacy tests and files remain present unless a superseding, reviewed migration bead approves deletion.

## Error Taxonomy

- ERR-001: `CompileError::EmptySteps` when a canonical source has no steps.
- ERR-002: `CompileError::UnsupportedTopLevelDeclaration` or `CompileError::UnsupportedTopLevelResult` when unsupported declarations remain in source.
- ERR-003: `CompileError::UnsupportedStepControlField` when unsupported control fields are present.
- ERR-004: `CompileError::DuplicateStepId` for duplicate top-level or nested source step IDs.
- ERR-005: `CompileError::DuplicateOutputName` for repeated named outputs in the same lowering scope.
- ERR-006: `CompileError::UnknownOutputName` for `finish` or source expressions that reference absent outputs.
- ERR-007: `CompileError::StepFieldShape` for missing, empty, incompatible, or ambiguous primitive fields.
- ERR-008: `CompileError::StepIndexOutOfRange`, `CompileError::SlotIndexOutOfRange`, or `CompileError::PrimitiveLoweringLimitExceeded` for checked numeric bound failures.
- ERR-009: `CompileError::Workflow` wrapping `WorkflowError` when core structural validation rejects generated parts.
- ERR-010: `CompileError::CanonicalYaml` when `YamlCompiler::compile` receives source rejected by canonical YAML parsing.
- ERR-011: `CompileError::UnsupportedStepPrimitive` remains legal only for primitives explicitly outside this bead scope (`Save`, `Do`, `Choose`) until their beads contract them; it is illegal for `ForEach`, `Together`, `Collect`, `Reduce`, `Repeat`, `Wait`, and `Ask` after implementation.

## Contract Signatures

- Existing: `pub fn compile_source(source: &vb_yaml::ast::WorkflowSource) -> Result<CompiledWorkflow, CompileErrors>`.
- Existing: `pub fn compile_workflow(source: &[u8]) -> Result<CompiledWorkflow, CompileErrors>`.
- Existing: `impl YamlCompiler { pub fn compile(&self, source: &[u8]) -> Result<CompiledWorkflow, CompileErrors> }`.
- Existing helper targets: `lower_for_each`, `lower_together`, `lower_collect`, `lower_reduce`, `lower_repeat`, `lower_wait`, `lower_ask` in `crates/vb_compile/src/lib.rs`.
- Contracted internal abstraction for downstream design, not yet an implementation signature: source body expansion must be a fallible operation returning `Result<LoweredBody, CompileErrors>` or equivalent, never panicking.

## Verus-Owned Clauses

- INV-001, INV-003, INV-004, INV-005 and PRE-007 are Rust-local pure/core obligations suitable for Verus over an abstract lowering plan.
- POST-006 through POST-012 require Verus for local primitive shape preservation through `verification/verus/v1_primitive_lowering.rs`; the proof surface must be non-vacuous and must derive shape claims from abstract constructors/transitions or bridge invariants.
- INV-008 is enforced by static scan and code review, not Verus alone.

## TLA+-Owned Clauses

- POST-006: bounded `ForEach` lifecycle.
- POST-007: bounded `Together` fan-out/fan-in lifecycle.
- POST-008: bounded `Collect` pagination lifecycle.
- POST-009: bounded `Reduce` lifecycle.
- POST-010: bounded `Repeat` retry lifecycle.
- POST-011 and POST-012: suspend/resume lifecycle for `Wait` and `Ask` at model level.

## Theorem-Owned Clauses

- No Lean/Aeneas/Hax theorem kernel is mandatory at contract time.
- If Verus cannot express dense-index preservation for recursive nested body expansion without excessive trusted code, a tiny Lean theorem may model preorder allocation preserving dense indexes.

## Non-goals

- Generated Rust/codegen parity.
- Runtime execution semantics beyond valid numeric IR shape and lifecycle model.
- Adding YAML dependencies to runtime crates.
- Removing legacy `parse_ast`, `lower/mod.rs`, or other migration-era compiler files.
- Performance speedup claims; performance requirement is no uncontrolled asymptotic or allocation blow-up, not a benchmarked acceleration.
