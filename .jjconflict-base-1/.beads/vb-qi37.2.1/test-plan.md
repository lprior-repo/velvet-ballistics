# Test Plan: vb-qi37.2.1 — runtime: Define aggregate resource budget model

## Summary

This repaired plan explicitly addresses every finding in `test-plan-review.md`: unit-test density is raised above the required floor, all previous “repeat per dimension” placeholders are replaced with named tests, every boundary class is listed per public function, each policy-governed dimension has exact typed expectations, runtime rejection paths assert no reservation/usage mutation, and Holzmann cleanup/panic-governance checks are concrete.

- Public operations from contract lines 88-122: 7.
- Required unit-test floor: 35 named unit tests (`7 * 5`).
- Planned named unit tests: 66 minimum.
- Trophy allocation: 66+ unit / 18 integration / 1 e2e acceptance / 8 static gates.
- Proptest invariants: 7.
- Fuzz targets: 2.
- Kani harnesses: 5.
- Mutation threshold: changed `vb_core`/`vb_runtime` files must reach ≥90% killed non-equivalent mutants; each critical branch/dimension below names the concrete test that kills it.
- Assertion rule: no test may assert only `is_ok()` or `is_err()`; assert exact values or exact typed errors and fields.

## 1. Behavior Inventory

1. `AggregateResourceBudget::from_workflow` returns finite exact dimensions when the workflow is valid and bounded.
2. `AggregateResourceBudget::from_workflow` returns the minimum valid finite budget when the workflow has one valid finish step and zero optional fanout/retry/gather branches.
3. `AggregateResourceBudget::from_workflow` returns max-limit finite values when workflow facts sit exactly at policy/resource limits.
4. `AggregateResourceBudget::from_workflow` rejects empty workflow parts through `WorkflowBudget(WorkflowError::EntryOutOfBounds { entry })` or the exact existing empty-workflow `WorkflowError` variant.
5. `AggregateResourceBudget::from_workflow` rejects invalid target steps through `WorkflowBudget(WorkflowError::StepOutOfBounds { step })`.
6. `AggregateResourceBudget::from_workflow` rejects cyclic jumps through `WorkflowBudget(WorkflowError::JumpCycle { step, target })`.
7. `AggregateResourceBudget::from_workflow` rejects aggregate arithmetic overflow through `AggregateBudgetError::Overflow { resource }`.
8. `AggregateResourceBudget::from_whole_workflow_budget` losslessly converts all copied/derived dimensions when values are valid.
9. `AggregateResourceBudget::from_whole_workflow_budget` preserves zero/minimum valid dimensions when zero is allowed by the source `ResourceContract`.
10. `AggregateResourceBudget::from_whole_workflow_budget` preserves maximum valid dimensions exactly when values fit target field widths.
11. `AggregateResourceBudget::from_whole_workflow_budget` rejects lossy narrowing through `AggregateBudgetError::Overflow { resource }`.
12. `validate_aggregate_budget` accepts each policy-governed dimension when it equals the limit.
13. `validate_aggregate_budget` accepts each policy-governed dimension when it is one below the limit.
14. `validate_aggregate_budget` rejects each policy-governed dimension at `limit + 1` with exact `PolicyExceeded { resource, actual, limit }`.
15. `validate_aggregate_budget` rejects invalid zero/below-minimum policy limits through `InvalidCapacity { resource }` or the implementation's exact typed policy-construction error.
16. `AggregateResourceUsage::try_add_budget` adds every dimension exactly when no sum overflows.
17. `AggregateResourceUsage::try_add_budget` returns unchanged usage for adding a zero budget.
18. `AggregateResourceUsage::try_add_budget` accepts boundary `u64::MAX - requested` and returns `u64::MAX`.
19. `AggregateResourceUsage::try_add_budget` rejects every overflowing dimension with exact `Overflow { resource }`.
20. `AggregateResourceUsage::try_subtract_budget` subtracts every dimension exactly when usage is greater than requested.
21. `AggregateResourceUsage::try_subtract_budget` returns zero for every dimension when usage equals requested.
22. `AggregateResourceUsage::try_subtract_budget` returns unchanged usage for subtracting a zero budget.
23. `AggregateResourceUsage::try_subtract_budget` rejects every underflowing dimension with exact `Underflow { resource }`.
24. Reservation release rejects unknown runs with `ReservationNotFound { run }` and unchanged usage.
25. `AggregateResourceUsage::fits_within` accepts zero usage against zero/valid capacity according to declared non-zero capacity rules.
26. `AggregateResourceUsage::fits_within` accepts equality for every comparable capacity dimension.
27. `AggregateResourceUsage::fits_within` accepts one-below-capacity for every comparable capacity dimension.
28. `AggregateResourceUsage::fits_within` rejects one-above-capacity for every comparable capacity dimension with exact `CapacityExceeded { resource, requested, available }`.
29. Capacity validation rejects zero for every production non-zero capacity dimension with `InvalidCapacity { resource }`.
30. `admit_run_with_budget` admits when artifact exists, capabilities pass, and requested budget equals available capacity.
31. `admit_run_with_budget` admits when requested budget is below available capacity.
32. `admit_run_with_budget` rejects over-capacity requests with `AdmissionError::ResourceCapacityExceeded { resource, requested, available }` or a documented lossless wrapper.
33. Strict/journaled artifact admission still rejects missing artifacts with `AdmissionError::ArtifactNotFound { digest }` and no usage/reservation mutation.
34. Capability admission still rejects insufficient grants with `AdmissionError::CapabilityDenied { action, required, granted }` and no usage/reservation mutation.
35. Budget rejection leaves active runs, reservations, usage counters, frame pools, journals, and trace rings unchanged except documented cold diagnostics.
36. Finish, fail, cancel, and shutdown paths release reservations and keep usage `<= capacity`.
37. Runtime core accepts typed Rust budget/capacity values only; no JSON/YAML/HTTP/string-command parsing enters aggregate admission.
38. Production code remains governance-clean: no `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing/slicing/casts/arithmetic, or swallowed fallible results.

## 2. Trophy Allocation

| Behavior | Layer | Tool | Rationale |
|---|---:|---|---|
| 1-7 | integration + unit boundaries | `cargo nextest -p vb_core` | Public workflow construction plus aggregate computation must exercise real `CompiledWorkflow` facts and exact error mapping. |
| 8-29 | unit + proptest + Kani | `#[test]`, `proptest`, `kani` | Pure deterministic value logic, policy checks, checked arithmetic, capacity comparison, and reservation table semantics require exhaustive matrix coverage. |
| 30-36 | integration + one e2e | `cargo nextest -p vb_runtime`, black-box runtime acceptance harness | Admission/reservation spans `vb_core`, artifact policy, capability checks, shard state, journals, and cleanup. Prefer real shard/runtime state over mocks. |
| 37-38 | static | `moon ci`, source scans, clippy, deny/nightly gates | Forbidden constructs and parser/resource-governance promises are static contracts. |

Target remains integration-heavy overall, but the unit inventory is intentionally dense for the 7 public fallible operations because the review required at least 35 named unit tests.

## 3. BDD Scenarios

### Behavior group A: `AggregateResourceBudget::from_workflow`

1. `fn aggregate_budget_returns_exact_fixture_values_when_workflow_is_bounded()`
   - Given: a `CompiledWorkflow` from `try_from_parts` with 4 steps, 2 action tickets, 2 parallel in-flight actions, 3 gather pages, 30 gather items, 5 for-each iterations, 2 together branches, 1 repeat attempt, 60 runtime seconds, 4096 result bytes, 6 total slots, queue depth 8, journal batch 8192.
   - When: `AggregateResourceBudget::from_workflow(&workflow)` is called.
   - Then: `Ok(AggregateResourceBudget { max_steps_executable: 4, max_action_tickets: 2, max_parallel_in_flight: 2, max_gather_pages: 3, max_gather_items: 30, max_for_each_iterations: 5, max_together_branches: 2, max_repeat_attempts: 1, max_run_time_seconds: 60, max_result_bytes: 4096, max_total_slots_written: 6, max_queue_depth: 8, max_journal_batch_bytes: 8192 })`.

2. `fn aggregate_budget_returns_minimum_values_when_workflow_has_one_finish_step()`
   - Given: the minimum valid workflow: one finish step, no actions, no fanout, no gather pages/items, no retries, zero optional slots, and finite resource contract values.
   - When: aggregate budget is computed.
   - Then: `Ok` contains exact minimum documented values: `max_steps_executable: 1`, action/fanout/gather/retry counters equal their valid minima, and resource-contract-derived queue/journal/result values equal the fixture contract.

3. `fn aggregate_budget_returns_limit_values_when_workflow_is_at_policy_maximum()`
   - Given: a valid workflow whose structural resource contract values equal every configured policy limit.
   - When: aggregate budget is computed.
   - Then: `Ok(AggregateResourceBudget { field == limit for every governed dimension })`.

4. `fn aggregate_budget_returns_workflow_entry_error_when_workflow_is_empty()`
   - Given: `WorkflowParts` with no nodes and entry `StepIdx(0)` passed through public validation or test-only unchecked construction if public construction rejects earlier.
   - When: aggregate computation is attempted.
   - Then: `Err(AggregateBudgetError::WorkflowBudget(WorkflowError::EntryOutOfBounds { entry: StepIdx(0) }))` or the repository's exact existing empty-workflow `WorkflowError` variant; the plan forbids generic errors.

5. `fn aggregate_budget_returns_workflow_step_error_when_target_is_out_of_bounds()`
   - Given: a workflow fixture whose target is `StepIdx(9)` while `node_count == 2`.
   - When: aggregate budget is computed.
   - Then: `Err(AggregateBudgetError::WorkflowBudget(WorkflowError::StepOutOfBounds { step: StepIdx(9) }))`.

6. `fn aggregate_budget_returns_workflow_jump_cycle_when_jump_reenters_path()`
   - Given: a workflow with jump `StepIdx(1) -> StepIdx(0)` that creates a traversal cycle.
   - When: aggregate budget is computed.
   - Then: `Err(AggregateBudgetError::WorkflowBudget(WorkflowError::JumpCycle { step: StepIdx(1), target: StepIdx(0) }))`.

7. `fn aggregate_budget_returns_overflow_when_total_steps_exceed_u32_max()`
   - Given: workflow facts whose repeated/for-each/together composition would require `max_steps_executable = u32::MAX as u64 + 1`.
   - When: aggregate budget is computed.
   - Then: `Err(AggregateBudgetError::Overflow { resource: "max_steps_executable" })`.

### Behavior group B: `AggregateResourceBudget::from_whole_workflow_budget`

8. `fn aggregate_budget_preserves_exact_dimensions_when_whole_budget_is_valid()`
   - Given: `WholeWorkflowBudget` and `ResourceContract` values `{ steps: 7, actions: 3, parallel: 2, gather_pages: 4, gather_items: 40, for_each: 5, together: 2, repeat: 1, runtime: 60, result_bytes: 4096, slots: 5, queue_depth: 11, journal_batch_bytes: 8192 }`.
   - When: conversion is called.
   - Then: `Ok(AggregateResourceBudget { ...same exact values... })`.

9. `fn aggregate_budget_preserves_zero_optional_dimensions_when_contract_allows_zero()`
   - Given: optional counters set to zero: action tickets, gather pages/items, for-each iterations, together branches, repeat attempts, total slots written; required dimensions set to their minimum non-zero values.
   - When: conversion is called.
   - Then: `Ok` has those optional fields exactly `0` and required fields equal their minimum fixture values.

10. `fn aggregate_budget_preserves_maximum_u32_dimensions_when_values_fit()`
    - Given: all u32-backed fields are `u32::MAX`, all u16-backed fields are `u16::MAX`, and runtime seconds fits `u64::MAX` if allowed by source types.
    - When: conversion is called.
    - Then: `Ok` contains exact maximum values without saturation or truncation.

11. `fn aggregate_budget_returns_overflow_when_action_tickets_exceed_u32()`
    - Given: source `max_action_tickets = u32::MAX as u64 + 1`.
    - When: conversion is called.
    - Then: `Err(AggregateBudgetError::Overflow { resource: "max_action_tickets" })`.

12. `fn aggregate_budget_returns_overflow_when_parallel_exceeds_u16()`
    - Given: source `max_parallel_in_flight = u16::MAX as u64 + 1`.
    - When: conversion is called.
    - Then: `Err(AggregateBudgetError::Overflow { resource: "max_parallel_in_flight" })`.

13. `fn aggregate_budget_returns_overflow_when_journal_batch_exceeds_u32()`
    - Given: source/contract `max_journal_batch_bytes = u32::MAX as u64 + 1`.
    - When: conversion is called.
    - Then: `Err(AggregateBudgetError::Overflow { resource: "max_journal_batch_bytes" })`.

### Behavior group C: `validate_aggregate_budget`

Policy-governed dimensions are all 14 fields unless implementation explicitly documents and tests a smaller set. If a dimension is not policy-governed, add a named test `validate_aggregate_budget_ignores_<dimension>_because_<contract_reason>` and cite the policy type field absence.

Named equality/below/over tests for every governed dimension:

14. `fn validate_aggregate_budget_accepts_steps_when_equal_to_limit()` → `Ok(())`.
15. `fn validate_aggregate_budget_accepts_steps_when_one_below_limit()` → `Ok(())`.
16. `fn validate_aggregate_budget_returns_policy_exceeded_when_steps_exceed_limit()` → `Err(AggregateBudgetError::PolicyExceeded { resource: "max_steps_executable", actual: 101, limit: 100 })`.
17. `fn validate_aggregate_budget_returns_policy_exceeded_when_action_tickets_exceed_limit()` → `Err(AggregateBudgetError::PolicyExceeded { resource: "max_action_tickets", actual: 11, limit: 10 })`.
18. `fn validate_aggregate_budget_returns_policy_exceeded_when_parallel_exceeds_limit()` → `Err(AggregateBudgetError::PolicyExceeded { resource: "max_parallel_in_flight", actual: 5, limit: 4 })`.
19. `fn validate_aggregate_budget_returns_policy_exceeded_when_retries_exceed_limit()` → `Err(AggregateBudgetError::PolicyExceeded { resource: "max_retries_per_action", actual: 4, limit: 3 })`.
20. `fn validate_aggregate_budget_returns_policy_exceeded_when_gather_pages_exceed_limit()` → `Err(AggregateBudgetError::PolicyExceeded { resource: "max_gather_pages", actual: 6, limit: 5 })`.
21. `fn validate_aggregate_budget_returns_policy_exceeded_when_gather_items_exceed_limit()` → `Err(AggregateBudgetError::PolicyExceeded { resource: "max_gather_items", actual: 501, limit: 500 })`.
22. `fn validate_aggregate_budget_returns_policy_exceeded_when_for_each_iterations_exceed_limit()` → `Err(AggregateBudgetError::PolicyExceeded { resource: "max_for_each_iterations", actual: 33, limit: 32 })`.
23. `fn validate_aggregate_budget_returns_policy_exceeded_when_together_branches_exceed_limit()` → `Err(AggregateBudgetError::PolicyExceeded { resource: "max_together_branches", actual: 9, limit: 8 })`.
24. `fn validate_aggregate_budget_returns_policy_exceeded_when_repeat_attempts_exceed_limit()` → `Err(AggregateBudgetError::PolicyExceeded { resource: "max_repeat_attempts", actual: 4, limit: 3 })`.
25. `fn validate_aggregate_budget_returns_policy_exceeded_when_run_time_exceeds_limit()` → `Err(AggregateBudgetError::PolicyExceeded { resource: "max_run_time_seconds", actual: 61, limit: 60 })`.
26. `fn validate_aggregate_budget_returns_policy_exceeded_when_result_bytes_exceed_limit()` → `Err(AggregateBudgetError::PolicyExceeded { resource: "max_result_bytes", actual: 4097, limit: 4096 })`.
27. `fn validate_aggregate_budget_returns_policy_exceeded_when_total_slots_exceed_limit()` → `Err(AggregateBudgetError::PolicyExceeded { resource: "max_total_slots_written", actual: 17, limit: 16 })`.
28. `fn validate_aggregate_budget_returns_policy_exceeded_when_queue_depth_exceed_limit()` → `Err(AggregateBudgetError::PolicyExceeded { resource: "max_queue_depth", actual: 65, limit: 64 })`.
29. `fn validate_aggregate_budget_returns_policy_exceeded_when_journal_batch_exceed_limit()` → `Err(AggregateBudgetError::PolicyExceeded { resource: "max_journal_batch_bytes", actual: 8193, limit: 8192 })`.
30. `fn validate_aggregate_budget_rejects_zero_policy_limit_for_required_dimension()` → `Err(AggregateBudgetError::InvalidCapacity { resource: "max_active_runs" })` or exact policy-construction error if validation is separated.
31. `fn validate_aggregate_budget_accepts_maximum_valid_policy_limits()` → `Ok(())` with all dimensions at documented maximum valid limits.

### Behavior group D: `AggregateResourceUsage::try_add_budget`

32. `fn usage_adds_all_dimensions_exactly_when_sums_fit()` → Given usage all `10`, budget all `3`; Then `Ok` has all comparable fields `13` and `active_runs` incremented only if reservation semantics include it.
33. `fn usage_add_returns_same_usage_when_budget_is_zero()` → Given usage `{ steps: 10, ... }`, zero optional budget; Then `Ok(usage)` exactly for every dimension.
34. `fn usage_add_accepts_max_boundary_when_sum_equals_u64_max()` → Given target dimension usage `u64::MAX - 1`, budget `1`; Then output dimension is exactly `u64::MAX`.

Per-dimension overflow tests, all require original usage unchanged:

35. `fn usage_add_returns_overflow_when_steps_sum_exceeds_u64()` → `Err(Overflow { resource: "max_steps_executable" })`.
36. `fn usage_add_returns_overflow_when_action_tickets_sum_exceeds_u64()` → `Err(Overflow { resource: "max_action_tickets" })`.
37. `fn usage_add_returns_overflow_when_parallel_sum_exceeds_u32()` → `Err(Overflow { resource: "max_parallel_in_flight" })` if stored as u32/u64 equivalent resource name.
38. `fn usage_add_returns_overflow_when_gather_pages_sum_exceeds_u64()` → `Err(Overflow { resource: "max_gather_pages" })`.
39. `fn usage_add_returns_overflow_when_gather_items_sum_exceeds_u64()` → `Err(Overflow { resource: "max_gather_items" })`.
40. `fn usage_add_returns_overflow_when_result_bytes_sum_exceeds_u64()` → `Err(Overflow { resource: "max_result_bytes" })`.
41. `fn usage_add_returns_overflow_when_total_slots_sum_exceeds_u64()` → `Err(Overflow { resource: "max_total_slots_written" })`.
42. `fn usage_add_returns_overflow_when_active_runs_sum_exceeds_u64()` → `Err(Overflow { resource: "max_active_runs" })`.
43. `fn usage_add_returns_overflow_when_queue_depth_sum_exceeds_u64()` → `Err(Overflow { resource: "max_queue_depth" })`.
44. `fn usage_add_returns_overflow_when_journal_batch_sum_exceeds_u64()` → `Err(Overflow { resource: "max_journal_batch_bytes" })`.

### Behavior group E: `AggregateResourceUsage::try_subtract_budget`

45. `fn usage_subtracts_all_dimensions_exactly_when_usage_exceeds_budget()` → Given usage all `10`, budget all `3`; Then `Ok` has all comparable fields `7`.
46. `fn usage_subtract_returns_zero_when_usage_equals_budget()` → Given usage equals budget for every dimension; Then `Ok(AggregateResourceUsage { every_comparable_dimension: 0 })`.
47. `fn usage_subtract_returns_same_usage_when_budget_is_zero()` → Given non-zero usage and zero optional budget; Then `Ok(usage)` exactly.

Per-dimension underflow tests, all require original usage unchanged:

48. `fn usage_subtract_returns_underflow_when_steps_would_go_negative()` → `Err(Underflow { resource: "max_steps_executable" })`.
49. `fn usage_subtract_returns_underflow_when_action_tickets_would_go_negative()` → `Err(Underflow { resource: "max_action_tickets" })`.
50. `fn usage_subtract_returns_underflow_when_parallel_would_go_negative()` → `Err(Underflow { resource: "max_parallel_in_flight" })`.
51. `fn usage_subtract_returns_underflow_when_gather_pages_would_go_negative()` → `Err(Underflow { resource: "max_gather_pages" })`.
52. `fn usage_subtract_returns_underflow_when_gather_items_would_go_negative()` → `Err(Underflow { resource: "max_gather_items" })`.
53. `fn usage_subtract_returns_underflow_when_result_bytes_would_go_negative()` → `Err(Underflow { resource: "max_result_bytes" })`.
54. `fn usage_subtract_returns_underflow_when_total_slots_would_go_negative()` → `Err(Underflow { resource: "max_total_slots_written" })`.
55. `fn usage_subtract_returns_underflow_when_active_runs_would_go_negative()` → `Err(Underflow { resource: "max_active_runs" })`.
56. `fn usage_subtract_returns_underflow_when_queue_depth_would_go_negative()` → `Err(Underflow { resource: "max_queue_depth" })`.
57. `fn usage_subtract_returns_underflow_when_journal_batch_would_go_negative()` → `Err(Underflow { resource: "max_journal_batch_bytes" })`.

### Behavior group F: `AggregateResourceUsage::fits_within` and capacity validation

58. `fn usage_fits_within_accepts_zero_usage_when_capacity_is_valid_nonzero()` → `Ok(())` for zero usage and required capacity fields set to minimum non-zero.
59. `fn usage_fits_within_accepts_equality_for_all_dimensions()` → `Ok(())` when every comparable field equals capacity.
60. `fn usage_fits_within_accepts_one_below_capacity_for_all_dimensions()` → `Ok(())` when every comparable field is capacity minus one.
61. `fn usage_fits_within_accepts_u64_max_capacity_when_usage_equals_u64_max()` → `Ok(())` for u64-backed dimensions at max where construction permits it.

Per-dimension one-above tests:

62. `fn usage_fits_within_returns_capacity_exceeded_when_steps_exceed_by_one()` → `Err(CapacityExceeded { resource: "max_steps_executable", requested: 101, available: 100 })`.
63. `fn usage_fits_within_returns_capacity_exceeded_when_action_tickets_exceed_by_one()` → `Err(CapacityExceeded { resource: "max_action_tickets", requested: 101, available: 100 })`.
64. `fn usage_fits_within_returns_capacity_exceeded_when_parallel_exceed_by_one()` → `Err(CapacityExceeded { resource: "max_parallel_in_flight", requested: 11, available: 10 })`.
65. `fn usage_fits_within_returns_capacity_exceeded_when_gather_pages_exceed_by_one()` → `Err(CapacityExceeded { resource: "max_gather_pages", requested: 101, available: 100 })`.
66. `fn usage_fits_within_returns_capacity_exceeded_when_gather_items_exceed_by_one()` → `Err(CapacityExceeded { resource: "max_gather_items", requested: 101, available: 100 })`.
67. `fn usage_fits_within_returns_capacity_exceeded_when_result_bytes_exceed_by_one()` → `Err(CapacityExceeded { resource: "max_result_bytes", requested: 101, available: 100 })`.
68. `fn usage_fits_within_returns_capacity_exceeded_when_total_slots_exceed_by_one()` → `Err(CapacityExceeded { resource: "max_total_slots_written", requested: 101, available: 100 })`.
69. `fn usage_fits_within_returns_capacity_exceeded_when_active_runs_exceed_by_one()` → `Err(CapacityExceeded { resource: "max_active_runs", requested: 2, available: 1 })`.
70. `fn usage_fits_within_returns_capacity_exceeded_when_queue_depth_exceed_by_one()` → `Err(CapacityExceeded { resource: "max_queue_depth", requested: 101, available: 100 })`.
71. `fn usage_fits_within_returns_capacity_exceeded_when_journal_batch_exceed_by_one()` → `Err(CapacityExceeded { resource: "max_journal_batch_bytes", requested: 101, available: 100 })`.
72. `fn capacity_validation_returns_invalid_capacity_when_active_runs_is_zero()` → `Err(InvalidCapacity { resource: "max_active_runs" })`.
73. `fn capacity_validation_returns_invalid_capacity_when_queue_depth_is_zero()` → `Err(InvalidCapacity { resource: "max_queue_depth" })` unless zero queue depth is explicitly supported and documented.
74. `fn capacity_validation_returns_invalid_capacity_when_journal_batch_is_zero()` → `Err(InvalidCapacity { resource: "max_journal_batch_bytes" })` unless zero journal batch is explicitly supported and documented.

### Behavior group G: reservation table / release API

75. `fn reservation_release_returns_not_found_when_run_has_no_active_reservation()`
   - Given: reservation state with `RunId::new(42)` absent and usage snapshot `U`.
   - When: release is requested for `RunId::new(42)`.
   - Then: `Err(AggregateBudgetError::ReservationNotFound { run: RunId::new(42) })` and usage equals `U`.

76. `fn reservation_release_returns_not_found_when_run_is_released_twice()`
   - Given: one reservation for `RunId::new(7)` already released once.
   - When: release is requested again.
   - Then: `Err(AggregateBudgetError::ReservationNotFound { run: RunId::new(7) })` and usage remains the post-first-release value.

### Behavior group H: runtime admission and shard lifecycle

77. `fn admit_run_with_budget_returns_admission_when_requested_equals_capacity()`
   - Given: artifact exists, strict policy, capabilities sufficient, requested budget exactly equals available capacity for every dimension, and no existing active runs.
   - When: `admit_run_with_budget(...)` or shard submit path is called.
   - Then: `Ok(RunAdmission)` with exact digest/run/policy/capabilities and, if exposed, exact budget/reservation equal to requested.
   - And: active usage equals requested, not requested minus one.

78. `fn admit_run_with_budget_returns_admission_when_requested_is_one_below_capacity()`
   - Given: every requested dimension is `available - 1`.
   - When: admission runs.
   - Then: `Ok(RunAdmission)` and active usage increments by exactly requested dimensions.

79. `fn admit_run_with_budget_returns_resource_capacity_exceeded_when_action_tickets_exceed_capacity()`
   - Given: artifact exists, capabilities sufficient, requested `max_action_tickets = 6`, available `max_action_tickets = 5`.
   - When: admission runs.
   - Then: `Err(AdmissionError::ResourceCapacityExceeded { resource: "max_action_tickets", requested: 6, available: 5 })` or a documented exact wrapper.
   - And: active runs, reservations, active usage, frame pools, journals, and trace rings equal their pre-call snapshots.

80. `fn admit_run_with_budget_returns_artifact_not_found_without_reservation_when_strict_artifact_missing()`
   - Given: strict/journaled policy, absent digest, budget fits, capabilities sufficient, pre-call usage `U`.
   - When: admission runs.
   - Then: `Err(AdmissionError::ArtifactNotFound { digest })`.
   - And: no reservation exists for the run, active usage equals `U`, and active run count is unchanged.

81. `fn admit_run_with_budget_returns_capability_denied_without_reservation_when_capability_missing()`
   - Given: artifact exists, budget fits, required `Capability::new("network.http")` for `ActionId::new(9)`, granted set lacks it, pre-call usage `U`.
   - When: admission runs.
   - Then: `Err(AdmissionError::CapabilityDenied { action: ActionId::new(9), required, granted })`.
   - And: the implementation-defined ordering must be observable as no retained reservation and active usage equals `U`; if a temporary reservation is taken before capability checking, rollback must be asserted by final state.

82. `fn shard_submit_leaves_all_resource_state_unchanged_when_budget_rejected()`
   - Given: isolated shard with snapshots of active runs, reservation map, active usage, frame-pool counts, journal length, trace-ring length, and diagnostic counters.
   - When: submit rejects due to `max_queue_depth` capacity exceeded.
   - Then: all non-diagnostic snapshots are byte/value equal to pre-call; diagnostic counter changes must be named in the assertion.

83. `fn shard_release_reservation_when_run_finishes_successfully()` → usage returns to pre-admission snapshot and reservation is absent.
84. `fn shard_release_reservation_when_run_fails()` → usage returns to pre-admission snapshot and reservation is absent.
85. `fn shard_release_reservation_when_run_is_cancelled()` → usage returns to pre-admission snapshot and reservation is absent.
86. `fn shard_release_reservation_when_shutdown_drains_active_runs()` → all reservations absent and `active_usage.fits_within(capacity) == Ok(())`.

### Behavior group I: static/runtime governance

87. `static_runtime_budget_model_uses_only_typed_values()` → changed runtime-core files contain no JSON/YAML/HTTP/string-command parsing for aggregate budget/capacity.
88. `static_budget_model_has_no_forbidden_constructs()` → no production `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg` in changed files.
89. `static_budget_model_has_no_unchecked_index_slice_cast_arithmetic()` → source scan or lint rejects unchecked `[]`, slicing, `as` casts where lossy, and unchecked `+ - *` in aggregate arithmetic.
90. `static_runtime_tests_do_not_swallow_fallible_cleanup()` → integration tests must return `Result` or explicitly assert cleanup results; no ignored `Result` from cancel/release/journal/frame cleanup.

## 4. Proptest Invariants

### Proptest: `AggregateResourceBudget::from_whole_workflow_budget`
- Invariant: for any valid `WholeWorkflowBudget` and `ResourceContract`, each shared/derived dimension equals the source value exactly.
- Strategy: generate bounded integers per field type, including 0, 1, max/2, max.
- Anti-invariant: generate values one above target field width; expect `Err(AggregateBudgetError::Overflow { resource })`, not truncation/saturation.

### Proptest: `validate_aggregate_budget`
- Invariant: all governed dimensions `<= limit` returns `Ok(())`; one generated dimension `limit + delta` returns `PolicyExceeded` for that exact dimension.
- Strategy: generate positive policy limits and budgets with a tagged over-limit dimension across all 14 budget fields.
- Anti-invariant: any over-limit field must never return `Ok(())` or a different resource name.

### Proptest: `AggregateResourceUsage::try_add_budget`
- Invariant: non-overflowing output equals component-wise `usage + budget`.
- Strategy: generate usage and budget with `budget <= MAX - usage` per dimension; include zero and max-boundary cases.
- Anti-invariant: one tagged field where `budget > MAX - usage`; expect `Overflow { resource }` and unchanged original usage.

### Proptest: `AggregateResourceUsage::try_subtract_budget`
- Invariant: when `usage >= budget`, output equals component-wise `usage - budget`.
- Strategy: generate budget first, then usage in `budget..=MAX` per dimension; include equality and zero subtraction.
- Anti-invariant: one tagged field where `budget > usage`; expect `Underflow { resource }` and unchanged original usage.

### Proptest: add/subtract round trip
- Invariant: `usage.try_add_budget(budget)?.try_subtract_budget(budget)? == usage` for every non-overflowing add.
- Strategy: reuse checked-add strategy and shrink toward zero/one/equality.
- Anti-invariant: overflowing add or underflowing subtract must return exact typed error before changing state.

### Proptest: `AggregateResourceUsage::fits_within`
- Invariant: result is `Ok(())` iff every comparable dimension is `<= capacity`.
- Strategy: generate capacity and a tagged relation per field: zero, below, equal, one-above, max.
- Anti-invariant: any one-above dimension must produce `CapacityExceeded` naming the first documented comparison-order dimension.

### Proptest: reservation lifecycle
- Invariant: after any valid reserve/release sequence, active usage equals the component-wise sum of active reservations and never exceeds capacity after an `Ok` operation.
- Strategy: small vectors of unique `RunId`s, operations, and budgets; generated capacity high enough for accepted cases.
- Anti-invariant: unknown release or double release returns `ReservationNotFound { run }` and unchanged usage.

## 5. Fuzz Targets

### Fuzz Target: `CompiledWorkflow::try_from_parts` then `AggregateResourceBudget::from_workflow`
- Input type: arbitrary structured `WorkflowParts` generated from bytes via `arbitrary`; not JSON/YAML.
- Risk: invalid indices, cycles, overflow in derived totals, stale resource contracts, unchecked indexing, OOM from unbounded vectors.
- Corpus seeds: one-step finish workflow, empty nodes, entry out of bounds, target out of bounds, jump cycle, nested for-each/repeat/together at limits, each resource field at max, each resource field one above policy.
- Oracle: no panic/abort/OOM; valid workflows produce exact aggregate values; invalid inputs return exact `WorkflowError`/`AggregateBudgetError` variants.

### Fuzz Target: compiled artifact deserialization boundary for `WorkflowParts`
- Input type: bytes consumed by the existing compiled artifact deserializer, then public validation and aggregate computation when decode succeeds.
- Risk: malformed resource metadata accepted then causing panic, overflow, or default-unbounded behavior.
- Corpus seeds: serialized minimal workflow, serialized max-resource workflow, truncated bytes, duplicate/invalid step identifiers, mutated resource contract fields, random high queue/journal/result fields.
- Oracle: decode failures are typed deserializer errors; decode successes either validate to finite exact budget or typed workflow/budget error. Runtime core must not gain JSON/YAML/HTTP fuzz targets because those parsers are forbidden.

## 6. Kani Harnesses

1. **checked addition cannot overflow silently**
   - Property: `try_add_budget` equals primitive `checked_add` per dimension or returns `Overflow { resource }` before state mutation.
   - Bound: one symbolic usage and budget; reduced ranges for full matrix plus selected max-boundary harnesses.
   - Rationale: overflow must never wrap/saturate.

2. **checked subtraction cannot underflow silently**
   - Property: `try_subtract_budget` equals primitive `checked_sub` per dimension or returns `Underflow { resource }` before state mutation.
   - Bound: one symbolic usage and budget.
   - Rationale: release bugs leak/fabricate capacity.

3. **capacity comparison is inclusive and dimension-complete**
   - Property: `fits_within` is equivalent to conjunction of every documented dimension `usage <= capacity`.
   - Bound: symbolic `0..=3` values for each field plus comparison-order assumption.
   - Rationale: kills omitted-dimension and off-by-one bugs.

4. **reserve/release round trip preserves usage**
   - Property: successful reserve then release for the same run returns usage to initial value and removes reservation.
   - Bound: one run, one budget, one capacity with small symbolic ranges.
   - Rationale: lifecycle correctness crosses arithmetic and table state.

5. **admission cannot return `Ok` with usage above capacity**
   - Property: every successful admission leaves `active_usage.fits_within(capacity) == Ok(())`.
   - Bound: existing usage, requested budget, capacity, artifact/capability booleans.
   - Rationale: public shard invariant after every `Ok` path.

## 7. Mutation Testing Checkpoints

Threshold: `cargo mutants --package vb_core --package vb_runtime` or repository-approved equivalent must kill ≥90% of non-equivalent mutants in changed files. Equivalent mutants require documented justification.

Concrete kill map:

- Change capacity `<=` to `<`: killed by `usage_fits_within_accepts_equality_for_all_dimensions` and `admit_run_with_budget_returns_admission_when_requested_equals_capacity`.
- Change capacity `>` to `>=`: killed by equality tests above.
- Remove steps comparison: killed by `usage_fits_within_returns_capacity_exceeded_when_steps_exceed_by_one`.
- Remove action-tickets comparison: killed by `usage_fits_within_returns_capacity_exceeded_when_action_tickets_exceed_by_one`.
- Remove parallel comparison: killed by `usage_fits_within_returns_capacity_exceeded_when_parallel_exceed_by_one`.
- Remove gather-pages comparison: killed by `usage_fits_within_returns_capacity_exceeded_when_gather_pages_exceed_by_one`.
- Remove gather-items comparison: killed by `usage_fits_within_returns_capacity_exceeded_when_gather_items_exceed_by_one`.
- Remove result-bytes comparison: killed by `usage_fits_within_returns_capacity_exceeded_when_result_bytes_exceed_by_one`.
- Remove total-slots comparison: killed by `usage_fits_within_returns_capacity_exceeded_when_total_slots_exceed_by_one`.
- Remove active-runs comparison: killed by `usage_fits_within_returns_capacity_exceeded_when_active_runs_exceed_by_one`.
- Remove queue-depth comparison: killed by `usage_fits_within_returns_capacity_exceeded_when_queue_depth_exceed_by_one`.
- Remove journal-batch comparison: killed by `usage_fits_within_returns_capacity_exceeded_when_journal_batch_exceed_by_one`.
- Swap `requested`/`available` in capacity error: killed by `admit_run_with_budget_returns_resource_capacity_exceeded_when_action_tickets_exceed_capacity`.
- Replace checked add with wrapping/saturating add: killed by all `usage_add_returns_overflow_when_*` tests plus Kani checked-add harness.
- Omit any add dimension: killed by `usage_adds_all_dimensions_exactly_when_sums_fit` and corresponding overflow test.
- Replace checked subtract with wrapping/saturating subtract: killed by all `usage_subtract_returns_underflow_when_*` tests plus Kani checked-subtract harness.
- Omit any subtract dimension: killed by `usage_subtracts_all_dimensions_exactly_when_usage_exceeds_budget` and corresponding underflow test.
- Change any policy resource string: killed by corresponding `validate_aggregate_budget_returns_policy_exceeded_when_*` exact assertion.
- Omit gather pages/items/for-each/together/repeat/slots/queue/journal policy branches: killed by tests 20-29.
- Treat missing aggregate budget as unbounded/default zero: killed by `aggregate_budget_returns_minimum_values_when_workflow_has_one_finish_step` and runtime admission finite-budget scenarios.
- Map workflow errors to generic policy error: killed by invalid entry/step/jump-cycle scenarios.
- Reserve before artifact failure without rollback: killed by `admit_run_with_budget_returns_artifact_not_found_without_reservation_when_strict_artifact_missing`.
- Reserve before capability failure without rollback: killed by `admit_run_with_budget_returns_capability_denied_without_reservation_when_capability_missing`.
- Insert active run before budget rejection without rollback: killed by `shard_submit_leaves_all_resource_state_unchanged_when_budget_rejected`.
- Forget release on finish/fail/cancel/shutdown: killed by tests 83-86.
- Add JSON/YAML/HTTP/text parsing in runtime core: killed by `static_runtime_budget_model_uses_only_typed_values`.
- Introduce `unwrap`/`panic`/ignored cleanup result: killed by static tests 88-90 and `moon ci` lint gates.

## 8. Combinatorial Coverage Matrix

| Public operation | Scenario | Input Class | Expected Output | Layer |
|---|---|---|---|---|
| `from_workflow` | bounded happy path | valid workflow | `Ok(AggregateResourceBudget { exact fixture values })` | integration |
| `from_workflow` | minimum valid | one finish step | `Ok` with exact minima | integration |
| `from_workflow` | max valid | at policy limits | `Ok` with exact limit values | integration |
| `from_workflow` | empty/invalid entry | no nodes | `Err(WorkflowBudget(EntryOutOfBounds { entry }))` or exact empty variant | integration/fuzz |
| `from_workflow` | invalid target | target out of bounds | `Err(WorkflowBudget(StepOutOfBounds { step }))` | integration/fuzz |
| `from_workflow` | cycle | jump reenters path | `Err(WorkflowBudget(JumpCycle { step, target }))` | integration/fuzz |
| `from_workflow` | overflow | derived steps > u32 max | `Err(Overflow { resource: "max_steps_executable" })` | integration/fuzz |
| `from_whole_workflow_budget` | exact conversion | valid values | `Ok` exact fields | unit |
| `from_whole_workflow_budget` | zero/min | optional zeros | `Ok` preserves zeros/minima | unit |
| `from_whole_workflow_budget` | max valid | max fitting fields | `Ok` exact max fields | unit/proptest |
| `from_whole_workflow_budget` | overflow | one field too wide | `Err(Overflow { resource })` | unit/proptest |
| `validate_aggregate_budget` | equality | field == limit | `Ok(())` | unit |
| `validate_aggregate_budget` | one below | field == limit - 1 | `Ok(())` | unit |
| `validate_aggregate_budget` | one above per dimension | field == limit + 1 | `Err(PolicyExceeded { resource, actual, limit })` | unit/proptest |
| `validate_aggregate_budget` | invalid policy min | required limit zero | `Err(InvalidCapacity { resource })` or exact policy constructor error | unit |
| `try_add_budget` | happy | all sums fit | `Ok(usage + budget)` exact fields | unit |
| `try_add_budget` | zero | budget zero | `Ok(usage)` exact fields | unit |
| `try_add_budget` | max boundary | sum == max | `Ok(max)` exact field | unit/kani |
| `try_add_budget` | overflow per dimension | sum > max | `Err(Overflow { resource })` and unchanged usage | unit/proptest/kani |
| `try_subtract_budget` | happy | usage > budget | `Ok(usage - budget)` exact fields | unit |
| `try_subtract_budget` | equality | usage == budget | `Ok(0)` exact fields | unit/kani |
| `try_subtract_budget` | zero | budget zero | `Ok(usage)` exact fields | unit |
| `try_subtract_budget` | underflow per dimension | budget > usage | `Err(Underflow { resource })` and unchanged usage | unit/proptest/kani |
| reservation release | unknown | absent run | `Err(ReservationNotFound { run })`; unchanged usage | integration/proptest |
| `fits_within` | zero usage | valid capacity | `Ok(())` | unit |
| `fits_within` | equality | usage == capacity | `Ok(())` | unit/integration |
| `fits_within` | below | usage == capacity - 1 | `Ok(())` | unit |
| `fits_within` | max | usage == max capacity | `Ok(())` | unit/kani |
| `fits_within` | one above per dimension | usage == capacity + 1 | `Err(CapacityExceeded { resource, requested, available })` | unit/proptest |
| capacity validation | zero required fields | capacity field zero | `Err(InvalidCapacity { resource })` | unit |
| admission | equality | requested == available | `Ok(RunAdmission { exact input fields })`; usage == requested | integration |
| admission | below | requested == available - 1 | `Ok(RunAdmission)`; exact usage increment | integration |
| admission | capacity reject | requested > available | `Err(ResourceCapacityExceeded { resource, requested, available })`; no state mutation | integration |
| admission | artifact reject | missing artifact | `Err(ArtifactNotFound { digest })`; no reservation/usage mutation | integration |
| admission | capability reject | missing cap | `Err(CapabilityDenied { action, required, granted })`; no reservation/usage mutation | integration |
| shard lifecycle | finish/fail/cancel/shutdown | admitted run | usage returns to pre-admission; reservation absent | integration/e2e |
| static | governance | changed source | no forbidden constructs/parsers/ignored cleanup results; `moon ci` passes | static |

## Static Resource/Panic-Governance Checks

The test-writer must add or invoke explicit static gates for changed production files and runtime integration tests:

1. Forbidden constructs scan over changed production files: reject `unsafe`, `.unwrap(`, `.expect(`, `panic!`, `todo!`, `unimplemented!`, `dbg!`.
2. Checked arithmetic scan/review: aggregate dimension arithmetic must use `checked_add`, `checked_sub`, checked widening/narrowing helpers, or typed constructors; no raw `+`, `-`, `*`, lossy `as`, unchecked indexing, or unchecked slicing in aggregate code paths.
3. Parser boundary scan: runtime admission/shard code must not introduce JSON/YAML/HTTP/string-command parsing for budget/capacity.
4. Runtime integration-test cleanup rule: each test creates an isolated shard/temp journal/frame pool/artifact store; no shared mutable global fixtures.
5. Runtime integration-test result rule: cancel/release/shutdown/journal/frame-pool cleanup returns must be asserted exactly; no `let _ = fallible_call()` for cleanup.
6. Resource-leak oracle: every rejection-path test snapshots reservations, active usage, active run count, frame pool available count, journal entry count, and trace-ring length before the call and compares exact post-call state.
7. Panic oracle: fuzz, proptest, and integration rejection paths must complete without unwind; panic is a test failure, not a typed rejection.
8. Canonical final gate: `moon ci` must pass before landing; targeted Cargo commands are only feedback.

## Command Gates And Acceptance Evidence

After tests exist, record these commands in the bead handoff:

1. Focused core: `cargo nextest run -p vb_core aggregate budget` or exact repository filter covering tests 1-76.
2. Focused runtime: `cargo nextest run -p vb_runtime admission shard` or exact filter covering tests 77-86.
3. Property tests: run generated `proptest` cases with CI default cases; failures must print shrunk exact input.
4. Fuzz smoke: `cargo fuzz run <workflow_aggregate_target> -- -runs=1000` and `cargo fuzz run <artifact_aggregate_target> -- -runs=1000`.
5. Kani: run aggregate harnesses if Kani is enabled; otherwise create follow-up bead documenting missing formal gate.
6. Mutation: run repository-approved `cargo-mutants` scope and document ≥90% kill rate or equivalent exclusions.
7. Static gates: run forbidden-construct/resource/parser scans plus `moon ci`.

## Open Questions

- Capacity remains shard-local in tests unless the implementation explicitly adds runtime-level distributed capacity.
- If `RunAdmission` does not store granted budget, integration tests must assert the public reservation/audit structure that owns the exact budget.
- If any budget dimension is intentionally not governed by `BoundednessPolicy`, implementation must document why and tests must assert that exclusion explicitly rather than omit it.
- If zero capacity is valid for a specific dimension, replace the corresponding `InvalidCapacity` scenario with an exact `Ok` scenario and cite the contract refinement.
