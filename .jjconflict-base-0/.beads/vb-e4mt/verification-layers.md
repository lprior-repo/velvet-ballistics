# Verification Layers — vb-e4mt

**Bead**: vb-e4mt — bdd: Resource bounds and budget enforcement acceptance scenarios
**State**: 3 (contract)

---

## Boundary Summary

| Layer | Owner | Scope |
|-------|-------|-------|
| TLA+ temporal model | `WorkflowBudgetSpec`, `AggregateResourceSpec`, `StepBudgetSpec` | Admission boundedness, aggregate lifecycle, step exhaustion signaling |
| Verus proof | `WholeWorkflowBudget::compute`, `BoundednessPolicy::validate`, `AggregateResourceUsage` | Pure Rust core; entry bounds, overflow freedom, exact error variants |
| Lean theorem | `vb_proof_kernels::resource_budget` | Sequential/branch/loop composition soundness |
| Kani bounded model check | Budget arithmetic, state transitions | Numeric overflow, index bounds, panic freedom |
| Miri | Budget operations | UB detection, Stacked Borrows |
| Proptest | Budget composition | Exhaustively generated inputs for composition functions |
| Cargo fuzz | Expression program parsing | Malformed ops, stack overflow attempts |
| Loom/Shuttle | Frame pool concurrent access | Thread interleavings, race conditions |
| Integration tests | Full BDD scenarios | Happy/error/edge coverage per acceptance criteria |
| Manual QA | End-to-end budget enforcement | Actual CLI verify runs |

---

## Layer Assignment Matrix

| Contract Clause | Primary Layer | Secondary Layers | Evidence |
|-----------------|---------------|------------------|----------|
| INV-001 (workflow boundedness) | TLA+ | Verus + Kani | `WorkflowBudgetSpec` model + `WholeWorkflowBudget::compute` proof |
| INV-002 (aggregate non-overflow) | Verus | Kani + TLA+ | `AggregateResourceUsage::try_add_budget` proof + `AggregateResourceSpec` model |
| INV-003 (frame pool boundedness) | type bounds | integration test | `FramePoolKey = (u16, u16)` trivially finite |
| INV-004 (expr stack boundedness) | Verus | cargo fuzz | `check_expr_stack_bound` proof + Gate 7 fuzz |
| INV-005 (step budget monotonic) | TLA+ | Kani | `StepBudgetSpec` model + `checked_sub` harness |
| INV-006 (BudgetError exhaustiveness) | TLA+ | Kani | `WorkflowBudgetSpec` + exhaustive variant harness |
| PRE-001 (compute entry bounds) | Verus | Kani | entry index bounds proof |
| PRE-002 (ResourceContract finiteness) | Verus | proptest | `ResourceContract` construction invariants |
| POST-001 (finite budget output) | Verus | Kani | `WholeWorkflowBudget::compute` proof |
| POST-002 (exact error variant) | Verus | Kani | `BoundednessPolicy::validate` proof |
| POST-003 (aggregate admit) | Verus | Kani | `try_add_budget` overflow harness |
| POST-004 (fits_within) | Verus | Kani | capacity check harness |
| POST-006 (step budget signal) | TLA+ | integration | `StepBudgetSpec` + `EngineSignal::StepBudgetExhausted` tests |

---

## Verus Scope

**Rust targets**:
- `vb_core::budget::WholeWorkflowBudget::compute`
- `vb_core::budget::BoundednessPolicy::validate`
- `vb_core::budget::AggregateResourceUsage::try_add_budget`
- `vb_core::budget::AggregateResourceUsage::try_sub_budget`
- `vb_core::budget::AggregateResourceUsage::fits_within`
- `vb_core::workflow::check_expr_stack_bound`
- `vb_proof_kernels::resource_budget::sequential_compose`
- `vb_proof_kernels::resource_budget::branch_compose`
- `vb_proof_kernels::resource_budget::loop_compose`
- `vb_proof_kernels::resource_budget::Policy::within`

**Spec/proof functions**:
- `spec_whole_workflow_budget_compute` — abstract IR walk model
- `proof_compute_preserves_boundedness` — `WholeWorkflowBudget::compute` output satisfies policy
- `proof_validate_returns_exact_error` — `BoundednessPolicy::validate` returns exact variant
- `proof_try_add_budget_no_overflow` — `try_add_budget` returns `Ok` or `Overflow`
- `proof_try_sub_budget_no_underflow` — `try_sub_budget` returns `Ok` or `Underflow`
- `proof_fits_within_exact` — `fits_within` is exact capacity comparison
- `proof_expr_stack_bound_finite` — `check_expr_stack_bound` never exceeds 64

**Trusted boundary**: Validated `CompiledWorkflow` with `ResourceContract` from `vb_compile`; finite `CompiledNode` slice; `BoundednessPolicy::DEFAULT` as global ceiling.

**Shell exclusions**: I/O, async scheduling, storage, wall-clock time, network, FFI.

---

## TLA+ Scope

**Module**: `WorkflowBudgetSpec.tla`, `AggregateResourceSpec.tla`, `StepBudgetSpec.tla`

**Variables**:
- `WorkflowBudgetSpec`: `workflowBudget`, `policy`, `admitted`
- `AggregateResourceSpec`: `usage`, `capacity`, `reservations`, `pending`
- `StepBudgetSpec`: `stepBudget`, `stepsExecuted`, `signal`

**Actions**:
- `ComputeBudget`, `ValidateAgainstPolicy`, `AdmitWorkflow`, `RejectWorkflow`
- `RequestAdmission`, `AdmitRun`, `ReleaseRun`, `RejectAdmission`
- `ConsumeSteps`, `ExhaustBudget`

**Safety invariants**:
- `InvAdmission`: admitted => workflowBudget valid + passes policy
- `InvNoOverflow`: usage dimensions <= capacity dimensions
- `InvExhaustionBeforeSteps`: exhaustion signal precedes over-budget steps

**Temporal properties**:
- `WF_AdmitRun`: weak fairness on admit
- `WF_ReleaseRun`: weak fairness on release
- `EventuallyTerminal`: every run eventually admits or rejects

**Evidence command**:
```bash
tlc -config specs/WorkflowBudgetSpec.cfg specs/WorkflowBudgetSpec.tla
tlc -config specs/AggregateResourceSpec.cfg specs/AggregateResourceSpec.tla
tlc -config specs/StepBudgetSpec.cfg specs/StepBudgetSpec.tla
```

---

## Kani Scope

**Harnesses**:
- `kani_harness_whole_workflow_budget_compute` — arbitrary `CompiledNode` slice, arbitrary `StepIdx`, arbitrary `ResourceContract`; proves no panic, entry bounds respected
- `kani_harness_boundedness_policy_validate` — arbitrary `WholeWorkflowBudget`; proves each error variant returned exactly when expected
- `kani_harness_try_add_budget_no_overflow` — arbitrary `AggregateResourceUsage`, `AggregateResourceBudget`; proves `Ok` or `AggregateBudgetError::Overflow`
- `kani_harness_fits_within_exact` — arbitrary usage/capacity; proves exact boolean result
- `kani_harness_step_budget_consume` — arbitrary `StepBudget`, arbitrary `u64`; proves `Exhausted` variant on over-consumption

**Note**: GAP-1 — `BudgetError` missing `primitive`, `node_index`, `structural_path` fields may affect Kani harness completeness for error variant mapping.

---

## Proptest Scope

**Properties**:
- `proptest_sequential_compose_within_policy` — 1000 iterations: `sequential_compose(a, b)` respects policy when `a` and `b` individually respect it
- `proptest_branch_compose_within_policy` — same for branch max
- `proptest_loop_compose_within_policy` — same for loop multiplication
- `proptest_budget_saturation` — `saturating_add`/`saturating_mul` never overflows
- `proptest_expr_stack_bound_matches_ops` — random `ExprOp` sequences; computed depth matches declared depth

---

## Cargo Fuzz Scope

**Targets**:
- `fuzz_parse_expression_ops` — arbitrary `ExprOp` bytes; proves no stack overflow, bounds respected
- `fuzz_workflow_budget_compute` — arbitrary node/layout bytes; proves no panic on malformed IR

---

## Loom/Shuttle Scope

**Concurrent scenarios**:
- Frame pool acquire/release race — multiple threads acquiring same key
- Aggregate usage admit/release race — concurrent `try_add_budget`/`try_sub_budget`
- Step budget consume race — concurrent tick boundary reset vs. consume

Note: If shuttle tooling is unavailable, this falls back to TLA+ model + Miri for UB detection.

---

## Integration Test Scope (workspace_tests)

**Key test files**:
- `vb_qi37_2_4_integration_budget_errors.rs` — 15+ scenarios covering all `BudgetError` variants
- `vb_fzx7_budget_arithmetic.rs` — budget arithmetic correctness
- `bdd_validation_tests.rs` — BDD validation tests
- `cli_verify_integration.rs` — `bdd_full_profile_fails_closed_on_budget_violation`, `bdd_standard_profile_warns_not_fails_on_budget`

**BDD scenarios for resource bounds**:
1. Given a workflow with unbounded `for_each` — when validated — then reject with `TotalStepsExceeded`
2. Given a workflow exceeding `max_fanout` — when validated — then reject with `FanoutExceeded`
3. Given a workflow with `collect` without limits — when validated — then reject with `GatherItemsExceeded`
4. Given a workflow exceeding nesting depth — when validated — then reject with `NestingDepthExceeded`
5. Given a run consuming all step budget — when executing — then emit `StepBudgetExhausted`
6. Given aggregate usage at capacity — when new run requests admission — then reject with `CapacityExceeded`
7. Given expression stack at depth 64 — when evaluating — then accept
8. Given expression stack exceeds 64 — when evaluating — then reject with `ExpressionStackExceeded`

---

## Manual QA Scope

- `cargo run -- verify --full-profile` on representative workflows
- Verify budget metadata reported correctly in CLI output
- Verify step budget exhaustion produces expected error in logs

---

## Waivers

| Waiver ID | Clause | Reason | Compensating Evidence |
|-----------|--------|--------|----------------------|
| WAIVER-VRF-001 | INV-001 (workflow boundedness — TLA+) | `WorkflowBudgetSpec.tla` not yet written as formal .tla file | `WholeWorkflowBudget::compute` Verus proof + integration tests |
| WAIVER-VRF-002 | GAP-1 BudgetError fields | `BudgetError` lacks BLOCK_LOCAL fields; not blocking BDD acceptance | Existing `BudgetError` variant tests cover all 9 variants |
| WAIVER-VRF-003 | Loom/Shuttle frame pool | Shuttle tooling unavailable | Miri UB checks + type bounds + integration tests |
| WAIVER-VRF-004 | Lean extraction | Bead scope is BDD acceptance, not theorem extraction | Verus proofs + proptest cover kernel composition functions |
