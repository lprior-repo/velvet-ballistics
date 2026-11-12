# Contract: vb-qi37.2.1 - runtime: Define aggregate resource budget model

## 1. Scope

Define the contract for an aggregate resource budget model that lets `vb_core` compute and validate a whole-workflow resource requirement, and lets `vb_runtime` reject run admission when the requested aggregate budget cannot fit the configured runtime or shard capacity.

The model must preserve the current architecture split:

- `vb_core::budget` owns pure, deterministic budget value types, checked arithmetic, and policy validation.
- `vb_core::workflow` exposes compiled workflow resource facts through `CompiledWorkflow`, `ResourceContract`, and validated `WorkflowParts`.
- `vb_core::validation::resource` remains the structural validation layer for resource contracts and must not gain runtime dependencies.
- `vb_runtime::admission` performs admission decisions against artifact presence, capabilities, and aggregate capacity.
- `vb_runtime::shard::types` may carry capacity snapshots or reservation state, but must use core budget/domain types instead of inventing parallel budget dimensions.

## 2. Domain Terms

- Aggregate resource budget: a conservative whole-workflow requirement derived from `ResourceContract` plus compiled IR shape.
- Capacity: the runtime or shard-local resource ceiling available for admitted runs.
- Requested budget: the aggregate budget required by one submitted workflow run.
- Available budget: capacity not yet reserved by currently active/admitted runs.
- Reservation: a bounded, typed accounting decision that subtracts requested budget from available capacity before execution begins.
- Release: returning a reservation to available capacity when a run finishes, fails, is cancelled, or is rejected after partial admission.
- Dimension: a named resource field such as steps executable, action tickets, parallel in-flight actions, gather items, result bytes, total slots, queue depth, or journal bytes.

## 3. Assumptions

- The aggregate model builds on the existing `WholeWorkflowBudget` and `BoundednessPolicy` in `crates/vb_core/src/budget.rs`.
- A missing explicit aggregate budget means "compute from `CompiledWorkflow`"; it does not mean "unbounded".
- Runtime enforcement is per shard first because `RunId` routes to a shard; global runtime reporting may sum shard snapshots but must not duplicate admission logic.
- The first implementation may validate capacity at submit/tick admission time, not at cold artifact compilation time, provided `CompiledWorkflow::try_from_parts` still validates boundedness.
- Existing strict/journaled/relaxed artifact admission behavior remains intact; budget capacity checks are orthogonal and apply regardless of artifact policy unless an explicit test-only bypass is introduced behind a test-only feature.

## 4. Open Questions

- Should aggregate capacity be configurable only through `ShardConfig`, or also through a runtime-level policy distributed evenly across shards?
- Should `RunAdmission` store the exact granted aggregate budget for audit/journal replay, or should it store only digest/run/capabilities/policy and rely on recomputation?
- Should `max_step_budget_per_tick` contribute to aggregate capacity, or remain an execution throttle separate from admission capacity?
- Should result bytes and journal batch bytes be reserved pessimistically at admission or checked at write boundaries only?

## 5. Required Contract Types

The implementation must expose or preserve typed Rust data. Runtime core must not parse JSON, YAML, HTTP, or text command payloads for this model.

Recommended signatures, names may vary only if semantic parity is exact:

```rust
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
}

pub struct AggregateResourceUsage { /* same comparable dimensions as capacity */ }

pub struct AggregateReservation {
    pub run: vb_core::ids::RunId,
    pub requested: AggregateResourceBudget,
}
```

Fallible operations must be railway-oriented:

```rust
impl AggregateResourceBudget {
    pub fn from_workflow(workflow: &vb_core::workflow::CompiledWorkflow)
        -> Result<Self, AggregateBudgetError>;

    pub fn from_whole_workflow_budget(
        budget: vb_core::budget::WholeWorkflowBudget,
        contract: vb_core::workflow::ResourceContract,
    ) -> Result<Self, AggregateBudgetError>;
}

impl AggregateResourceUsage {
    pub fn try_add_budget(&self, budget: &AggregateResourceBudget)
        -> Result<Self, AggregateBudgetError>;

    pub fn try_subtract_budget(&self, budget: &AggregateResourceBudget)
        -> Result<Self, AggregateBudgetError>;

    pub fn fits_within(&self, capacity: &AggregateResourceCapacity)
        -> Result<(), AggregateBudgetError>;
}

pub fn validate_aggregate_budget(
    budget: &AggregateResourceBudget,
    policy: &vb_core::budget::BoundednessPolicy,
) -> Result<(), AggregateBudgetError>;

pub fn admit_run_with_budget(
    store: &dyn vb_runtime::admission::ArtifactStore,
    policy: vb_core::policy::RuntimePolicy,
    digest: vb_core::ids::WorkflowDigest,
    run_id: vb_core::ids::RunId,
    caps: vb_core::capability::CapabilitySet,
    requested: AggregateResourceBudget,
    available: AggregateResourceCapacity,
) -> Result<vb_runtime::admission::RunAdmission, vb_runtime::admission::AdmissionError>;
```

## 6. Preconditions

- `CompiledWorkflow` inputs must already be created by `CompiledWorkflow::try_from_parts`, not unchecked production constructors.
- `WorkflowParts.resource_contract` must structurally cover nodes, slots, constants, expressions, accessors, expression stack, fanout, and output bytes before aggregate computation is trusted.
- Entry and target `StepIdx` values must be valid; out-of-bounds or cyclic paths must be rejected before or during budget computation.
- Every aggregate dimension must have a finite integer bound. Unknown, missing, NaN, textual, or dynamic dimensions are invalid by construction.
- Capacity comparison must receive a fully initialized capacity snapshot. No default "infinite capacity" is allowed in production config.
- Runtime reservation must occur before a run frame is inserted into active shard state or before hot resources are allocated for the run.
- Every fallible constructor or operation must return `Result<T, Error>`; no panic path may encode precondition failures.

## 7. Postconditions

- Successful aggregate construction returns a budget whose dimensions are all known and finite.
- `requested <= available` for every capacity dimension admits the run for budget purposes.
- `requested > available` for any capacity dimension rejects the run with the exact failing dimension, requested value, and available/limit value.
- Checked summation failure returns an overflow error and never wraps, saturates silently, truncates, or casts lossy values.
- A successful reservation monotonically increases usage by exactly the requested dimensions.
- A successful release monotonically decreases usage by exactly the reserved dimensions and never underflows.
- Rejection leaves active run state, usage counters, frame pools, journals, and trace rings unchanged except for permitted cold diagnostic counters/events that are explicitly documented.
- `RunAdmission` remains an immutable record after creation; if extended with budget fields, getters must expose copies or references without mutation.

## 8. Invariants

- No accepted workflow has unknown bounds.
- `ResourceContract` limits apply within one workflow; `BoundednessPolicy` limits are absolute cross-workflow policy ceilings.
- Validation order is: structural `ResourceContract` validation, whole-workflow budget computation, boundedness policy validation, runtime capacity comparison, then reservation.
- Core budget logic has no dependency on runtime, storage, HTTP, JSON, YAML, or allocation-heavy config parsing.
- Runtime admission must use core aggregate types or lossless conversions from them; no parallel runtime-only dimension vocabulary may drift from core semantics.
- Capacity comparison is inclusive: equality admits, greater-than rejects.
- Every arithmetic operation that combines dimensions is checked.
- Release is idempotency-safe only when tied to an existing reservation; releasing an unknown reservation is a typed error.
- Shard-local active usage must never exceed shard-local capacity after any public admission, tick, cancel, finish, or shutdown path returns `Ok`.
- Test-only bypasses must be gated by existing test utilities and cannot affect production admission.

## 9. Error Taxonomy

Core errors should be represented as a new semantic enum or integrated into existing `BudgetError`/`WorkflowError` without losing information:

- `AggregateBudgetError::WorkflowBudget(WorkflowError)` - whole-workflow computation failed due to invalid entry, target, cycle, or structural workflow issue.
- `AggregateBudgetError::PolicyExceeded { resource: &'static str, actual: u64, limit: u64 }` - computed aggregate exceeds `BoundednessPolicy` or hard per-workflow policy.
- `AggregateBudgetError::CapacityExceeded { resource: &'static str, requested: u64, available: u64 }` - runtime/shard capacity cannot fit the requested budget.
- `AggregateBudgetError::Overflow { resource: &'static str }` - checked add/mul or widening conversion failed.
- `AggregateBudgetError::Underflow { resource: &'static str }` - release/subtraction would make usage negative.
- `AggregateBudgetError::InvalidCapacity { resource: &'static str }` - capacity snapshot contains zero for dimensions that must be non-zero or otherwise violates hard limits.
- `AggregateBudgetError::ReservationNotFound { run: RunId }` - release was requested for a run without an active reservation.
- `AdmissionError::ResourceCapacityExceeded { resource, requested, available }` - runtime-facing rejection variant, if errors remain in `vb_runtime::admission`.

Existing variants such as `AdmissionError::ArtifactNotFound` and `AdmissionError::CapabilityDenied` must remain unchanged and continue to identify their current rejection causes.

## 10. Acceptance Criteria

- `.beads/vb-qi37.2.1/contract.md` exists and is non-empty.
- The design specifies a single source of truth for aggregate budget semantics in `vb_core`.
- Capacity comparison semantics are explicit: `requested <= available` admits; `requested > available` rejects.
- Overflow and underflow behavior is explicit and typed.
- Every listed precondition, postcondition, and invariant has at least one scenario or proof obligation below.
- The contract forbids production `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing, unchecked slicing, unchecked casts, and unchecked arithmetic.
- The final implementation must pass `moon ci`; targeted Cargo tests are only local feedback.

## 11. Martin Fowler Given/When/Then Scenarios

### Scenario 1: aggregate budget is computed for a bounded workflow
Given a `CompiledWorkflow` built through `try_from_parts` with finite loop, fanout, retry, slot, and output limits
When `AggregateResourceBudget::from_workflow` is called
Then it returns `Ok(budget)`
And every budget dimension is finite
And `budget.max_result_bytes <= workflow.resource_contract().max_output_bytes`
And no runtime state is accessed.

### Scenario 2: equality with capacity admits
Given a requested aggregate budget
And an available capacity with exactly equal values for every comparable dimension
When admission compares requested budget to capacity
Then admission succeeds for budget purposes
And no `CapacityExceeded` error is returned.

### Scenario 3: exceeding capacity by one rejects
Given a requested aggregate budget whose `max_action_tickets` is one greater than available capacity
When runtime admission checks aggregate capacity
Then admission returns `ResourceCapacityExceeded` or `CapacityExceeded`
And the error identifies `max_action_tickets`, requested value, and available value
And the run is not inserted into active shard state.

### Scenario 4: checked addition overflow rejects deterministically
Given current aggregate usage near the integer maximum for a dimension
And a requested budget that would overflow that dimension when added
When `try_add_budget` is called
Then it returns `Overflow { resource }`
And usage remains unchanged.

### Scenario 5: release cannot underflow
Given current aggregate usage lower than the reservation being released
When `try_subtract_budget` is called
Then it returns `Underflow { resource }`
And usage remains unchanged.

### Scenario 6: missing explicit aggregate budget is computed, not treated as unbounded
Given a valid `CompiledWorkflow` without a persisted aggregate budget field
When runtime admission needs a requested budget
Then it computes the budget from core workflow data
And either admits with finite dimensions or rejects with a typed computation/policy error.

### Scenario 7: artifact checks still run under strict policy
Given strict runtime policy
And a requested budget that fits capacity
And an artifact digest absent from the artifact store
When `admit_run_with_budget` is called
Then admission returns `ArtifactNotFound`
And no budget reservation is retained.

### Scenario 8: budget rejection is independent of capability checks
Given a requested budget that exceeds capacity
And granted capabilities that would otherwise satisfy all required actions
When admission checks the run
Then admission rejects for resource capacity
And the error taxonomy does not mislabel the rejection as `CapabilityDenied`.

### Scenario 9: active usage never exceeds capacity after cancellation
Given a run admitted with a budget reservation
When the run is cancelled and the shard processes cancellation successfully
Then the reservation is released
And active usage is less than or equal to capacity for every dimension.

### Scenario 10: no dynamic config parsing enters runtime core
Given runtime capacity configuration is present as typed `ShardConfig` or typed capacity values
When runtime admission evaluates aggregate budget
Then it performs no JSON, YAML, HTTP, or string-command parsing in the runtime core.

## 12. Proof Obligations

- Unit proof: every aggregate constructor rejects overflow and policy violations with exact error variants.
- Unit proof: capacity equality succeeds and capacity greater-than succeeds for every dimension.
- Unit proof: capacity less-than by one fails for every dimension.
- Unit proof: usage add/subtract round-trips exactly for a valid reservation.
- Unit proof: release of a missing or oversized reservation fails without underflow.
- Integration proof: `CompiledWorkflow::try_from_parts` still rejects workflows whose computed `WholeWorkflowBudget` exceeds `BoundednessPolicy`.
- Runtime proof: admission rejection leaves `Shard::runs.len()` and active usage unchanged.
- Runtime proof: successful admission records enough information to release the exact reservation on finish/fail/cancel.
- Static proof: implementation contains no `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing, unchecked slicing, unchecked casts, or unchecked arithmetic.
- CI proof: final implementation runs `moon ci` successfully.

## 13. Out-of-Scope Boundaries

- Do not implement production code as part of this bead state.
- Do not design YAML, JSON, HTTP, or CLI parsing for aggregate budgets.
- Do not change authoring-language syntax.
- Do not add generated Rust lowering for the aggregate model in this bead.
- Do not introduce distributed/multi-server global capacity coordination.
- Do not make performance claims without benchmark evidence.
- Do not replace existing artifact/capability admission behavior except to compose budget checks with it.

## 14. Risk Notes

- Existing `budget.rs` uses some saturating conversions and `unwrap_or` fallback patterns; implementation must avoid silent saturation for aggregate capacity accounting unless explicitly retained for cold diagnostics and proven harmless.
- Runtime currently admits based on artifact presence and capabilities; adding capacity reservation risks partial-admission leaks unless reservation ordering and rollback are specified and tested.
- Per-shard capacity may surprise operators if runtime-level capacity is expected; documentation must name the scope of each capacity value.
- Duplicating budget types in runtime would create drift; adapters must be lossless and thin.
- Result bytes, journal batch bytes, and queue depth may be enforced at multiple layers; aggregate admission must not contradict write-boundary checks.
- Relaxed policy is useful for tests, but must not become an unbounded-resource bypass in production.
