# TLA+ Specification — vb-qi37.2.5

## Bead Identity
- **Bead**: vb-qi37.2.5
- **Title**: quality: Boundedness adversarial tests
- **State**: 5 (proof-writer repair)

## TLA+ Scope Determination

### Scope Analysis
This bead covers **boundedness** properties of vb_core's budget, value store, and engine loop:

1. **StepBudget** — bounded counter that never exceeds `MAX_STEP_BUDGET` (10,000)
2. **ValueStore** — arena cap enforcement before exceeding `max_arena_entries`
3. **run_until_blocked** — deterministic loop that terminates when budget exhausts

### Temporal Behavior Assessment

| Component | Has Temporal Behavior? | Concurrent State? | State Machines? |
|-----------|----------------------|-------------------|----------------|
| StepBudget | NO | NO | NO |
| ValueStore | NO | NO | NO |
| run_until_blocked | NO | NO | NO |
| EngineSignal | NO | NO | NO |

### Conclusion: No TLA+ Required

**Rationale**: All boundedness properties are **datalog-style invariant proofs** over deterministic
finite-state systems. There is no:

- Concurrency or parallelism
- Nondeterministic scheduling
- Liveness requirements (termination is a pure bound, not temporal)
- Fairness or progress properties
- State-machine workflows with external choices

The `run_until_blocked` loop is a **structurally recursive deterministic loop** with a
verified upper bound (`MAX_STEP_BUDGET`). The Verus loop invariant `INV-004` proves:

```
∀budget, iterations • budget.remaining = initial - iterations
                      ∧ iterations ≤ initial
                      ∧ budget.remaining ≥ 0
                      ∧ (budget.remaining = 0 ⇒ loop terminates)
```

This is not a temporal property — it is a pure first-order invariant over natural numbers.

## Compensation for TLA+ Absence

| Property | Primary Proof | Complementary Evidence |
|----------|--------------|----------------------|
| INV-001 (StepBudget bounds) | Verus spec/Proof | Kani harness |
| INV-002 (ValueStore cap) | Verus spec/Proof | Kani harness |
| INV-003 (count_total_steps bound) | Verus spec/Proof | Kani harness |
| INV-004 (loop termination) | Verus loop invariant | Kani harness |
| INV-005 (budget monotonic) | Verus spec/Proof | proptest |
| INV-006 (try_take monotonic) | Verus spec/Proof | Kani harness |

## Waiver

**TLA+ waiver granted** per verification-layers.md Section "Waiver":
- Single-threaded deterministic execution
- No liveness/deadlock/fairness concerns
- Termination proven by verified loop bound
- Compensating evidence: Verus INV-004 + Kani harness structural verification

**Owner**: State 3 (Contract and type model)
**Reviewer**: proof-reviewer

## Artifact Cross-Reference

- Verus specs: `verification/verus/*.rs`
- Kani harnesses: `crates/vb_core/src/kani/*.rs`
- Verification layers: `.beads/vb-qi37.2.5/verification-layers.md`
