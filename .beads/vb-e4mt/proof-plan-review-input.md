# Proof Plan Review Input — vb-e4mt

## Bead
vb-e4mt: Resource bounds and budget enforcement

## State
4 → (proof planning)

## Source Checkout
/home/lewis/src/velvet-ballistics

## Scope Cluster
resource-bounds-budget-enforcement

---

## Risk Summary

| Risk | Category | Sev | Verifier Lane |
|------|----------|-----|---------------|
| Budget overflow (arithmetic) | arithmetic | HIGH | Kani + Verus |
| BoundednessPolicy wrong error | validation | HIGH | Verus + TLA+ |
| Step budget exhaustion timing | temporal | HIGH | TLA+ + Kani |
| Expression stack overflow | bounded-state | HIGH | Kani + Fuzz |
| Aggregate usage overflow | arithmetic | HIGH | Verus + Kani |
| Missing TLA specs | temporal | MEDIUM | TLA+ (BLOCKED) |

---

## Verifier Lane Coverage

| Lane | Count | BLOCKED? |
|------|-------|----------|
| TLA+ | 3 | YES — specs MISSING |
| Verus | 6 | No |
| Kani | 5 | No |
| Proptest | 4 | No |
| Fuzz | 1 | No |
| Integration | 2 | No |
| BDD | 6 | No |
| Gauntlet | 2 | No (deferred) |

---

## Critical Findings

### BLOCKER: TLA+ Specs Missing
- `specs/WorkflowBudgetSpec.tla` — NOT FOUND
- `specs/AggregateResourceSpec.tla` — NOT FOUND
- `specs/StepBudgetSpec.tla` — NOT FOUND

These specs are required by the contract §TLA+-Owned Clauses for:
- INV-001 (temporal safety of WholeWorkflowBudget admission)
- INV-002 (AggregateResourceUsage never exceeds capacity)
- POST-006 (StepBudgetExhausted raised BEFORE over-consumption)

**Recommended action**: proof-writer MUST create these 3 specs before TLA+ lane can execute.

### Non-Blocker: GAP-001 (BudgetError fields)
BudgetError missing `primitive`, `node_index`, `structural_path` fields per BLOCK_LOCAL spec. Open question OQ-001 recorded. Waiver candidate identified.

### Non-Blocker: OQ-002, OQ-003
Coverage completeness issues documented; compensating evidence from Kani + Fuzz lanes identified.

---

## Obligation Map (key obligations only)

| ID | Clause | Risk | Artifact | Status |
|----|--------|------|----------|--------|
| TLA-WF-001 | INV-001 | temporal | specs/WorkflowBudgetSpec.tla | MISSING |
| TLA-WF-002 | INV-002 | arithmetic | specs/AggregateResourceSpec.tla | MISSING |
| TLA-WF-003 | POST-006 | temporal | specs/StepBudgetSpec.tla | MISSING |
| VERUS-BUDGET-001 | PRE-001 | arithmetic | budget.rs + verus/budget_bounded.rs | EXISTS |
| VERUS-BUDGET-002 | POST-001 | arithmetic | budget.rs | EXISTS |
| VERUS-BUDGET-003 | POST-002 | validation | budget.rs | EXISTS |
| VERUS-BUDGET-004 | POST-003 | arithmetic | budget.rs | EXISTS |
| VERUS-BUDGET-005 | POST-004 | validation | budget.rs | EXISTS |
| VERUS-BUDGET-006 | INV-004 | bounded-state | budget.rs + verus/resource_budget.rs | EXISTS |
| KANI-BUDGET-001 | PRE-001 | arithmetic | kani_workflow_arbitrary.rs | EXISTS |
| KANI-BUDGET-002 | POST-002 | validation | kani_resource_budget_bounded.rs | EXISTS |
| KANI-BUDGET-003 | POST-003 | arithmetic | kani_budget_arithmetic_refinement.rs | EXISTS |
| KANI-BUDGET-004 | POST-004 | validation | kani_budget_arithmetic_refinement.rs | EXISTS |
| KANI-BUDGET-005 | INV-005 | temporal | kani_step_budget*.rs | EXISTS |
| PROP-BUDGET-001 | PRE-002 | arithmetic | vb_proof_kernels | EXISTS |
| PROP-BUDGET-002 | PRE-002 | arithmetic | vb_proof_kernels | EXISTS |
| PROP-BUDGET-003 | PRE-002 | arithmetic | vb_proof_kernels | EXISTS |
| PROP-BUDGET-004 | INV-004 | bounded-state | vb_core | EXISTS |
| FUZZ-BUDGET-001 | INV-004 | bounded-state | fuzz_parse_expression_ops | EXISTS |

---

## Waiver Candidates

| ID | Reason | Compensating Evidence |
|----|--------|----------------------|
| WAIVE-OQ-001 | GAP-001: BudgetError missing BLOCK_LOCAL fields | KANI-BUDGET-002 exercises all 9 variants |
| WAIVE-OQ-002 | OQ-002: BoundednessPolicy CI coverage incomplete | KANI-BUDGET-002 + VERUS-BUDGET-003 |
| WAIVE-OQ-003 | OQ-003: Gate 7 coverage unknown | FUZZ-BUDGET-001 + VERUS-BUDGET-006 |

---

## Discovery Evidence (anti-hallucination)

```
budget.rs: #![forbid(unsafe_code)] — CONFIRMED
budget.rs line 1414: unwrap_or(u64::MAX) — single unwrap_or usage
TLA specs: 3 MISSING at specs/WorkflowBudgetSpec.tla, specs/AggregateResourceSpec.tla, specs/StepBudgetSpec.tla
Verus specs: EXISTS (budget_bounded.rs, budget_monotonic.rs, resource_budget.rs)
Kani harnesses: EXISTS (6 harness files in vb_core/src/)
Proof kernel: vb_proof_kernels/src/resource_budget.rs EXISTS, pure Rust, no unsafe
```

---

## Reviewer Questions

1. Should TLA-WF-001/002/003 be BLOCKED until specs are created, or should proof-writer create them as part of proof-writing phase?
2. Is the waiver for GAP-001 acceptable given compensating evidence from Kani-BUDGET-002?
3. Should OQ-001 block GATE-PROOF-001 or be deferred to a follow-up bead?
