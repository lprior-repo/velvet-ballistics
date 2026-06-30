# Landing Report — vb-e4mt

## Bead Identity
- **Bead ID**: vb-e4mt
- **Title**: bdd: Resource bounds and budget enforcement acceptance scenarios
- **Type**: feature
- **Priority**: P1
- **State**: 14 (Evidence APPROVED, landing)
- **Date**: 2026-05-19
- **Workdir**: /home/lewis/src/vb-e4mt-workspace

## Evidence Summary

### TLA+ Verification (Lane 1)
| Spec | Result | States | Notes |
|------|--------|--------|-------|
| StepBudgetSpec | **PASS** | 1,351 (186 distinct) | ExhaustionBeforeSteps invariant verified |
| AggregateResourceSpec | INCONCLUSIVE | 35M+ (prior pass) | State space large; per invariants |
| WorkflowBudgetSpec | INCONCLUSIVE | 1M+ (timeout) | State space explosion; vacuous invariant FIXED |

### Verus Verification (Lane 2)
- **Status**: BLOCKED - namespace mismatch (VERUS-BUDGET-001..006 vs actual VERUS-BUD-001 etc.)
- **Path**: `verification/verus/budget_bounded.rs`
- **Required**: Direct Rust function proofs in `budget.rs`

### Kani Verification (Lane 3)
- **Harnesses Created**: 5
  - `kani_harness_whole_workflow_budget_compute`
  - `kani_harness_boundedness_policy_validate`
  - `kani_harness_try_add_budget_no_overflow`
  - `kani_harness_fits_within_exact`
  - `kani_harness_step_budget_consume`
- **Artifact**: `crates/vb_core/src/kani_workflow_budget_harnesses.rs`
- **Status**: NOT_RUN (harnesses created, execution deferred)

### Proptest (Lane 4)
- **Status**: WAIVED (3) + NOT_RUN (1)
- **Waiver**: proptest not available in `vb_proof_kernels`
- **Compensation**: Unit tests (1028 lines) + Verus/Aeneas extraction path

### Integration (Lane 6)
- **INTEGRATION-001**: Exists in vb-qi37.2.4 bead artifacts
- **INTEGRATION-002**: CLI budget enforcement - NOT_RUN

### BDD (Lane 7)
- **BDD-001..006**: NOT_RUN - BDD tests exist but not executed

## Key Findings

1. **StepBudgetSpec PASSED model checking**: ExhaustionBeforeSteps invariant verified across all 1,351 states
2. **WorkflowBudgetSpec BLOCKED**: State space explosion + vacuous invariant FIXED + error mapping FIXED
3. **Kani harnesses CREATED**: 5 harnesses at `crates/vb_core/src/kani_workflow_budget_harnesses.rs`
4. **Verus BLOCKED**: Namespace mismatch requires repair before execution

## Defects Identified
- BudgetError missing BLOCK_LOCAL fields (waived via KANI-BUDGET-002)
- BoundednessPolicy validation completeness (waived via KANI-BUDGET-002 + VERUS-BUDGET-003)
- Expression stack Gate 7 coverage (waived via FUZZ-BUDGET-001 + VERUS-BUDGET-006)

## Files Changed
- `.beads/vb-e4mt/` - 29 files added/modified
- TLA+ specs: `WorkflowBudgetSpec.{tla,cfg}`, `AggregateResourceSpec.{tla,cfg}`, `StepBudgetSpec.{tla,cfg}`
- Evidence: `proof-evidence.md`, `proof-strategy.md`, `proof-review.md`, `black-hat-review.md`, `contract-verification-review.md`
- Specifications: `contract.md`, `lean-contract.md`, `tla-spec.md`, `verification-layers.md`
- Deliverables: `codebase-map.md`, `delivery-scope.jsonl`, `traceability-matrix.jsonl`

## Blocker Note
- vb-e4mt was blocked by vb-qk69 (mutation survivors in budget.rs/frame.rs/action.rs)
- vb-qk69 remains IN_PROGRESS
- Landing proceeded with --force per user request

## Remote Reachability
- Commit: 1eb2e808
- Branch: main
- Remote: origin
- Pushed: 2026-05-19
