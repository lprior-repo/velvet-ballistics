# TLA+ Temporal Model Plan - vb-qi37.2.5

## Boundary
- Temporal/workflow behavior: bounded deterministic execution slice, budget exhaustion, blocked/finished transitions, and admission/rejection of nested composition before runtime.
- Rust/core behavior excluded from TLA+: arithmetic lemmas for saturation/monotonicity (`verification/verus/resource_budget.rs`) and step budget underflow freedom (`verification/verus/step_budget.rs`).
- External systems abstracted: action handlers, storage, wall-clock, generated runtime chunks, and OS process memory.
- Non-applicability rationale: not applicable; this bead contains temporal lifecycle behavior and requires a TLA+ model obligation.

## TLA+-Owned Clauses
- INV-002, POST-001 -> planned module `BoundednessSlice` for execution slice budget consumption and `StepBudgetExhausted` eventuality.
- POST-006, INV-006 -> planned module `NestedBoundednessAdmission` for finite nested workflow admission/rejection.

## Model Shape
- Module/model paths: `specs/vb_qi37_2_5/BoundednessSlice.tla` with config `specs/vb_qi37_2_5/BoundednessSlice.cfg`; `specs/vb_qi37_2_5/NestedBoundednessAdmission.tla` with config `specs/vb_qi37_2_5/NestedBoundednessAdmission.cfg`.
- Variables: `pc`, `remaining`, `signal`, `workflow_state`, `computed_budget`, `policy`, `store_count`, `max_store_count`, `diagnostic`.
- Init actions: `InitSlice`, `InitAdmission`.
- Next/actions: `TakeStep`, `BlockOnAction`, `BlockOnWait`, `Finish`, `ExhaustBudget`, `ComputeBudget`, `RejectOverLimit`, `AcceptWithinLimit`, `InsertValue`, `RejectValueGrowth`.
- State constraints: finite workflows, finite budget dimensions, finite store cap, finite fanout/repeat/gather bounds for TLC.
- Symmetry sets: optional symmetry over branch identifiers and value kinds if proof-writer chooses bounded branch/value domains.
- Bounded model limits: at least budgets `0..3`, fanout `0..3`, nesting `0..3`, store cap `0..3`, and one exceeding value for each limit.

## Properties
- Safety invariants:
  - `BudgetNeverNegative`: remaining budget is never negative.
  - `NoTransitionAfterExhaust`: once exhausted, no deterministic transition consumes further budget.
  - `StoreCountWithinCap`: capped store count never exceeds configured cap.
  - `RejectsOverPolicy`: over-policy budget cannot reach accepted state.
  - `TypedTerminalOutcome`: every terminal adversarial path has a typed signal or diagnostic.
- Liveness/eventuality:
  - `EventuallyBlockedFinishedOrExhausted`: under weak fairness, every finite enabled execution slice eventually blocks, finishes, errors, or exhausts budget.
  - `EventuallyAcceptOrRejectAdmission`: every finite computed budget eventually reaches accept or typed reject.
- Fairness assumptions: weak fairness on `TakeStep` while budget remains and node can execute; weak fairness on `ComputeBudget` for finite admitted inputs; no fairness assumed for external action completion.
- Deadlock freedom: model must report no deadlock except explicit terminal states (`Finished`, `Blocked`, `Exhausted`, `Rejected`, `Accepted`).
- Refinement to Rust/runtime behavior: `remaining` refines `StepBudget::remaining`; `ExhaustBudget` refines `EngineSignal::StepBudgetExhausted`; `RejectValueGrowth` refines `CoreError::BudgetExceeded { budget: "max_slots" }`; admission reject actions refine `BudgetError`/verifier diagnostics.

## Evidence Command
- `tlc -metadir /tmp/opencode/tlc-vb-qi37-2-5-slice specs/vb_qi37_2_5/BoundednessSlice.tla -config specs/vb_qi37_2_5/BoundednessSlice.cfg`
- `tlc -metadir /tmp/opencode/tlc-vb-qi37-2-5-nested specs/vb_qi37_2_5/NestedBoundednessAdmission.tla -config specs/vb_qi37_2_5/NestedBoundednessAdmission.cfg`
- Expected evidence: TLC reports `Model checking completed. No error has been found.` for both configs, with no invariant violations, no unexpected deadlock, and temporal properties checked over the complete finite state spaces.

## Waivers
- No TLA+ waiver. Model files are blocked on proof-writer scope, not waived.
