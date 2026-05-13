# TLA+ Temporal Model Plan: vb-qi37.2.1 — Aggregate Resource Budget Model

## Non-applicability Rationale

The aggregate resource budget model is entirely local to the Rust type system and has **no temporal, workflow, protocol, scheduler, queue, retry, claim/lease, lifecycle, concurrent, or distributed behavior**. All operations are:

1. Pure functions on value types (no side effects).
2. Deterministic arithmetic on fixed-width integers.
3. Stateless comparisons with no memory of past invocations.
4. Synchronous — no async, no scheduling, no concurrency.

Therefore, **no TLA+ model is required** for this bead. All critical properties are proven by:
- **Verus** — Rust-local pure proof of checked arithmetic invariants.
- **Kani** — Bounded model checking of symbolic budget values.
- **Lean** — Theorem-proven kernel of arithmetic properties.
- **Unit/integration tests** — Concrete value coverage.

## TLA+-Owned Clauses

None. TLA+ is not applicable to this bead's scope.

## Alternative Verification

If a future bead introduces temporal budget enforcement (e.g., per-tick budget consumption tracking, budget exhaustion liveness, or concurrent admission races), the following TLA+ model would be appropriate:

```
MODULE BudgetTemporal

VARIABLES
  available   \* AggregateResourceCapacity snapshot
  reservations \* Map RunId -> AggregateResourceBudget
  active       \* Set of active RunIds

Invariant: SumOfReservations <= Available
Liveness: Every admitted run eventually releases (finish/fail/cancel)
Fairness: No run holds reservation forever without progress
```

However, this is **out of scope** for vb-qi37.2.1.
