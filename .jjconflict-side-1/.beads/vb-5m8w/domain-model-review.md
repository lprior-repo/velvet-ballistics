# Domain Model Review: Step Budget Exhaustion

## Decision
Budget exhaustion is a scheduler suspension/rescheduling outcome, not terminal workflow failure.

## Current Domain Alignment
- `StepBudget::new` clamps to `MAX_STEP_BUDGET = 10_000`.
- `try_take` returns false at zero rather than decrementing, underflowing, or panicking.
- Core and runtime signals carry explicit `StepBudgetExhausted` variants.
- Runtime shard lifecycle treats `StepBudgetExhausted` like continued work, preserving the run.

## State Lattice
- `Runnable`: can consume budget and execute a deterministic step.
- `RunningStep`: transient state after successful budget consumption and before step outcome.
- `SuspendedBudget`: budget exhausted before a step starts; resumable with fresh budget.
- `SuspendedExternal`: awaiting action, wait, or ask; resumable by matching external event.
- `Finished`: terminal success.
- `TypedError`: terminal explicit execution failure.
- `InvariantViolation`: model/proof sink only; unreachable in valid implementation.

## Illegal State Transitions
- `SuspendedBudget -> Finished` without a later consumed step.
- `SuspendedBudget -> TypedError` solely because budget reached zero.
- `SuspendedBudget -> Deleted/Lost/CleanupTerminal` solely because budget reached zero.
- `Budget == 0 -> RunningStep`.
- `Budget == 0 -> StepStarted` or `StepSucceeded` evidence.
- `try_take(0) -> budget = MAX_U64` or any wrapped value.
- `ExhaustBudget` modeled as terminal, as seen in legacy `BoundednessSlice.tla`.

## Refinement Boundary
- TLA+ owns temporal state movement across slices, suspension, resume, fairness, deadlock freedom, and evidence ordering.
- Verus/Kani own Rust-local arithmetic and no-panic/no-underflow properties.
- Runtime tests own concrete evidence emission and shard lifecycle mapping.

## Review Risks For Next State
- The new TLA+ model must not reuse the legacy terminal exhaustion transition.
- The model must include explicit invalid arithmetic/error states, not only bounded TLC constants.
- The model must prove state preservation on zero-budget exhaustion and distinguish it from action/wait/ask suspension.
