# Martin Fowler Test Plan: vb-qi37.2.1 — Aggregate Resource Budget Model

## Summary

Pure deterministic budget operations (checked arithmetic, capacity comparison, policy validation, dimension conversion) in `vb_core::budget`. Runtime admission and reservation lifecycle in `vb_runtime::admission` and `vb_runtime::shard`. Full Given-When-Then scenarios covering happy path, error path, edge cases, and contract verification.

## Happy Path Tests

### Scenario: budget computation returns exact finite dimensions for a bounded workflow
Given: a `CompiledWorkflow` built through `try_from_parts` with finite loop limits, fanout, retries, slot count, and output limits
When: `AggregateResourceBudget::from_workflow` is called
Then: it returns `Ok(budget)` with every dimension finite and equal to the workflow's compiled resource facts
And: `budget.max_result_bytes <= workflow.resource_contract().max_output_bytes`
And: no runtime state is accessed

### Scenario: equality with capacity admits the run
Given: a requested aggregate budget with `max_action_tickets = 5`
And: an available capacity with `max_action_tickets = 5` for every comparable dimension
When: `fits_within` is called
Then: it returns `Ok(())`
And: `admit_run_with_budget` admits the run

### Scenario: below-capacity budget admits the run
Given: a requested aggregate budget with every dimension one less than available capacity
And: artifact exists in store, capabilities are sufficient, strict policy
When: `admit_run_with_budget` is called
Then: it returns `Ok(RunAdmission { budget: Some(requested) })`
And: active usage increases by exactly the requested dimensions

### Scenario: checked addition produces exact sums with no overflow
Given: current aggregate usage `{ max_steps_executable: 10, max_action_tickets: 3, ... }`
And: a requested budget `{ max_steps_executable: 5, max_action_tickets: 2, ... }`
When: `try_add_budget` is called
Then: it returns `Ok({ max_steps_executable: 15, max_action_tickets: 5, ... })`
And: no dimension overflows or wraps

### Scenario: checked subtraction produces exact differences with no underflow
Given: current aggregate usage `{ max_steps_executable: 10, max_action_tickets: 3, ... }`
And: a releasing budget `{ max_steps_executable: 5, max_action_tickets: 2, ... }`
When: `try_subtract_budget` is called
Then: it returns `Ok({ max_steps_executable: 5, max_action_tickets: 1, ... })`
And: no dimension underflows or wraps

### Scenario: add then subtract roundtrip recovers original usage
Given: current aggregate usage `U`
And: a budget `B` where `U.try_add_budget(B)` succeeds
When: `U.try_add_budget(B).and_then(|U2| U2.try_subtract_budget(B))` is called
Then: the result is `Ok(U)` exactly

### Scenario: policy validation accepts budgets at and below policy limits
Given: a `BoundednessPolicy` with `absolute_max_steps_executable: 1_000_000`
And: a budget with `max_steps_executable: 1_000_000`
When: `validate_aggregate_budget` is called
Then: it returns `Ok(())`

### Scenario: capacity comparison accepts zero usage against valid capacity
Given: a capacity with all dimensions set to valid non-zero values
And: an aggregate usage with all dimensions set to zero
When: `fits_within` is called
Then: it returns `Ok(())`

### Scenario: admission admits with exact equality on all dimensions
Given: strict policy, present artifact, sufficient capabilities
And: requested budget exactly equals available capacity for every dimension
When: `admit_run_with_budget` is called
Then: it returns `Ok(RunAdmission)` with budget stored
And: active usage equals the requested dimensions exactly

### Scenario: run finish releases reservation and usage returns to pre-admission
Given: a run was admitted with budget `B` and active usage `U_before + B`
When: the run finishes successfully and the shard processes finish
Then: the reservation is released
And: active usage returns to `U_before`
And: `fits_within(capacity)` still returns `Ok(())`

### Scenario: run failure releases reservation
Given: a run was admitted with budget `B` and active usage `U_before + B`
When: the run fails and the shard processes failure
Then: the reservation is released
And: active usage returns to `U_before`

### Scenario: run cancellation releases reservation
Given: a run was admitted with budget `B` and active usage `U_before + B`
When: the run is cancelled and the shard processes cancellation
Then: the reservation is released
And: active usage returns to `U_before`

### Scenario: shutdown drains all active runs and releases all reservations
Given: a shard with multiple admitted runs and active usage `U_total`
When: shutdown is initiated
Then: all reservations are released
And: active usage returns to zero
And: `fits_within(capacity)` returns `Ok(())`

## Error Path Tests

### Scenario: missing workflow entry rejects with exact error
Given: a `CompiledWorkflow` with no nodes and entry `StepIdx(0)`
When: `AggregateResourceBudget::from_workflow` is called
Then: it returns `Err(AggregateBudgetError::WorkflowBudget(WorkflowError::EntryOutOfBounds { entry: StepIdx(0) }))`

### Scenario: out-of-bounds step target rejects with exact error
Given: a workflow fixture whose target is `StepIdx(9)` while `node_count == 2`
When: `AggregateResourceBudget::from_workflow` is called
Then: it returns `Err(AggregateBudgetError::WorkflowBudget(WorkflowError::StepOutOfBounds { step: StepIdx(9) }))`

### Scenario: jump cycle rejects with exact error
Given: a workflow with jump `StepIdx(1) -> StepIdx(0)` creating a traversal cycle
When: `AggregateResourceBudget::from_workflow` is called
Then: it returns `Err(AggregateBudgetError::WorkflowBudget(WorkflowError::JumpCycle { step: StepIdx(1), target: StepIdx(0) }))`

### Scenario: exceeding capacity by one rejects with exact dimension
Given: a requested budget with `max_action_tickets = 6`
And: available capacity with `max_action_tickets = 5`
When: `fits_within` is called
Then: it returns `Err(AggregateBudgetError::CapacityExceeded { resource: "max_action_tickets", requested: 6, available: 5 })`
And: the run is not admitted

### Scenario: addition overflow rejects with exact dimension and unchanged usage
Given: current usage with `max_steps_executable: u64::MAX`
And: a budget requesting `max_steps_executable: 1`
When: `try_add_budget` is called
Then: it returns `Err(AggregateBudgetError::Overflow { resource: "max_steps_executable" })`
And: the original usage is unchanged

### Scenario: subtraction underflow rejects with exact dimension and unchanged usage
Given: current usage with `max_steps_executable: 0`
And: a budget requesting `max_steps_executable: 1`
When: `try_subtract_budget` is called
Then: it returns `Err(AggregateBudgetError::Underflow { resource: "max_steps_executable" })`
And: the original usage is unchanged

### Scenario: budget exceeding policy limit rejects with exact dimension and values
Given: a budget with `max_steps_executable: 1_000_001`
And: a `BoundednessPolicy` with `absolute_max_steps_executable: 1_000_000`
When: `validate_aggregate_budget` is called
Then: it returns `Err(AggregateBudgetError::PolicyExceeded { resource: "max_steps_executable", actual: 1_000_001, limit: 1_000_000 })`

### Scenario: zero capacity for active runs rejects as invalid
Given: an `AggregateResourceCapacity` with `max_active_runs: 0`
When: production capacity validation runs
Then: it returns `Err(AggregateBudgetError::InvalidCapacity { resource: "max_active_runs" })`

### Scenario: admission rejects with missing artifact under strict policy
Given: strict runtime policy
And: the artifact digest is absent from the artifact store
And: the requested budget fits within available capacity
When: `admit_run_with_budget` is called
Then: it returns `Err(AdmissionError::ArtifactNotFound { digest })`
And: no reservation exists for the run
And: active usage is unchanged

### Scenario: admission rejects with missing capability
Given: the requested action requires `Capability::new("network.http", ActionId(9))`
And: the granted capability set does not include this capability
When: admission checks capabilities
Then: it returns `Err(AdmissionError::CapabilityDenied { action: ActionId(9), required, granted })`
And: no reservation exists for the run
And: active usage is unchanged

### Scenario: budget rejection leaves all shard state unchanged
Given: an isolated shard with snapshots of active runs, reservation map, active usage, frame-pool counts, journal length, and trace-ring length
And: a submit with a budget that exceeds capacity
When: the submit is processed
Then: all non-diagnostic snapshots are byte/value equal to pre-call
And: diagnostic counter changes are named in the assertion

### Scenario: release of unknown run returns ReservationNotFound
Given: a reservation state that does not contain `RunId::new(42)`
When: release is requested for `RunId::new(42)`
Then: it returns `Err(AggregateBudgetError::ReservationNotFound { run: RunId::new(42) })`
And: active usage is unchanged

### Scenario: double release returns ReservationNotFound on second call
Given: a reservation for `RunId::new(7)` that was already released once
When: release is requested again for `RunId::new(7)`
Then: it returns `Err(AggregateBudgetError::ReservationNotFound { run: RunId::new(7) })`
And: active usage remains at the post-first-release value

## Edge Case Tests

### Scenario: zero budget add returns same usage unchanged
Given: current usage with `max_steps_executable: 10`
And: a budget with `max_steps_executable: 0` and all other dimensions zero
When: `try_add_budget` is called
Then: it returns `Ok(original_usage)` exactly

### Scenario: zero budget subtract returns same usage unchanged
Given: current usage with `max_steps_executable: 10`
And: a budget with `max_steps_executable: 0` and all other dimensions zero
When: `try_subtract_budget` is called
Then: it returns `Ok(original_usage)` exactly

### Scenario: subtract with equal usage returns zero for all dimensions
Given: current usage `{ max_steps_executable: 5, max_action_tickets: 2, ... }`
And: a budget `{ max_steps_executable: 5, max_action_tickets: 2, ... }`
When: `try_subtract_budget` is called
Then: it returns `Ok(AggregateResourceUsage { max_steps_executable: 0, max_action_tickets: 0, ... })`

### Scenario: add with max boundary sum equals u64::MAX
Given: current usage with `max_steps_executable: u64::MAX - 1`
And: a budget with `max_steps_executable: 1`
When: `try_add_budget` is called
Then: it returns `Ok({ max_steps_executable: u64::MAX, ... })`

### Scenario: per-dimension overflow rejected independently
Given: current usage with `max_action_tickets: u64::MAX - 1` and `max_steps_executable: 0`
And: a budget with `max_action_tickets: 2` and `max_steps_executable: 1`
When: `try_add_budget` is called
Then: it returns `Err(AggregateBudgetError::Overflow { resource: "max_action_tickets" })`
And: `max_steps_executable` usage is unchanged

### Scenario: per-dimension underflow rejected independently
Given: current usage with `max_action_tickets: 1` and `max_steps_executable: 5`
And: a budget with `max_action_tickets: 2` and `max_steps_executable: 1`
When: `try_subtract_budget` is called
Then: it returns `Err(AggregateBudgetError::Underflow { resource: "max_action_tickets" })`
And: `max_steps_executable` usage is unchanged

### Scenario: aggregate budget with zero optional dimensions
Given: a `WholeWorkflowBudget` with `max_action_tickets: 0`, `max_gather_pages: 0`, `max_gather_items: 0`
And: a `ResourceContract` with valid minimum values
When: `AggregateResourceBudget::from_whole_workflow_budget` is called
Then: it returns `Ok` with those optional fields exactly `0`

### Scenario: budget at exact policy maximum passes validation
Given: a budget with every governed dimension at exactly the policy limit
When: `validate_aggregate_budget` is called
Then: it returns `Ok(())`

### Scenario: aggregate budget from minimal one-finish-step workflow
Given: a valid workflow with exactly one Finish step and no actions/fanout/gather/retry
When: `AggregateResourceBudget::from_workflow` is called
Then: it returns `Ok(budget)` with `max_steps_executable: 1` and all counters at their valid minima

## Contract Verification Tests

### Precondition Tests

- `test_precondition_valid_workflow_construct_produces_budget`
- `test_precondition_workflow_parts_resource_contract_covers_all_fields`
- `test_precondition_entry_step_idx_is_valid`
- `test_precondition_no_cyclic_jumps`
- `test_precondition_all_dimensions_finite`
- `test_precondition_capacity_snapshot_fully_initialized`
- `test_precondition_reservation_before_frame_insertion`
- `test_precondition_all_fallible_ops_return_result`

### Postcondition Tests

- `test_postcondition_successful_construction_returns_finite_exact_dimensions`
- `test_postcondition_requested_less_than_or_equal_to_available_admits`
- `test_postcondition_checked_addition_never_wraps`
- `test_postcondition_checked_subtraction_never_underflows`
- `test_postcondition_requested_greater_than_available_rejects_exactly`
- `test_postcondition_add_then_subtract_roundtrips`
- `test_postcondition_rejection_leaves_state_unchanged`
- `test_postcondition_run_admission_immutable_after_creation`

### Invariant Tests

- `test_invariant_no_accepted_workflow_has_unknown_bounds`
- `test_invariant_resource_contract_vs_boundedness_policy_scope_separation`
- `test_invariant_validation_order_structural_then_budget_then_policy_then_capacity_then_reservation`
- `test_invariant_capacity_comparison_inclusive_equality_admits`
- `test_invariant_every_arithmetic_operation_is_checked`
- `test_invariant_release_idempotent_only_with_existing_reservation`
- `test_invariant_active_usage_never_exceeds_shard_local_capacity`
- `test_invariant_test_only_bypasses_gated_by_test_utilities`

### Error Taxonomy Tests

- `test_error_workflow_budget_invalid_entry`
- `test_error_workflow_budget_invalid_target`
- `test_error_workflow_budget_cycle`
- `test_error_policy_exceeded_per_dimension`
- `test_error_capacity_exceeded_per_dimension`
- `test_error_overflow_per_dimension`
- `test_error_underflow_per_dimension`
- `test_error_invalid_capacity_zero_active_runs`
- `test_error_reservation_not_found_unknown_run`
- `test_error_resource_capacity_exceeded_runtime_admission`

### Static Governance Tests

- `test_static_no_unsafe_in_production`
- `test_static_no_unwrap_expect_panic_in_production`
- `test_static_no_unchecked_indexing_slicing_casts_arithmetic`
- `test_static_no_json_yaml_http_parsing_in_runtime_core`
- `test_static_no_forbidden_test_constructs_in_production`

## Given-When-Then Summary Table

| ID | Scenario | Given | When | Then |
|----|----------|-------|------|------|
| GWT-01 | budget computation finite | valid CompiledWorkflow | from_workflow called | Ok(budget) finite dims |
| GWT-02 | equality admits | requested == available | fits_within | Ok(()) |
| GWT-03 | below capacity admits | requested < available | admit_run_with_budget | Ok(RunAdmission) |
| GWT-04 | add no overflow | usage + budget fits | try_add_budget | Ok(exact sums) |
| GWT-05 | subtract no underflow | usage >= budget | try_subtract_budget | Ok(exact diffs) |
| GWT-06 | roundtrip | U.add(B).sub(B) | chain | Ok(U) |
| GWT-07 | policy equality | budget == policy limit | validate | Ok(()) |
| GWT-08 | zero usage fits | usage=0, capacity>0 | fits_within | Ok(()) |
| GWT-09 | exact admission | requested==capacity | admit | Ok + usage=requested |
| GWT-10 | finish releases | run finished | lifecycle | usage back to pre |
| GWT-11 | fail releases | run failed | lifecycle | usage back to pre |
| GWT-12 | cancel releases | run cancelled | lifecycle | usage back to pre |
| GWT-13 | shutdown drains | shutdown initiated | lifecycle | all released, usage=0 |
| GWT-14 | invalid entry | empty workflow | from_workflow | Err(EntryOutOfBounds) |
| GWT-15 | out-of-bounds step | target > node_count | from_workflow | Err(StepOutOfBounds) |
| GWT-16 | jump cycle | jump to in-path node | from_workflow | Err(JumpCycle) |
| GWT-17 | over capacity | requested > available | fits_within | Err(CapacityExceeded) |
| GWT-18 | add overflow | u64::MAX + 1 | try_add_budget | Err(Overflow) unchanged |
| GWT-19 | sub underflow | 0 - 1 | try_subtract_budget | Err(Underflow) unchanged |
| GWT-20 | policy exceeded | budget > policy | validate | Err(PolicyExceeded) |
| GWT-21 | zero active runs | capacity.max_active_runs=0 | validate | Err(InvalidCapacity) |
| GWT-22 | artifact missing | strict + absent digest | admit | Err(ArtifactNotFound) |
| GWT-23 | capability denied | missing cap | admit | Err(CapabilityDenied) |
| GWT-24 | rejection unchanged | over capacity | submit | all state unchanged |
| GWT-25 | unknown release | run not in reservation | release | Err(ReservationNotFound) |
| GWT-26 | double release | already released run | release | Err(ReservationNotFound) |
| GWT-27 | zero add | budget all zero | try_add_budget | Ok(same usage) |
| GWT-28 | zero subtract | budget all zero | try_subtract_budget | Ok(same usage) |
| GWT-29 | equal subtract zero | usage == budget | try_subtract_budget | Ok(usage=0) |
| GWT-30 | max boundary add | u64::MAX-1 + 1 | try_add_budget | Ok(u64::MAX) |
| GWT-31 | per-dim overflow | one dim overflows | try_add_budget | Err(first overflow dim) |
| GWT-32 | per-dim underflow | one dim underflows | try_subtract_budget | Err(first underflow dim) |
| GWT-33 | zero optional dims | counters=0 | from_whole | Ok(preserves zeros) |
| GWT-34 | policy max equality | budget==limit each dim | validate | Ok(()) |
| GWT-35 | minimal workflow | one Finish step | from_workflow | Ok(minimum values) |
