# Contract Specification: vb-gvmt Generated Rust Semantic Parity

## Context
- Feature: finish generated Rust semantic parity in `vb_codegen` for the currently emitted subset of `CompiledWorkflow` IR.
- Authoritative source: `/velvet-ballistics-MASTER.md`; cited constraints include generated mode preserving IR semantics exactly for step states, slot writes, taint behavior, suspension semantics, journal events, typed errors, and result values (lines 656-658), action completion/journal/taint rules (lines 954-958, 972-983), and generated parity requirements (lines 1098-1106).
- Existing implementation fact: `crates/vb_codegen/src/lib.rs::compare_generated_to_ir` currently checks source patterns and counts, not execution equivalence over terminal result, taint, journal, and typed errors.
- Assumption A1: because `CompiledWorkflow` does not expose a complete action contract table, generated `Do` without a verified contract table must conservatively follow runtime `execute_do_without_contract` semantics.
- Assumption A2: exact Rust test/harness paths may be chosen by implementation agents, but observable values below are contractual and must not be weakened.
- Evidence update E1: TLA+, Verus, Kani, and POST-011 executable semantic parity artifacts now exist under `.beads/vb-gvmt/` and `crates/vb_codegen/src/tests.rs`; mutation adequacy remains deferred because the scoped cargo-mutants run produced only unviable mutants.

## Domain Terms
- Generated runner: standalone Rust emitted by `emit_rust_workflow`.
- IR runner: runtime/IR interpreter behavior that generated code must refine.
- Observable semantics: terminal `StepOutcome`/`EngineSignal`, result `SlotValue`, result `Taint`, typed error variant and fields, journal event sequence/signatures, slot values, slot taints, and step-budget behavior.
- Clean-required taint enforcement: deterministic pure action paths without a validator-proven declassification contract must reject clean output produced from tainted input.
- Resume payload: action/ask completion data. The current generated API validates generated-local identity (`step`, `action_id`, `output_slot`, `ask_step`, `resume_step`, and pending resume state) before mutation; outer runtime/run routing owns any broader run id or ticket mapping not present in the generated API.

## Preconditions
- PRE-001: The input `CompiledWorkflow` passed to generated emission has already passed normal compile/validation for supported generated subset nodes.
- PRE-002: Every emitted generated runner owns finite capacities for slots, taints, journal buffer, expression stack, list/object stores, and step budget before execution.
- PRE-003: Generated code must never use unsafe, unwrap, expect, panic, unchecked indexing/slicing/casts/arithmetic, JSON, runtime YAML, HTTP, or runtime string reference resolution.
- PRE-004: For `Do` nodes, if no action contract table is available to generated code, the action is treated as no-contract conservative mode for taint/result validation.
- PRE-005: Resume input for action/ask must include enough generated-local identity to validate step/action-or-ask/output-or-answer target/resume target before mutating frame state. Run id and external ticket identity are outside the current generated API boundary.

## Postconditions
- POST-001: A generated run and its IR run produce identical terminal outcome: same `Finished` vs typed error/suspension, same result `SlotValue`, and same result `Taint`.
- POST-002: `Finish` reads the result slot and result-slot taint; generated terminal success preserves `Clean`, `DerivedFromSecret`, and `Secret` exactly.
- POST-003: `Do` suspension appends/emits `ActionScheduled` at suspension with step, action id, input slot, and resume pc equivalent to runtime for the generated-local observation boundary.
- POST-004: `Do` resume validates generated-local completion identity before mutation, writes the target output slot with output value and taint, emits `SlotWritten`, then emits `ActionCompleted`, then advances to the next step equivalent to IR.
- POST-005: `AskResume` validates generated-local answer identity before mutation, writes the answer slot with exact answer value and taint, emits `AskAnswered` if the runtime journal model has that event, emits `SlotWritten`, then advances exactly as IR.
- POST-006: Runtime-equivalent generated journal ordering is deterministic: deterministic slot writes emit `SlotWritten`; action suspension emits `ActionScheduled`; action resume emits `SlotWritten` and `ActionCompleted`; successful finish emits `RunFinished`; ask answer emits `AskAnswered` if defined by runtime and never silently drops answer-slot mutation.
- POST-007: DeterministicPure/no-contract `Do` with tainted input and clean output returns `DriveError::TaintViolation { step }` (or the runtime-equivalent typed error if renamed) before the clean output becomes observable.
- POST-008: Step-budget exhaustion returns `DriveError::StepBudgetExhausted` with no extra step side effects beyond those produced before the exhausted step.
- POST-009: Generated equivalence checks execute generated and IR runners over the same fixture and compare typed observations, not source-pattern counts.

## Invariants
- INV-001: Slot value and taint arrays are parallel; every successful slot write writes both value and taint or preserves a documented existing taint rule identical to IR.
- INV-002: No unchecked slot/constant/expression/journal access exists in generated execution; out-of-bounds accesses return typed errors.
- INV-003: Journal capacity is bounded; when capacity is exhausted, generated execution returns a typed capacity error before dropping, reordering, or partially recording required events.
- INV-004: Taint join is monotonic: `Secret >= DerivedFromSecret >= Clean`; generated action/result taint is never less restrictive than runtime under the same inputs and contracts.
- INV-005: Resume transition is single-use and generated-local identity checked; wrong step, wrong action/ask, wrong output slot/resume step, missing pending state, or duplicate conflicting completion fails with typed error before frame mutation. Wrong run/ticket validation is an outer runtime routing concern until generated APIs carry those fields.
- INV-006: Generated and IR traces are observationally equivalent after normalizing non-semantic implementation detail; journal event kind/order and semantic fields must match.
- INV-007: Unsupported final IR primitives remain rejected with typed unsupported errors until parity is proven; this bead does not silently broaden unsupported primitive semantics.

## Error Taxonomy
- ERR-001: `DriveError::SlotOutOfBounds { slot }` for invalid slot reads/writes.
- ERR-002: `DriveError::ExprOutOfBounds { expr }` for invalid expression reads.
- ERR-003: `DriveError::StepBudgetExhausted` for budget exhaustion.
- ERR-004: `DriveError::TaintViolation { step }` for no-contract deterministic action clean output from tainted input.
- ERR-005: `DriveError::ActionSuspend { step, action_id, input_slot, resume_pc }` for action suspension, paired with `ActionScheduled` journal emission.
- ERR-006: `DriveError::AskSuspend { step, prompt_slot, timeout_slot, resume_pc }` for ask suspension, paired with ask scheduling state.
- ERR-007: `DriveError::InvalidResume { step }` or runtime-equivalent typed error for malformed/wrong generated-local action/ask resume identity.
- ERR-008: `DriveError::JournalCapacityExceeded { needed, capacity }` or runtime-equivalent typed error for bounded journal overflow; exact variant may require implementation if absent.
- ERR-009: `DriveError::UnsupportedPrimitive { primitive }` for unsupported final IR primitives in generated mode.

## Contract Signatures (desired proof/test surface, not implementation)
- `emit_rust_workflow(workflow: &CompiledWorkflow) -> Result<String, CodegenError>` preserves subset validation and emits only safe generated source.
- Semantic parity must be driven by executable generated-vs-IR/runtime tests or a future comparator that runs both sides; the current `compare_generated_to_ir(source, workflow)` remains a static source-pattern/count guard and is not itself semantic evidence.
- Generated runner contract shape: `drive(frame, resume_payload, step_budget, journal) -> Result<StepOutcome, DriveError>` where all fallible paths are typed `Result` and all observable state is comparable to IR.
- Generated action resume contract shape: `complete_action(step, action_id, output_slot, output_value, output_taint) -> Result<GeneratedRunStatus, DriveError>` validates generated-local identity before mutation.
- Generated ask resume contract shape: `answer_ask(ask_step, resume_step, answer_value, answer_taint) -> Result<GeneratedRunStatus, DriveError>` validates generated-local identity before mutation.

## Verus-Owned Clauses
- INV-001, INV-002, INV-003, INV-004, INV-005: smallest Verus surface is a pure abstract generated state transition model over bounded arrays, taint lattice, journal capacity, and resume payload identity. Implemented at `.beads/vb-gvmt/proofs/generated_semantics_verus.rs`; evidence command is `/home/lewis/.local/bin/verus --crate-type=lib .beads/vb-gvmt/proofs/generated_semantics_verus.rs`.

## TLA+-Owned Clauses
- POST-003 through POST-006 and INV-006: temporal lifecycle of generated run, valid action suspension/resume, valid ask suspension/resume, journal ordering, and terminal completion. The current TLA+ evidence does not claim INV-005 invalid-resume preservation. Implemented at `.beads/vb-gvmt/specs/GeneratedParity.tla` with `.beads/vb-gvmt/specs/GeneratedParity.cfg`; evidence command is `tlc -config .beads/vb-gvmt/specs/GeneratedParity.cfg .beads/vb-gvmt/specs/GeneratedParity.tla`.

## Theorem-Owned Clauses
- None required now. The taint lattice is small enough for Verus; Lean is a waiver candidate only if Verus cannot express the lattice/refinement relation after implementation.

## Non-goals
- The original contract-authoring step did not itself require production implementation; implementation, tests, and proof evidence are now linked through `proof-obligations.jsonl`, the reports in this bead directory, and the committed source changes.
- No full final-IR primitive expansion beyond currently emitted/generated subset unless separate beads add parity for rejected primitives.
- No performance speedup claim; performance evidence is limited to no hidden dynamic allocation/static scan unless a later bead asserts speed ratios.
