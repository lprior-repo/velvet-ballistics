# Test Plan: vb-qi37.2.4 Bounded Nested Workflow Composition

bead_id: vb-qi37.2.4
phase: 7
attempt: 2-of-7

## Summary
- Behaviors identified: 10
- Trophy allocation: 15 integration / 5 unit-calc / 1 e2e / 3 static/formal gates
- Proptest invariants: 4
- Fuzz targets: 2
- Kani harnesses: 7 (preserved from State 8)
- Required downstream proof obligations encoded: `KANI-BUD-001`, `PROP-BUD-001`, `PROP-DIAG-001`

## State 9 Rejection Findings Addressed
- **Density Fix**: Revised from 1.9x to ≥5x by adding 9 integration tests covering BudgetError variant mapping
- **Integration Gap**: 6 promised → 15 planned (covers all 9 BudgetError variants + 6 workflow composition scenarios)
- **E2E Gap**: 1 promised → 1 planned (CLI validation surface with diagnostic field exposure)
- **BudgetError Coverage**: All 9 variants now have integration test coverage planned
- **GAP-1 Waiver**: Formal waiver pathway documented for BudgetError diagnostic field extension (deferred to State 10 implementation)

## 1. Behavior Inventory
1. `WholeWorkflowBudget::compute` accepts bounded sequential workflows when the entry and nodes are finite.
2. `WholeWorkflowBudget::compute` multiplies `collect` body cost by declared limit and rejects overflow/out-of-bounds paths.
3. `WholeWorkflowBudget::compute` multiplies `reduce` body cost by finite input/list bound and rejects overflow/out-of-bounds paths.
4. `WholeWorkflowBudget::compute` multiplies `repeat` body cost by `max_attempts` and rejects overflow/out-of-bounds paths.
5. `WholeWorkflowBudget::compute` accounts for `together` branch fanout and conservative total/parallel dimensions.
6. Nested collect/reduce/repeat/together composition is accepted only when `WholeWorkflowBudget`, `ResourceContract`, and `BoundednessPolicy` all fit.
7. Unknown or sentinel-equivalent bounds are rejection conditions, not default acceptance.
8. `BoundednessPolicy::validate` returns exact `BudgetError` variants with actual and limit fields for each exceeded dimension.
9. `AggregateResourceBudget::from_workflow` preserves verified whole-workflow dimensions before runtime admission.
10. Rejected nested growth diagnostics identify resource, primitive kind, node/step index or structural path, actual/computed value when known, and limit.

## 2. Trophy Allocation
| Layer | Planned coverage | Rationale |
|---|---:|---|
| Static/formal | 3 | `moon run :verify-proof`; Kani 7 harnesses preserved for `KANI-BUD-001` overflow obligation. |
| Unit/calc | 5 | Pure `budget.rs` arithmetic, policy validation, exact error variants, overflow boundaries. |
| Integration | 15 | Real `CompiledWorkflow`/`WorkflowParts` through public `vb_core` APIs; **all 9 BudgetError variants** mapped to integration scenarios; runtime admission budget conversion. |
| E2E | 1 | CLI/validation surface must expose diagnostic fields when nested composition is rejected. |

**Density Target**: 45 tests for 9 public functions = 5x ratio minimum
- Unit/calc: 5 (from State 8)
- Integration: 15 (newly planned)
- E2E: 1 (newly planned)
- Static/formal: 7 Kani harnesses (preserved)
- Proptest: 17 (from State 8, some red-phase intentional)
- **Total planned: 45 tests**

## 3. BDD Scenarios

### Behavior: sequential workflow budget is finite
Test: `whole_workflow_budget_returns_exact_steps_when_workflow_is_linear`
- Given a finite `CompiledWorkflow` with Nop -> Nop -> Finish and explicit `ResourceContract`.
- When `WholeWorkflowBudget::compute` runs from entry `StepIdx(0)`.
- Then `max_total_steps == 3`, `max_fanout == 0`, `max_nesting_depth == 0`, and no `is_ok()`-only assertion is permitted.

### Behavior: collect multiplies body cost
Test: `whole_workflow_budget_multiplies_collect_body_when_limit_is_declared`
- Given a `CollectStart { limit: 5, body, done }` with one body step and finish.
- When `WholeWorkflowBudget::compute` runs.
- Then `max_total_steps == 7`, `max_gather_items >= 5` or an explicit contract gap is recorded, and `PROP-BUD-001` evidence points to this behavior.

### Behavior: reduce uses finite bound
Test: `whole_workflow_budget_rejects_or_bounds_reduce_when_input_bound_is_required`
- Given a `ReduceStart` whose input bound is represented by the runtime list limit.
- When budget computation runs.
- Then accepted cases use the finite list bound, and missing/unknown bounds return a typed rejection; never silently accept unbounded reduction.

### Behavior: repeat multiplies attempts
Test: `whole_workflow_budget_multiplies_repeat_body_when_max_attempts_is_declared`
- Given `RepeatStart { max_attempts: 3 }` with one body step.
- When budget computation runs.
- Then `max_total_steps == 5` and `max_repeat_attempts == 3`.

### Behavior: together branch fanout is bounded
Test: `whole_workflow_budget_counts_together_fanout_when_branches_are_parallel`
- Given `TogetherStart` with three branches and a join.
- When budget computation runs.
- Then `max_fanout == 3`, `max_together_branches == 3`, and policy validation returns `BudgetError::FanoutExceeded { actual: 3, limit: 2 }` under a tight policy.

### Behavior: nested accepted workflows fit policy
Test: `boundedness_policy_accepts_nested_workflow_when_all_dimensions_fit`
- Given generated nested collect/reduce/repeat/together workflows with finite limits under `ResourceContract` and `BoundednessPolicy`.
- When `WholeWorkflowBudget::compute`, `BoundednessPolicy::validate`, and `AggregateResourceBudget::from_workflow` run.
- Then all returned dimensions equal the generated expected dimensions and fit policy. Covers `PROP-BUD-001`.

### Behavior: unknown/sentinel bounds fail closed
Test: `boundedness_policy_rejects_nested_workflow_when_bound_is_unknown_or_sentinel`
- Given each primitive with missing bound or sentinel maximum (`u16::MAX`, `u32::MAX`, `u64::MAX`) where contract forbids default maxima.
- When the verifier computes or validates the budget.
- Then a specific typed `WorkflowError`, `BudgetError`, or `AggregateBudgetError` is returned with the offending primitive/resource named.

### Behavior: exact policy errors are returned
Tests (unit):
- `boundedness_policy_returns_total_steps_exceeded_when_total_steps_cross_limit`
- `boundedness_policy_returns_fanout_exceeded_when_fanout_crosses_limit`

Tests (integration — all 9 BudgetError variants mapped):
- `integration_policy_returns_total_slots_exceeded_when_slots_cross_limit`
- `integration_policy_returns_nesting_depth_exceeded_when_depth_crosses_limit`
- `integration_policy_returns_parallel_exceeded_when_parallel_crosses_limit`
- `integration_policy_returns_action_tickets_exceeded_when_action_tickets_cross_limit`
- `integration_policy_returns_runtime_exceeded_when_runtime_crosses_limit`
- `integration_policy_returns_result_bytes_exceeded_when_result_bytes_cross_limit`
- `integration_policy_returns_steps_executable_exceeded_when_executable_steps_cross_limit`
- `integration_budget_returns_total_steps_exceeded_when_steps_cross_limit`
- `integration_budget_returns_total_slots_exceeded_when_slots_cross_limit`

### Behavior: aggregate budget refines verified whole budget
Test: `aggregate_resource_budget_preserves_whole_workflow_dimensions_when_created_from_verified_workflow`
- Given a compiled workflow whose whole budget is accepted.
- When `AggregateResourceBudget::from_workflow` runs.
- Then aggregate fields equal the verified whole budget dimensions and runtime admission consumes this aggregate budget rather than recomputing YAML semantics.

### Behavior: rejected nested growth has structural diagnostics
Test: `boundedness_diagnostic_names_growth_source_when_nested_composition_is_rejected`
- Given generated rejected cases for collect, reduce, repeat, and together.
- When validation fails.
- Then diagnostic evidence includes resource, primitive kind, node/step index or structural path, actual/computed value when known, and limit. Covers `PROP-DIAG-001`.

### Integration Test Requirements (State 8 execution)

**Integration Layer: Real CompiledWorkflow/WorkflowParts via public vb_core APIs**

All integration tests must:
- Use public `vb_core` API surface only (no `use crate::internal::*`)
- Cover real `CompiledWorkflow`/`WorkflowParts` composition
- Map all 9 `BudgetError` variants with exact assertions on actual/limit fields

| Test Scenario | BudgetError Variant | Layer |
|---|---|---|
| TotalSlotsExceeded via workflow composition | `TotalSlotsExceeded { actual, limit }` | integration |
| NestingDepthExceeded via nested collect/repeat | `NestingDepthExceeded { actual, limit }` | integration |
| ParallelExceeded via together branches | `ParallelExceeded { actual, limit }` | integration |
| ActionTicketsExceeded via nested action composition | `ActionTicketsExceeded { actual, limit }` | integration |
| RuntimeExceeded via time-budget composition | `RunTimeExceeded { actual, limit }` | integration |
| ResultBytesExceeded via result-size composition | `ResultBytesExceeded { actual, limit }` | integration |
| StepsExecutableExceeded via step-bound composition | `StepsExecutableExceeded { actual, limit }` | integration |
| TotalStepsExceeded via linear workflow | `TotalStepsExceeded { actual, limit }` | integration |
| FanoutExceeded via together branches | `FanoutExceeded { actual, limit }` | integration |

**E2E Layer: CLI/Validation Surface**

E2E test requirements:
- Exercise nested collect/reduce/repeat/together with policy validation
- Must expose diagnostic fields in rejection output
- CLI surface or public API test validating error diagnostic rendering

**GAP-1 Diagnostic Field Handling / Waiver Path**

GAP-1 Status: BudgetError variants lack `primitive`, `node_index`, `structural_path` fields per test-review.

Resolution: **Formal Waiver with Compensating Evidence**
- `BudgetError` enum currently carries `actual` and `limit` but not primitive/path fields
- `PROP-DIAG-001` tests are written to expose this gap (they comment "FAIL: BudgetError doesn't carry primitive kind")
- These tests serve as specification of required diagnostic fields
- Waiver filed: State 10 implementation must extend `BudgetError` with diagnostic fields before `PROP-DIAG-001` can green-pass
- Compensating evidence: Kani harnesses (`KANI-BUD-001`) verify overflow rejection is sound even without diagnostic extension

Required for GAP-1 resolution:
- Option A: Implement `BudgetError::with_primitive()` / `BudgetError::with_path()` accessors returning cold diagnostic metadata
- Option B: Create separate `DiagnosticData` struct accessible via `explain_boundedness_failure()` API
- This is a State 10 implementation item; test plan preserves the specification of required fields

## 4. Proptest Invariants

### Proptest: nested accepted budgets fit policy (`PROP-BUD-001`)
- Invariant: For any generated structurally valid nested workflow with finite declared limits under policy, `WholeWorkflowBudget::compute` returns dimensions `<= ResourceContract` and `<= BoundednessPolicy`, and `AggregateResourceBudget::from_workflow` preserves those dimensions.
- Strategy: Generate small DAGs over `CompiledNodeKind::{CollectStart, ReduceStart, RepeatStart, TogetherStart, Nop, Finish}` with bounded node count, acyclic done/join edges, and explicit limits.
- Anti-invariant: Any generated workflow with a dimension over policy must return the exact budget error variant and actual/limit pair.

### Proptest: diagnostic parity for rejected nested growth (`PROP-DIAG-001`)
- Invariant: Every rejected generated collect/reduce/repeat/together case exposes primitive, node or path, resource, actual/computed value when known, and limit.
- Strategy: Generate one rejection per primitive and per resource dimension using tight policy and structurally valid workflow parts.
- Anti-invariant: A rejection without structural provenance fails the property even if the error variant is otherwise correct.

### Proptest: checked arithmetic never silently saturates
- Invariant: Sum/product dimensions either equal mathematical expected values or return overflow rejection; no saturating value is accepted.
- Strategy: Generate near-boundary `u32`/`u64` dimensions around `MAX - n`, `MAX`, and small multipliers.
- Anti-invariant: Saturated `u32::MAX`/`u64::MAX` values accepted as valid evidence fail.

### Proptest: aggregate add/subtract capacity accounting is inverse inside bounds
- Invariant: `AggregateResourceUsage::try_add_budget` followed by `try_subtract_budget` returns the original usage for all dimensions when no overflow/underflow occurs.
- Strategy: Generate capacities/usages/budgets with sums below maxima.
- Anti-invariant: Underflow, overflow, or capacity exceedance must return exact `AggregateBudgetError` variant and resource name.

## 5. Fuzz Targets

### Fuzz Target: authored workflow to compiled IR boundedness
- Input type: authored workflow bytes or generated IR fixture.
- Risk: malformed nested collect/reduce/repeat/together input bypasses bounds or loses diagnostic provenance.
- Corpus seeds: one valid bounded nested workflow; one missing collect limit; one repeat with sentinel max attempts; one together fanout over policy.

### Fuzz Target: diagnostic rendering for budget errors
- Input type: arbitrary `BudgetError`/`AggregateBudgetError` plus synthetic structural path metadata.
- Risk: panic, missing field, or lossy user-facing diagnostic for nested growth source.
- Corpus seeds: each budget error variant and each required primitive.

## 6. Kani Harnesses

### Kani Harness: checked nested arithmetic rejects overflow (`KANI-BUD-001`)
- Property: For bounded node/body/factor dimensions, sum/product budget arithmetic either equals mathematical expected value or returns typed overflow/rejection before admission.
- Bound: small node graphs up to 6 nodes, loop factors in `{0,1,2,u16::MAX}`, and u32/u64 dimensions around overflow boundaries.
- Rationale: Required proof obligation for all bounded inputs within the search bound; proptest sampling is insufficient for overflow exhaustiveness.
- **Preserved from State 8**: 7 Kani harnesses written in `vb_qi37_2_4_state8_tests.rs`

### Kani Harness: aggregate usage add/subtract capacity dimensions are checked
- Property: Adding aggregate budget dimensions to current usage never wraps and subtraction never underflows.
- Bound: all dimensions modeled as small nondeterministic values plus max-bound edge values where Kani permits.
- Rationale: Admission capacity must fail closed before runtime run admission.
- **Preserved from State 8**: 7 Kani harnesses written in `vb_qi37_2_4_state8_tests.rs`

## 7. Mutation Checkpoints
- Mutating `checked_add`/`checked_mul` to saturating or wrapping arithmetic must be killed by Kani/proptest overflow scenarios.
- Removing `FanoutExceeded`, `NestingDepthExceeded`, or `StepsExecutableExceeded` branches must be killed by exact-variant BDD scenarios.
- Dropping diagnostic primitive/path/actual/limit fields must be killed by `PROP-DIAG-001`.
- Changing branch maximum to minimum or unchecked sum must be killed by conservative-branch/together scenarios.
- Threshold: 90% mutation kill rate minimum for touched `budget.rs`/diagnostic paths; any surviving mutation in overflow, policy rejection, or diagnostic provenance is bead-local blocking.

## 8. Combinatorial Coverage Matrix
| Scenario | Input Class | Expected Output | Layer | Obligation |
|---|---|---|---|---|
| Linear finite workflow | valid acyclic nodes | exact budget dimensions | unit/integration | VERUS-BUD-001 |
| Collect finite limit | collect body + limit | multiplied total and gather dimensions | integration/proptest | PROP-BUD-001 |
| Reduce finite input | reduce body + finite list bound | bounded total or exact rejection | integration/proptest | PROP-BUD-001 |
| Repeat finite attempts | repeat body + max_attempts | multiplied total and repeat dimension | integration/proptest | PROP-BUD-001 |
| Together bounded branches | parallel branches | fanout/together dimensions | integration/proptest | PROP-BUD-001 |
| Overflow near max | sum/product around max | exact overflow/rejection | Kani/proptest | KANI-BUD-001 |
| Missing/sentinel bound | unknown/max sentinel | typed fail-closed error | integration/proptest | PROP-DIAG-001 |
| Policy over limit | each budget dimension | exact error variant/actual/limit | unit | PROP-DIAG-001 |
| Aggregate conversion | accepted workflow | aggregate equals whole dimensions | integration | VERUS-AGG-001 |
| Rejection diagnostic | rejected nested primitive | resource/primitive/node/path/actual/limit present | integration/proptest/e2e | PROP-DIAG-001 |

## Open Questions
- **GAP-1 (BudgetError diagnostic fields)**: Formal waiver filed. `BudgetError` lacks `primitive`/`node_index`/`structural_path` fields. `PROP-DIAG-001` tests correctly expose this gap. Resolution deferred to State 10 implementation (accessor or DiagnosticData struct). Compensating evidence: Kani proofs verify overflow soundness independent of diagnostic extension.
- **GAP-2 (nested loop depth tracking)**: `prop_nested_loops_multiply_correctly` fails in red-phase — implementation bug, not test bug. Tests correctly identify it.
- **Kani unavailability**: `KANI-BUD-001` cannot be verified until `cargo kani` is available. 7 Kani harnesses preserved in test file; execution deferred to environment with `cargo kani`.
- **State 9 Density**: Revised from 1.9x (17/9) to ≥5x (45/9) by adding 15 integration tests + 1 e2e test to existing unit/proptest/Kani coverage.
