# TLA+ Temporal Model Plan: vb-qi37.10

## Boundary

- Temporal behavior: bounded parity between generated execution and IR execution over the same finite workflow, finite slots, finite steps, finite journal capacity, finite generated stores, finite action/wait/ask/retry tickets, and finite step budget.
- Rust/core behavior excluded from TLA+: source emission syntax, Rust borrow checking, concrete value representation, exact Rust error structs, and helper implementation details. These belong to Verus/Kani/proptest/executable tests.
- External systems abstracted: action execution, timers, ask answers, Fjall persistence, and replay hydration. They are represented only as finite observable events/tickets for journal-signature parity.
- Non-applicability: none. This bead contains state-over-time execution and journal-order behavior, so TLA+ is applicable.

## TLA+-Owned Clauses

- INV-001: generated final-IR acceptance is fail-closed.
- POST-001: accepted generated execution matches IR oracle outcomes.
- POST-002: final IR support/rejection is explicit for all node families.
- POST-008 / INV-005: journal-signature order and typed Err states are preserved.

## Model Shape

- Planned module/model path: `verification/tla/VbQi3710GeneratedParity.tla` and `verification/tla/VbQi3710GeneratedParity.cfg` if State 4 proof-writing creates proof artifacts. Until then, this file is the State 3 sketch.
- Variables:
  - `mode \in {"IR", "GEN"}`
  - `pc[mode] \in Step`
  - `state[mode] \in [Step -> StepState]`
  - `slots[mode] \in [Slot -> SlotValueOrNone]`
  - `taint[mode] \in [Slot -> Taint]`
  - `journal[mode] \in Seq(JournalSig)` with bounded length
  - `budget[mode] \in Budget`
  - `stores[mode] \in GeneratedStoreState`
  - `outcome[mode] \in Outcome`
  - `accepted[mode] \in BOOLEAN`
  - `err[mode] \in ErrState`
- Finite sets:
  - `Step == 0..MAX_STEPS_MINUS_ONE`
  - `Slot == 0..MAX_SLOTS_MINUS_ONE`
  - `SeqNo == 0..MAX_JOURNAL_EVENTS_MINUS_ONE`
  - `Budget == 0..MAX_BUDGET`
  - `StoreLen == 0..MAX_STORE_ITEMS`
  - `NodeKind == {Nop, SetConst, Copy, EvalExpr, BuildObject, BuildList, Do, Choose, ChooseSlot, ForEachStart, ForEachNext, ForEachJoin, TogetherStart, TogetherBranch, TogetherJoin, CollectStart, CollectPage, CollectNext, CollectFinish, ReduceStart, ReduceNext, ReduceFinish, RepeatStart, RepeatAttempt, RepeatCheck, RepeatFinish, WaitUntil, WaitEvent, Ask, AskResume, RetryCheck, ErrorHandler, Jump, Finish}`
  - `ErrState == {NoErr, UnsupportedNode, UnsupportedExpr, StoreCapacityExceeded, IndexOutOfBounds, ArithmeticOverflow, InvalidStateTransition, MissingNextStep, RuntimeTypedErr, JournalCapacityExceeded}`
  - No unbounded `Nat` is used for operational counters; all counters live in finite bounded sets.
- Init action: `InitParity` initializes identical IR/GEN frames for an accepted supported workflow or sets `GEN` to `RejectedAtCodegen` with a typed unsupported error for rejected features.
- Next/actions:
  - `ValidateGenSubset`
  - `IrStep`
  - `GenStep`
  - `AppendJournalSig`
  - `StoreInsert`
  - `StoreLookup`
  - `SuspendActionWaitAsk`
  - `BudgetExhaust`
  - `TypedErr`
  - `Finish`
- State constraints:
  - `MAX_STEPS_MINUS_ONE <= 5` for initial TLC model.
  - `MAX_SLOTS_MINUS_ONE <= 5`.
  - `MAX_JOURNAL_EVENTS_MINUS_ONE <= 12`.
  - `MAX_STORE_ITEMS <= 6`.
  - `MAX_BUDGET <= 8`.
  - `WorkflowShape` is a finite fixture set covering one representative accepted/rejected shape per final IR family, not an arbitrary unbounded workflow generator.
- Symmetry sets: values may be symmetry-reduced by abstract value IDs when taint and value kind are equal. Step and slot IDs are not symmetric when the workflow shape assigns semantic roles.
- Bounded hardware arithmetic: `SeqNo`, `Budget`, store lengths, step indices, slot indices, and attempt/page counters are finite bounded model values. Increment at maximum transitions to a typed Err state such as `ArithmeticOverflow` or `JournalCapacityExceeded`, never to wraparound.

## Properties

- Safety invariants:
  - `FailClosedAcceptance`: if `accepted["GEN"] = TRUE`, every node/expression in workflow is in the generated support set.
  - `UnsupportedIsTyped`: if generated rejects a feature, `outcome["GEN"] = RejectedAtCodegen` and `err["GEN"] \in {UnsupportedNode, UnsupportedExpr}`.
  - `OutcomeParity`: if generated accepts and both modes are terminal/suspended/budget-exhausted/failed, then observable outcome tags and typed Err states match.
  - `PcParity`: accepted generated mode has the same final pc as IR at comparable stable points.
  - `SlotParity`: accepted generated mode has identical abstract slot values at comparable stable points.
  - `TaintParity`: accepted generated mode has identical slot and result taints.
  - `StepStateParity`: accepted generated mode has identical step states.
  - `JournalSigParity`: accepted generated mode has identical journal signature sequence.
  - `BoundedStores`: store lengths remain within `0..MAX_STORE_ITEMS`; overflow goes to typed Err.
  - `NoCounterWrap`: increments at max produce typed Err, not wraparound.
- Liveness/eventuality:
  - For finite deterministic accepted workflows with positive budget and no external suspension, both modes eventually reach `Finished` or the same typed `Failed` state.
  - For action/wait/ask suspension fixtures, both modes eventually reach matching `Suspended` when the external event is not supplied.
- Fairness assumptions:
  - Weak fairness on `IrStep` and `GenStep` while enabled and budget remains.
  - No fairness assumption for external action/timer/ask completion; absence of external completion is modeled as stable suspension, not deadlock.
- Deadlock freedom:
  - No deadlock for accepted deterministic workflows within finite budget unless both modes are in terminal, suspended, budget-exhausted, or typed Err states.
- Refinement to Rust/runtime behavior:
  - `IR` actions refine `vb_runtime::engine::execute::execute_node_full` and primitive modules.
  - `GEN` actions refine code emitted by `vb_codegen::emit_rust_workflow` after `validate_generated_subset` succeeds.
  - `JournalSig` refines normalized event observations from generated and runtime harnesses, not byte-level `vb_storage` envelopes.

## Evidence Command

- Blocked until State 4 creates executable TLA+ files. Planned command after proof artifacts exist: `tlc -config verification/tla/VbQi3710GeneratedParity.cfg verification/tla/VbQi3710GeneratedParity.tla`.
- If the repository standardizes a Moon proof lane before State 4, the proof obligation may be run through `moon run :verify-proof`, but the exact target must point to the created TLA+ module.

## Waivers

- None for bounded temporal parity. Full crash recovery replay/hydration temporal proof is out of scope and owned by Phase 33/44, not waived here.
