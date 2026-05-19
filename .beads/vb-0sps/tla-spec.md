# TLA+ Temporal Model Plan - vb-0sps State 3 Repair

## Boundary

- Temporal/workflow behavior: generated-vs-IR parity over finite scenario kernels for success, suspension/resume, typed error, and unsupported reject.
- Required non-vacuity: IR oracle and generated candidate have separate transition relations. Parity is proven through explicit observation/refinement predicates, not by assigning identical state in one action.
- Rust/core behavior excluded from TLA+ and handled by BDD/proptest/Verus waiver: concrete Rust value equality code, adapter implementation, generated source internals, compiler process, and memory safety.
- External systems abstracted: action dispatcher, timer wheel, ask responder, journal sink, and value store are bounded symbolic inputs/events.

## TLA+-Owned Clauses

- PRE-004: identical resume/external inputs are supplied to both machines.
- POST-003: suspension metadata parity.
- POST-004: resume parity and eventual progress under supplied resume input.
- POST-005: journal/event sequence parity over all contracted fields.
- INV-004: legal StepState transitions for both machines and terminal states do not reopen.
- INV-005: suspension boundary prevents PC/post-boundary writes before resume.
- INV-006: unsupported features reject before source acceptance/emission/compile/run.

## Required Model Shape

- Module path: `verification/tla/generated_ir_parity/GeneratedIrParity.tla`.
- Config paths to be authored/repaired by State 5:
  - `verification/tla/generated_ir_parity/GeneratedIrParity_success.cfg`
  - `verification/tla/generated_ir_parity/GeneratedIrParity_suspension_resume.cfg`
  - `verification/tla/generated_ir_parity/GeneratedIrParity_typed_error.cfg`
  - `verification/tla/generated_ir_parity/GeneratedIrParity_unsupported_reject.cfg`
  - `verification/tla/generated_ir_parity/GeneratedIrParity_divergence_sanity.cfg`
- Variables: `ir_pc`, `gen_pc`, `ir_slots`, `gen_slots`, `ir_taints`, `gen_taints`, `ir_steps`, `gen_steps`, `ir_journal`, `gen_journal`, `ir_blocked`, `gen_blocked`, `resumeQueue`, `ir_terminal`, `gen_terminal`, `ir_error`, `gen_error`, `unsupported`, `sourceEmitted`, plus any explicit candidate-fault/config selector variables used by the repaired model.
- Init action: `Init` or `InitSameInputs`, establishing PRE-003/PRE-004 equality for all public initial observations and resume inputs.
- Required separate actions/relations:
  - IR side: `IrStep`, `IrBlockAction`, `IrBlockWait`, `IrBlockAsk`, `IrResumeAction`, `IrResumeAsk`, `IrTimerFire`, `IrRecordEvent`, `IrFinish`, `IrError`.
  - Generated side: `GenStep`, `GenBlockAction`, `GenBlockWait`, `GenBlockAsk`, `GenResumeAction`, `GenResumeAsk`, `GenTimerFire`, `GenRecordEvent`, `GenFinish`, `GenError`, `GenUnsupportedReject`, `GenSourceAcceptOrEmit`.
  - Environment: `EnvSupplyActionCompletion`, `EnvSupplyAskAnswer`, `EnvSupplyTimer`, `EnvBudgetTick` or equivalent reachable bounded input population.
  - Combined next: `Next` may schedule pairs of IR/generated actions but must not update both sides by construction without applying their separate relations.
- State constraints: finite steps, slots, events, values, taints, error classes, tickets, retries, and bounded counters; sequence lengths must be constrained by `MaxStep`/`MaxEvent`; overflow must transition to explicit typed error or bounded reject state rather than append indefinitely.
- Symmetry sets: value/action identifiers may be symmetry-reduced only when field identity needed for mismatch diagnostics is preserved.

## Bounds and Tractability Contract

- Split configs are required so TLC completes. Do not use one monolithic config if it times out.
- Minimum accepted scenario-kernel bounds for this bead are intentionally small and finite: `MaxStep >= 2`, `MaxSlot >= 2`, `MaxEvent >= 4`, at least two taints, one action id, one retry, one ticket, and finite typed error classes.
- These bounds prove only the contracted BDD scenario kernels. They must not be described as generalized release/maxperf proof.
- If a config uses smaller bounds than the minimum above, it needs a waiver with owner, reason, limitation, expiry/follow-up, and compensating evidence.

## Properties

- Safety invariants:
  - `ObservationRefinesOracle` or equivalent: generated public observation equals/refines IR oracle observation at comparison points.
  - `SameObservableStateWhenTerminal`: terminal status, result, taint, PC, slots, taints, and step states match.
  - `SameBlockedMetadata`: blocked kind and metadata match.
  - `SameJournalPrefix`: journals match after every comparable transition, including all POST-005 fields.
  - `NoAdvancePastSuspension`: PC and post-boundary writes do not advance while blocked.
  - `UnsupportedNoSourceEmission`: unsupported reject implies no generated source acceptance/emission/compile/run.
  - `ValidStepStateTransitions`: both `ir_steps` and `gen_steps` obey the legal bounded StepState relation and terminal states do not reopen.
- Liveness/eventuality:
  - `EventuallyTerminalOrBlockedOrTypedError`: under weak fairness and finite budget, both modes reach matching terminal, matching blocked, matching typed error, or unsupported reject.
  - `ResumeEventuallyProgresses`: if matching resume input is supplied and transition is enabled, both modes eventually leave blocked state or return matching typed error.
- Fairness assumptions: weak fairness on enabled machine and resume actions; no fairness for absent external input.
- Deadlock freedom: no deadlock except explicit terminal, blocked-without-resume, typed error, unsupported reject, or bounded overflow/error states.
- Negative sanity: the divergence sanity config must make TLC find an invariant violation when a candidate fault is enabled, proving the refinement relation can fail.

## Evidence Commands Required from State 5

- `timeout 120 tlc -config verification/tla/generated_ir_parity/GeneratedIrParity_success.cfg -workers 1 verification/tla/generated_ir_parity/GeneratedIrParity.tla`
- `timeout 120 tlc -config verification/tla/generated_ir_parity/GeneratedIrParity_suspension_resume.cfg -workers 1 verification/tla/generated_ir_parity/GeneratedIrParity.tla`
- `timeout 120 tlc -config verification/tla/generated_ir_parity/GeneratedIrParity_typed_error.cfg -workers 1 verification/tla/generated_ir_parity/GeneratedIrParity.tla`
- `timeout 120 tlc -config verification/tla/generated_ir_parity/GeneratedIrParity_unsupported_reject.cfg -workers 1 verification/tla/generated_ir_parity/GeneratedIrParity.tla`
- `timeout 120 tlc -config verification/tla/generated_ir_parity/GeneratedIrParity_divergence_sanity.cfg -workers 1 verification/tla/generated_ir_parity/GeneratedIrParity.tla`

Expected evidence: the first four commands exit `0` with no invariant/property/deadlock failures; the divergence sanity command exits non-zero because TLC finds the injected generated/IR mismatch. Timeouts are not proof evidence.

## Waivers

- No TLA+ waiver. If TLC cannot complete after split configs, State 5/6 must either repair the model further or produce a separately reviewed waiver. This State 3 repair does not approve a timeout waiver.
