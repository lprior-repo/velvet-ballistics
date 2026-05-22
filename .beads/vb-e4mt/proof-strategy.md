# Proof Strategy — vb-e4mt: Resource Bounds and Budget Enforcement

## Bead Overview
- **Bead**: vb-e4mt
- **State**: 4 (proof planning)
- **Source checkout**: /home/lewis/src/velvet-ballistics
- **Isolated workspace**: /home/lewis/src/vb-e4mt-workspace
- **Scope cluster**: resource-bounds-budget-enforcement

## Risk Classification

| Risk | Category | Severity | Justification |
|------|----------|----------|---------------|
| Arithmetic overflow in budget computation | arithmetic | HIGH | u64 dimension overflow causes policy bypass |
| BoundednessPolicy validation completeness | validation | HIGH | Wrong error variant = incorrect admission decision |
| Step budget exhaustion timing | temporal | HIGH | Signal raised AFTER budget consumed = unsafe state |
| Expression stack overflow | bounded-state | HIGH | Stack depth > 64 = memory corruption risk |
| Aggregate usage overflow | arithmetic | HIGH | try_add_budget overflow = capacity violation |
| TLA spec completeness | temporal | MEDIUM | Missing specs = unverified temporal properties |
| GAP-001 BudgetError fields | validation | MEDIUM | Missing BLOCK_LOCAL fields; spec nonconformance |

## Discovery Evidence

```
$ grep -c "unsafe" vb_core/src/budget.rs
0  (file-level #![forbid(unsafe_code)])

$ grep -n "unwrap" vb_core/src/budget.rs
1414:u64::try_from(count).unwrap_or(u64::MAX)

$ grep -c "kani::proof\|verus::proof\|requires\|ensures" vb_core/src/budget.rs
13 (Verus proof markers present)

TLA specs at expected paths:
  specs/WorkflowBudgetSpec.tla      MISSING
  specs/AggregateResourceSpec.tla  MISSING
  specs/StepBudgetSpec.tla         MISSING (StepBudgetSuspension.tla EXISTS)

Verus specs:
  verification/verus/budget_bounded.rs     EXISTS
  verification/verus/budget_monotonic.rs   EXISTS
  verification/verus/resource_budget.rs    EXISTS

Kani harnesses:
  kani_budget_arithmetic_refinement.rs     EXISTS
  kani_resource_budget_bounded.rs          EXISTS
  kani_step_budget*.rs (5 files)          EXISTS

Proof kernel:
  vb_proof_kernels/src/resource_budget.rs  EXISTS (pure, #![forbid(unsafe_code)])
```

## Assumptions and Bounds

| Assumption | Bound | Source |
|------------|-------|--------|
| CompiledNode slice is finite | len <= u16::MAX | contract PRE-001 |
| ResourceContract dimensions are nonzero | min = 1 where zero = unbounded | contract PRE-002 |
| Frame pool key space | (u16::MAX, u16::MAX) per shard/tier | contract PRE-003 |
| Step budget is positive u64 at admission | min = 1 | contract PRE-004 |
| Expression stack max depth | <= 64 at gate 7 | contract PRE-005 |
| Budget computation loop nest | bounded by CompiledNode count | contract PRE-001 |
| Policy limits | max_total_steps=1M, max_slots=65535, max_fanout=64, max_nesting=8 | contract §Policy Limits |

## Verifier Lane Strategy

### Lane 1: TLA+ (TEMPORAL)
**Status**: 3 obligations; 3 MISSING artifacts

| Obligation | Spec | Discovery | Action |
|------------|------|-----------|--------|
| TLA-WF-001 | specs/WorkflowBudgetSpec.tla | MISSING | proof-writer must create |
| TLA-WF-002 | specs/AggregateResourceSpec.tla | MISSING | proof-writer must create |
| TLA-WF-003 | specs/StepBudgetSpec.tla | MISSING | proof-writer must create |

**Blocker**: TLA specs do not exist at referenced paths. These are NOT optional — temporal properties of workflow admission and step budget exhaustion require TLA+ verification per contract §TLA+-Owned Clauses.

**Rerun guidance**: After proof-writer creates specs, run:
```bash
tlc -config specs/WorkflowBudgetSpec.cfg specs/WorkflowBudgetSpec.tla
tlc -config specs/AggregateResourceSpec.cfg specs/AggregateResourceSpec.tla
tlc -config specs/StepBudgetSpec.cfg specs/StepBudgetSpec.tla
```

### Lane 2: Verus (RUST-LOCAL INVARIANT + ARITHMETIC)
**Status**: 6 obligations; 3 existing + 3 need creation

| Obligation | Target | Discovery | Action |
|------------|--------|-----------|--------|
| VERUS-BUDGET-001 | WholeWorkflowBudget::compute | EXISTS (budget.rs + verus/budget_bounded.rs) | verify scope matches |
| VERUS-BUDGET-002 | WholeWorkflowBudget::compute (finite output) | EXISTS | verify scope matches |
| VERUS-BUDGET-003 | BoundednessPolicy::validate | EXISTS (budget.rs + verus/budget_bounded.rs) | verify scope matches |
| VERUS-BUDGET-004 | AggregateResourceUsage::try_add_budget | EXISTS (budget.rs) | verify scope matches |
| VERUS-BUDGET-005 | AggregateResourceUsage::fits_within | EXISTS (budget.rs) | verify scope matches |
| VERUS-BUDGET-006 | check_expr_stack_bound | EXISTS (budget.rs + verus/resource_budget.rs) | verify scope matches |

**Rerun guidance**:
```bash
verus crates/vb_core/src/budget.rs
verus crates/vb_proof_kernels/src/resource_budget.rs
```

### Lane 3: Kani (BOUNDED MODEL CHECKING)
**Status**: 5 obligations; 5 harness files exist

| Obligation | Harness | Discovery | Action |
|------------|---------|-----------|--------|
| KANI-BUDGET-001 | kani_harness_whole_workflow_budget_compute | kani_workflow_arbitrary.rs EXISTS | verify harness name matches |
| KANI-BUDGET-002 | kani_harness_boundedness_policy_validate | kani_resource_budget_bounded.rs EXISTS | verify harness name matches |
| KANI-BUDGET-003 | kani_harness_try_add_budget_no_overflow | kani_budget_arithmetic_refinement.rs EXISTS | verify harness name matches |
| KANI-BUDGET-004 | kani_harness_fits_within_exact | kani_budget_arithmetic_refinement.rs EXISTS | verify harness name matches |
| KANI-BUDGET-005 | kani_harness_step_budget_consume | kani_step_budget*.rs (5 files) EXISTS | verify harness name matches |

**Rerun guidance**:
```bash
cargo kani -p vb_core --harness kani_harness_whole_workflow_budget_compute
cargo kani -p vb_core --harness kani_harness_boundedness_policy_validate
cargo kani -p vb_core --harness kani_harness_try_add_budget_no_overflow
cargo kani -p vb_core --harness kani_harness_fits_within_exact
cargo kani -p vb_core --harness kani_harness_step_budget_consume
```

### Lane 4: Proptest (ARBITRARY INPUT)
**Status**: 4 obligations

| Obligation | Target | Command |
|------------|--------|---------|
| PROP-BUDGET-001 | sequential_compose respects Policy | cargo test -p vb_proof_kernels proptest_sequential_compose_within_policy |
| PROP-BUDGET-002 | branch_compose respects Policy | cargo test -p vb_proof_kernels proptest_branch_compose_within_policy |
| PROP-BUDGET-003 | loop_compose respects Policy | cargo test -p vb_proof_kernels proptest_loop_compose_within_policy |
| PROP-BUDGET-004 | expr stack depth matches ops | cargo test -p vb_core proptest_expr_stack_bound_matches_ops |

### Lane 5: Fuzz (ADVERSARIAL INPUT)
**Status**: 1 obligation

| Obligation | Target | Command |
|------------|--------|---------|
| FUZZ-BUDGET-001 | check_expr_stack_bound | cargo fuzz run -p vb_core fuzz_parse_expression_ops -- -runs=10000 |

### Lane 6: Integration (WORKSPACE)
**Status**: 2 obligations

| Obligation | Target | Command |
|------------|--------|---------|
| INTEGRATION-001 | vb_qi37_2_4_integration_budget_errors | cargo test -p velvet-ballastics-workspace --test vb_qi37_2_4_integration_budget_errors |
| INTEGRATION-002 | CLI budget enforcement | cargo test -p vb_cli --test cli_verify_integration |

### Lane 7: BDD (BEHAVIOR-DRIVEN)
**Status**: 6 obligations

| Obligation | Scenario | Command |
|------------|----------|---------|
| BDD-001 | TotalStepsExceeded | cargo test -p velvet-ballastics-workspace bdd_unbounded_for_each_rejected |
| BDD-002 | FanoutExceeded | cargo test -p velvet-ballastics-workspace bdd_fanout_exceeded_rejected |
| BDD-003 | GatherItemsExceeded | cargo test -p velvet-ballastics-workspace bdd_collect_unlimited_rejected |
| BDD-004 | ExpressionStackExceeded | cargo test -p velvet-ballastics-workspace bdd_expr_stack_exceeded_rejected |
| BDD-005 | StepBudgetExhausted | cargo test -p velvet-ballastics-workspace bdd_step_budget_exhausted_signal |
| BDD-006 | CapacityExceeded | cargo test -p velvet-ballastics-workspace bdd_aggregate_capacity_exceeded |

### Lane 8: Gauntlet (CI GATE)
**Status**: 2 obligations (deferred to owner_state=12)

| Obligation | Command |
|------------|---------|
| GATE-PROOF-001 | moon run :verify-proof |
| GATE-STANDARD-001 | moon run :verify-standard |

## Waiver Candidates

| ID | Obligation | Reason | Owner | Compensating Evidence | Follow-up |
|----|-----------|--------|-------|----------------------|-----------|
| WAIVE-OQ-001 | GAP-001 | BudgetError BLOCK_LOCAL fields missing — spec nonconformance; resolution deferred to vb_qi37_2_4 BLOCK_LOCAL spec completion | unassigned | existing tests cover 9 BudgetError variants | OQ-001 must resolve before GATE-PROOF-001 |
| WAIVE-OQ-002 | BoundednessPolicy CI coverage | OQ-002: validation completeness not fully evidenced in CI; proptest + Kani provide coverage | unassigned | KANI-BUDGET-002 exercises all 8 error variants | OQ-002 follow-up in CI enhancement bead |
| WAIVE-OQ-003 | Expression stack Gate 7 coverage | OQ-003: test coverage completeness unknown for Gate 7 | unassigned | FUZZ-BUDGET-001 + VERUS-BUDGET-006 provide coverage | OQ-003 follow-up in test coverage bead |

## Proof Budget Summary

| Lane | Obligations | Estimated Cost | Blocker |
|------|-------------|----------------|---------|
| TLA+ | 3 | HIGH (spec creation + model checking) | YES - MISSING specs |
| Verus | 6 | MEDIUM | No |
| Kani | 5 | MEDIUM | No |
| Proptest | 4 | LOW | No |
| Fuzz | 1 | MEDIUM | No |
| Integration | 2 | LOW | No |
| BDD | 6 | LOW | No |
| Gauntlet | 2 | HIGH (CI) | No |

## Critical Path

1. **TLA+ specs MUST be created first** — temporal invariants (INV-001, INV-002, POST-006) cannot be verified without them
2. Verus can proceed in parallel with TLA+ spec creation
3. Kani can proceed in parallel with Verus
4. BDD/Integration are end-to-end and require full implementation

## Open Questions (must resolve before GATE-PROOF-001)

- **OQ-001**: BudgetError lacks `primitive`, `node_index`, `structural_path` fields per BLOCK_LOCAL spec
- **OQ-002**: BoundednessPolicy validation completeness — documented but not fully evidenced in CI
- **OQ-003**: Expression stack depth enforcement (Gate 7) — test coverage completeness unknown
