# Contract Specification: vb-qi37.2.1 — Aggregate Resource Budget Model

## Context

- **Feature:** Aggregate resource budget model for runtime admission and resource accounting.
- **Domain terms:**
  - Aggregate resource budget: conservative whole-workflow resource requirement derived from `ResourceContract` + compiled IR shape.
  - Capacity: runtime or shard-local resource ceiling for admitted runs.
  - Requested budget: aggregate budget required by one submitted workflow run.
  - Available budget: capacity not yet reserved by active/admitted runs.
  - Reservation: bounded, typed accounting decision that subtracts requested budget from available capacity before execution.
  - Release: returning reservation to available capacity when run finishes, fails, is cancelled, or is rejected after partial admission.
  - Dimension: named resource field (steps, action tickets, parallel in-flight, gather items, result bytes, slots, queue depth, journal bytes).
- **Assumptions:**
  - Builds on existing `WholeWorkflowBudget` and `BoundednessPolicy` in `crates/vb_core/src/budget.rs`.
  - Missing explicit aggregate budget means "compute from `CompiledWorkflow`"; does not mean "unbounded".
  - Runtime enforcement is per shard first; global runtime reporting may sum shard snapshots.
  - First implementation may validate capacity at submit/tick admission time.
  - Existing strict/journaled/relaxed artifact admission behavior remains intact.
- **Open questions:**
  - Should aggregate capacity be configurable only through `ShardConfig`, or also through runtime-level policy?
  - Should `RunAdmission` store exact granted aggregate budget for audit/journal replay?
  - Should `max_step_budget_per_tick` contribute to aggregate capacity or remain execution throttle?
  - Should result bytes and journal batch bytes be reserved pessimistically at admission?

## Preconditions

- PRE-001: `CompiledWorkflow` inputs must already be created by `CompiledWorkflow::try_from_parts`, not unchecked production constructors.
- PRE-002: `WorkflowParts.resource_contract` must structurally cover all required fields before aggregate computation.
- PRE-003: Entry and target `StepIdx` values must be valid; out-of-bounds or cyclic paths must be rejected.
- PRE-004: Every aggregate dimension must have a finite integer bound. Unknown, missing, NaN, textual, or dynamic dimensions are invalid by construction.
- PRE-005: Capacity comparison must receive a fully initialized capacity snapshot. No default "infinite capacity" in production config.
- PRE-006: Runtime reservation must occur before a run frame is inserted into active shard state.
- PRE-007: Every fallible constructor or operation must return `Result<T, Error>`; no panic path may encode precondition failures.

## Postconditions

- POST-001: Successful aggregate construction returns a budget whose all dimensions are known and finite.
- POST-002: `requested <= available` for every capacity dimension admits the run for budget purposes.
- POST-003: `requested > available` for any capacity dimension rejects the run with exact failing dimension, requested value, and available/limit value.
- POST-004: Checked summation failure returns `Overflow { resource }` and never wraps, saturates silently, truncates, or casts lossy values.
- POST-005: Checked subtraction failure returns `Underflow { resource }` and never wraps, saturates silently, or causes underflow.
- POST-006: A successful reservation monotonically increases usage by exactly the requested dimensions.
- POST-007: A successful release monotonically decreases usage by exactly the reserved dimensions and never underflows.
- POST-008: Rejection leaves active run state, usage counters, frame pools, journals, and trace rings unchanged.
- POST-009: `RunAdmission` remains immutable after creation; getters expose copies or references without mutation.

## Invariants

- INV-001: No accepted workflow has unknown bounds.
- INV-002: `ResourceContract` limits apply within one workflow; `BoundednessPolicy` limits are absolute cross-workflow policy ceilings.
- INV-003: Validation order: structural `ResourceContract` validation, whole-workflow budget computation, boundedness policy validation, runtime capacity comparison, then reservation.
- INV-004: Capacity comparison is inclusive: equality admits (requested == available is OK).
- INV-005: Every arithmetic operation is checked; no wrapping, saturating, or panicking arithmetic in budget operations.
- INV-006: Release is idempotent only with existing reservation; releasing non-existent reservation returns `ReservationNotFound`.
- INV-007: Active usage never exceeds shard-local capacity.
- INV-008: All 16 dimensions of `AggregateResourceBudget` are independent; overflow in one dimension does not affect another.

## Error Taxonomy

- `AggregateBudgetError::WorkflowBudget(WorkflowError)` — invalid entry/target/cycle in workflow IR.
- `AggregateBudgetError::PolicyExceeded { resource, actual, limit }` — budget dimension exceeds `BoundednessPolicy` absolute ceiling.
- `AggregateBudgetError::CapacityExceeded { resource, requested, available }` — requested budget exceeds shard available capacity.
- `AggregateBudgetError::Overflow { resource }` — checked addition overflow in dimension `resource`.
- `AggregateBudgetError::Underflow { resource }` — checked subtraction underflow in dimension `resource`.
- `AggregateBudgetError::InvalidCapacity { resource }` — zero capacity for production-required dimension.
- `AggregateBudgetError::ReservationNotFound { run }` — releasing unknown `RunId`.
- `AggregateBudgetError::StepCeilingExceeded { requested, limit }` — `max_step_budget_per_tick` is zero or exceeds `HARD_MAX_STEP_BUDGET_PER_TICK`.
- `AggregateBudgetError::PerTickCeilingExceeded { requested, limit }` — `max_transitions_per_tick` is zero or exceeds `HARD_MAX_TRANSITIONS_PER_TICK`.

## Contract Signatures

```rust
// Core budget type — 16-dimension conservative whole-workflow resource requirement.
pub struct AggregateResourceBudget {
    pub max_steps_executable: u32,
    pub max_action_tickets: u32,
    pub max_parallel_in_flight: u16,
    pub max_retries_per_action: u16,
    pub max_gather_pages: u32,
    pub max_gather_items: u32,
    pub max_for_each_iterations: u32,
    pub max_together_branches: u16,
    pub max_repeat_attempts: u16,
    pub max_run_time_seconds: u64,
    pub max_result_bytes: u32,
    pub max_total_slots_written: u32,
    pub max_queue_depth: u32,
    pub max_journal_batch_bytes: u32,
    pub max_step_budget_per_tick: u64,
    pub max_transitions_per_tick: u64,
}

pub struct AggregateResourceCapacity {
    pub max_steps_executable: u64,
    pub max_action_tickets: u64,
    pub max_parallel_in_flight: u32,
    pub max_gather_pages: u64,
    pub max_gather_items: u64,
    pub max_result_bytes: u64,
    pub max_total_slots_written: u64,
    pub max_active_runs: u64,
    pub max_queue_depth: u64,
    pub max_journal_batch_bytes: u64,
    pub max_step_budget_per_tick: u64,
    pub max_transitions_per_tick: u64,
}

pub struct AggregateResourceUsage {
    // Same 14 comparable dimensions as AggregateResourceCapacity,
    // plus max_step_budget_per_tick and max_transitions_per_tick.
}

pub struct AggregateReservation {
    pub run: RunId,
    pub requested: AggregateResourceBudget,
}

pub enum AggregateBudgetError {
    WorkflowBudget(WorkflowError),
    PolicyExceeded { resource: &'static str, actual: u64, limit: u64 },
    CapacityExceeded { resource: &'static str, requested: u64, available: u64 },
    Overflow { resource: &'static str },
    Underflow { resource: &'static str },
    InvalidCapacity { resource: &'static str },
    ReservationNotFound { run: RunId },
    StepCeilingExceeded { requested: u64, limit: u64 },
    PerTickCeilingExceeded { requested: u64, limit: u64 },
}

impl AggregateResourceBudget {
    pub fn from_workflow(workflow: &CompiledWorkflow) -> Result<Self, AggregateBudgetError>;
    pub fn from_whole_workflow_budget(budget: WholeWorkflowBudget, contract: ResourceContract) -> Result<Self, AggregateBudgetError>;
}

impl AggregateResourceUsage {
    pub fn try_add_budget(&self, budget: &AggregateResourceBudget) -> Result<Self, AggregateBudgetError>;
    pub fn try_subtract_budget(&self, budget: &AggregateResourceBudget) -> Result<Self, AggregateBudgetError>;
    pub fn fits_within(&self, capacity: &AggregateResourceCapacity) -> Result<(), AggregateBudgetError>;
}

pub fn validate_aggregate_budget(budget: &AggregateResourceBudget, policy: &BoundednessPolicy) -> Result<(), AggregateBudgetError>;
pub fn validate_step_ceilings(budget: &AggregateResourceBudget) -> Result<(), AggregateBudgetError>;
```

## Verus-Owned Clauses

- INV-005: All `add_dim`/`sub_dim` operations use `checked_add`/`checked_sub` and never wrap or panic.
- POST-003: `try_add_budget` returns `Ok(new_usage)` exactly when all dimension sums fit in u64; returns `Err(Overflow { resource })` for the first overflowing dimension.
- POST-004: `try_subtract_budget` returns `Ok(new_usage)` exactly when all dimension differences are non-negative; returns `Err(Underflow { resource })` for the first underflowing dimension.
- POST-006: `usage.try_add_budget(budget)?.try_subtract_budget(budget)? == usage` for all non-overflowing budgets.
- INV-004: `fits_within` admits when `usage <= capacity` for all dimensions; equality is admit.
- INV-001 + INV-003: `from_workflow` rejects invalid workflow IR, cyclic jumps, and unbounded dimensions.

## TLA+-Owned Clauses

- None for pure budget arithmetic. Budget model is entirely Rust-local with no temporal/state-over-time behavior. TLA+ verification is not required for arithmetic correctness — Verus + Kani + Lean cover all critical properties.

## Theorem-Owned Clauses

- THM-ADD-SAFETY: `try_add_budget` is equivalent to component-wise `checked_add`; no wrapping, no saturation.
- THM-SUB-SAFETY: `try_subtract_budget` is equivalent to component-wise `checked_sub`; no wrapping, no underflow.
- THM-FITS-INCLUSIVITY: `fits_within(capacity)` succeeds iff usage <= capacity for every dimension.
- THM-POLICY-EXACT: `validate_aggregate_budget(budget, policy)` succeeds iff every dimension <= corresponding policy limit.
- THM-ADD-SUB-ROUNDTRIP: add then subtract with same budget recovers original usage.
- THM-CONV-LOSSLESS: `from_whole_workflow_budget` is lossless when values fit target widths.

## Non-goals

- Runtime admission integration, artifact store trait dispatch, capability set operations, shard state mutation, reservation lifecycle, finish/fail/cancel/shutdown paths, Fjall/Mio integration.
- External action ABI, timer wheel, trace ring, frame pools, value store arenas.

## Blackhat Findings Addressed

- BH-BUD-01: u32 saturation — addressed by `validate_step_ceilings` with `HARD_MAX_STEP_BUDGET_PER_TICK` and `HARD_MAX_TRANSITIONS_PER_TICK` hard limits; overflow returns `StepCeilingExceeded` or `PerTickCeilingExceeded`.
- BH-BUD-02: `max_run_time_seconds` hardcoded to 0 — fixed in `from_whole_workflow_budget`; sourced from `WholeWorkflowBudget.max_run_time_seconds`.
- BH-BUD-03: information loss — all `from_whole_workflow_budget` conversions use exact integer narrowing; overflow returns `AggregateBudgetError::Overflow`.
- BH-BUD-06: saturating_add inconsistency — `add_dim` uses `checked_add` only; no `saturating_add` anywhere in budget arithmetic.
- BH-BUD-07: gather_items saturating — `gather_items` dimension uses `checked_add`/`checked_sub`; overflow returns `Overflow { "max_gather_items" }`.
