# Contract Specification - vb-0sps State 3 Repair

## Context

- Bead: `vb-0sps` / `VB-BDD-CATALOG-007` executable evidence gap.
- Scope: repair the State 3 contract layer for generated-vs-IR parity acceptance scenarios. This state writes contract artifacts only; it does not write production code, tests, TLA+ code, Verus code, or proof evidence.
- Startup authority read: `/home/lewis/.claude/skills/rust-contract/SKILL.md` and `/home/lewis/.agents/skills/rust-contract/SKILL.md`; both are version `2.6.0`. The `.agents` copy wins on conflict. Relevant rules: contract-first artifacts lines 36-48, TLA+/Verus split lines 68-99, proof obligation schema lines 135-160, no implementation lines 417-423.
- Repair input: State 6 rejected the previous layer because TLA+ timed out, TLA+ proved equality by construction, TLA metadata/bounds mismatched model/config, Verus obligations were blocked without valid waiver metadata, and canonical obligations lacked exact rows for `PRE-003`, `PRE-004`, `PRE-005`, `INV-001`, `INV-004`, `INV-005`, `INV-006`.

## Domain Terms

- IR oracle: public `vb_core` interpreter/runtime observations from `CompiledWorkflow`, `RunFrame`, `run_until_blocked`, `step_once`, resume APIs, `EngineSignal`, `StepBudget`, and `ValueStore`.
- Generated candidate: public `vb_codegen` generated runtime observations emitted only after `validate_generated_subset` accepts the workflow.
- Parity observation: normalized, typed evidence from both modes. Debug/display string equality is not acceptance evidence.
- Unsupported generated subset: a workflow feature that must return `CodegenError::UnsupportedIr { feature }` before generated source is accepted, compiled, run, or silently routed to the IR oracle.
- Independent transition model: TLA+ must model IR and generated machines as separate transition relations plus an observation/refinement relation. A model that writes identical state to both sides in one action is invalid.

## Assumptions

- The eventual BDD target remains `crates/workspace_tests/tests/vb_0sps_generated_ir_parity_bdd.rs`.
- State 5 owns proof/model code. State 3 only specifies what State 5 must produce.
- Verus cannot be executable until real public/test-support parity adapters exist. This contract records explicit temporary Verus waivers with owner, limitation, expiry/follow-up, and compensating evidence rather than fake proof targets.
- TLA+ is not waived. State 5 must produce a tractable, non-vacuous model and complete TLC evidence for split bounded scenario configs.

## Preconditions

- PRE-001: The fixture is a `CompiledWorkflow` constructed by `CompiledWorkflow::try_from_parts` or an intentionally invalid bounded fixture for typed-error parity.
- PRE-002: For positive generated-vs-IR scenarios, `validate_generated_subset(&workflow)` succeeds before emission or execution.
- PRE-003: IR and generated executions start from identical public observations: input slot values, slot taints, representable value-store contents, initial PC, step states, run id, budget, and resume payloads.
- PRE-004: Suspension/resume scenarios supply identical action completion, action failure, ask answer, wait/timer, retry, and budget inputs to both modes.
- PRE-005: Unsupported-subset scenarios do not execute generated code and do not compile or accept emitted source.

## Postconditions

- POST-001: Terminal success parity: terminal result `SlotValue`, terminal result `Taint`, terminal status, final PC, executed-step count where observable, all observed slot values, all observed slot taints, all step states, and normalized terminal events match exactly.
- POST-002: Typed error parity: error class and semantic fields match exactly, or match through a documented normalized adapter whose mapping is part of the BDD assertion.
- POST-003: Suspension parity: blocked status, suspension kind, suspended step, resume PC, action id/input/output slots, action ticket fields, retry attempt, wait deadline/event/timeout fields, ask prompt/answer/timeout fields, and no-run-past-boundary behavior match.
- POST-004: Resume parity: identical resume input causes identical output slot write, taint, step-state transition, completion/answer/failure event, PC, and later terminal result/error.
- POST-005: Journal/event parity: normalized event sequence preserves order, event kind, run id, step, slot, value handle, taint, action ticket/id, retry count, wait/ask metadata, terminal event, and typed failure fields.
- POST-006: Fail-closed unsupported subset: `validate_generated_subset` or `emit_rust_workflow` returns `CodegenError::UnsupportedIr { feature }`; no generated source is accepted, compiled, run, or silently routed to IR fallback.
- POST-007: Catalog closure: `VB-BDD-CATALOG-007` has an executable evidence target and no deferred follow-up only after the BDD target exists and passes.

## Invariants

- INV-001: IR interpreter observations are the semantic oracle; generated runtime is only the candidate for supported generated-subset workflows.
- INV-002: Parity comparison is structured and typed; `Debug`/display strings are not acceptance evidence except a stable unsupported diagnostic display explicitly documented outside this bead.
- INV-003: Taint lattice values are compared at every written slot and at terminal result.
- INV-004: Step-state sequences obey the master StepState transition contract for both IR and generated machines; terminal states do not reopen.
- INV-005: Suspension is a boundary: neither mode advances PC or writes post-boundary output before matching resume input.
- INV-006: Unsupported generated features fail closed before source acceptance/emission/compile/run; no fallback-to-IR may be counted as generated parity.
- INV-007: This bead does not activate `compile --emit rust`, maxperf, PGO, generated benchmark ratios, or generated mode as a current release gate.

## Error Taxonomy

- `ParityError::TerminalMismatch { field }`: terminal result/status/PC/state field differs.
- `ParityError::SlotMismatch { slot, field }`: slot value or taint differs.
- `ParityError::StepStateMismatch { step }`: step state or legal transition differs.
- `ParityError::SuspensionMismatch { field }`: blocked kind or metadata differs.
- `ParityError::ResumeMismatch { field }`: post-resume write, taint, event, PC, or terminal observation differs.
- `ParityError::JournalMismatch { index, field }`: normalized event kind/order/field differs.
- `ParityError::TypedErrorMismatch { field }`: typed error variant or semantic field differs.
- `ParityError::UnsupportedNotFailClosed { feature }`: unsupported feature emitted/compiled/ran or fell back silently.
- `CodegenError::UnsupportedIr { feature }`: required fail-closed result for unsupported generated subset.

## Contract Signatures

- `fn validate_generated_ir_bdd_fixture(workflow: &CompiledWorkflow) -> Result<GeneratedSubset, CodegenError>`
- `fn run_ir_observed(workflow: &CompiledWorkflow, input: ParityInput) -> Result<ObservedRun, CoreError>`
- `fn run_generated_observed(workflow: &CompiledWorkflow, input: ParityInput) -> Result<ObservedRun, CodegenError>`
- `fn compare_observed_runs(ir: &ObservedRun, generated: &ObservedRun) -> Result<(), ParityError>`
- `fn assert_unsupported_generated_fail_closed(workflow: &CompiledWorkflow) -> Result<(), CodegenError>`

## Required BDD Scenario Families

1. `given_supported_deterministic_workflow_when_ir_and_generated_finish_then_terminal_state_slots_taints_steps_and_events_match`.
2. `given_supported_action_wait_and_ask_workflows_when_run_until_blocked_then_suspension_metadata_matches_and_pc_does_not_advance`.
3. `given_blocked_action_or_ask_when_both_modes_resume_with_same_value_and_taint_then_output_taint_event_pc_and_final_result_match`.
4. `given_bounded_invalid_workflows_when_both_modes_execute_then_typed_error_variant_and_fields_match`.
5. `given_unsupported_generated_ir_when_codegen_validates_then_unsupported_ir_is_returned_before_source_emission`.
6. `given_acceptance_catalog_when_vb_0sps_bdd_exists_then_catalog_007_points_to_executable_evidence_not_deferred_follow_up`.

## Verus-Owned Clauses and Temporary Waivers

- Verus-owned when adapters exist: `PRE-003`, `POST-001`, `POST-002`, `INV-002`, `INV-003` for pure normalization/comparison/refinement of bounded `ObservedRun`, `ObservedEvent`, and `NormalizedError` records.
- Temporary waiver `WAIVER-VERUS-ADAPTERS-001`: Owner `State 5 proof-writer plus State 6 contract-verification reviewer`; reason `no concrete adapter exec functions exist at State 3, and creating proof-only models would violate no-vacuum Verus`; limitation `does not prove Rust-local comparator correctness`; expiry/follow-up `expires when State 5/implementation introduces compare_observed_runs, normalize_error, or event-sequence adapters, or before State 6 approval if those adapters already exist`; compensating evidence `focused BDD assertions, single-field-difference proptest/focused cases, TLA+ temporal refinement for sequence behavior, and static review forbidding debug-string equality`.

## TLA+-Owned Clauses

- `PRE-004`, `POST-003`, `POST-004`, `POST-005`, `INV-004`, `INV-005`, `INV-006`.
- Required model shape: separate IR and generated transition relations, explicit observation/refinement predicate, legal StepState transition relation for both machines, bounded event/step sequences with explicit overflow/error transitions, reachable external resume inputs, reachable source-emission/reject alternatives, and divergence sanity config that would fail if generated observations differ from IR.

## Theorem-Owned Clauses

- None for this repair. Lean/Aeneas/Hax remains waived because Verus/TLA+ own the needed proof surfaces.

## Non-goals

- No production implementation, tests, proof code, model code, emitted source, benchmarks, or proof evidence in State 3.
- No maxperf, PGO, generated-vs-IR speed claim, public generated-mode readiness, or current release-gate activation.
- No whole-fleet verification or broad mutation/Kani run.
