# TLA+ Temporal Model Plan: Bounded Admission

## Boundary
- Temporal behavior: workflow admission must occur only after successful whole-workflow boundedness verification and aggregate capacity reservation.
- Rust/core behavior excluded and handled by Verus/Kani/proptest: arithmetic for sequential sums, branch maxima, loop multiplication, overflow rejection, and diagnostic construction.
- External systems abstracted: storage, YAML parser, runtime shard execution, and proof artifact filesystem.

## TLA+-Owned Clauses
- INV-001: No admitted run lacks prior aggregate reservation.
- INV-006: Runtime admission consumes only `AggregateResourceBudget` derived from verified `WholeWorkflowBudget`.
- POST-010: Accepted aggregate budgets are materialized before runtime admission.

## Model Shape
- Existing model path: `specs/tla/BoundedAdmission.tla`.
- Existing config path: `specs/tla/BoundedAdmission.cfg`.
- Module: `BoundedAdmission`.
- Variables: `admitted_runs`, `shard_runs`, `reserved_resources`, `pending_admission`.
- Required model repair before execution if current model is insufficient: add an explicit `verified_budget`/`budget_status` state so `AdmitRun` is enabled only for a request whose budget is verified and reserved.
- Init action: `Init`.
- Actions: `RequestAdmission`, `AdmitRun`, `RejectAdmission`, `RunCompleted`.
- State constraints: finite `RunId`, finite `ShardId`, bounded pending/admitted sets, bounded resource dimensions.
- Symmetry sets: `RunId`, `ShardId` may be symmetry sets if TLC config declares finite sets.
- Bounded model limits: at least two runs and two shards; include over-limit reservations and rejected requests.

## Properties
- Safety invariant: `NoRunAdmittedWithoutReservation`.
- Safety invariant to add/confirm: `NoRunAdmittedWithoutVerifiedBudget`.
- Safety invariant: `ShardCapacityBounded`.
- Safety invariant: no admitted run has zero or absent resource dimensions.
- Temporal property: every pending admission is eventually admitted or rejected under weak fairness for `AdmitRun`/`RejectAdmission` when enabled.
- Deadlock freedom: TLC must report no deadlock under bounded finite state constraints.
- Fairness: weak fairness on admission resolution actions; no fairness assumption for external request generation.
- Refinement: Rust `AggregateResourceBudget::from_workflow` and `validate_aggregate_budget` establish the abstract verified/reserved request consumed by runtime admission.

## Evidence Command
- `tlc -config specs/tla/BoundedAdmission.cfg specs/tla/BoundedAdmission.tla`
- Rollup lane after proof repair: `moon run :verify-proof`.

## Waivers
- None for admission ordering. The existing TLA+ model may need proof-writer repair, but the temporal obligation is required.

## Status / Evidence Summary
- Status: planned. Current artifact defines the required model boundary and likely repair; it does not write TLA+ code.
