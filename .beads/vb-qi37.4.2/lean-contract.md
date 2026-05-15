# Theorem Kernel Projection: vb-qi37.4.2

## Boundary

- **TLA+-owned temporal model**: Journal ordering, replay safety, concurrency, lifecycle state transitions — modeled in TLA+ (see `tla-spec.md`).
- **Verus-owned Rust core**: Taint lattice, StepBudget arithmetic, RunFrame invariants, StepState transitions, EngineSignal canonical form, resource budget composition — proven in Verus.
- **Theorem-owned kernel**: None. The taint lattice and resource budget arithmetic are fully expressible in Verus; no Lean/Aeneas/Hax extraction is required.
- **Rust/runtime shell**: I/O, async scheduling, Fjall storage, IPC transport, UI rendering — excluded from formal proof, covered by integration tests and fuzzing.
- **External systems excluded**: OS scheduler, network transport, hardware memory model.

---

## Theorem-Owned Clauses

- **None**. All Rust-local pure/core critical behavior is expressible in Verus at L4.

---

## Lean/Aeneas/Hax Non-Applicability Rationale

### Taint Lattice (INV-001 to INV-006)

The taint lattice is a 3-element (Clean, DerivedFromSecret, Secret) distributive lattice with join as the least upper bound operation. This is directly expressible in Verus:

```verus
spec fn join_taint(a: Taint, b: Taint) -> Taint {
    if a >= b { a } else { b }
}
```

Verus can prove all six lattice laws (associative, commutative, idempotent, identity, no-downgrade-Secret, no-downgrade-DerivedFromSecret) as `proof fn`s with inductive arguments over the 3-element enum. No Lean kernel extraction is needed.

### Resource Budget Arithmetic (VB-CORE-RESOURCE-001, VB-CORE-RESOURCE-002, VB-CORE-RESOURCE-003)

Sequential sum, branch max, and loop multiplication are pure arithmetic properties over bounded integers (u32, u64). Verus's integer theory support is sufficient:

- `sequential_sum`: `a + b` without overflow — proven via `u32::try_from(a as u64 + b as u64)`
- `branch_max`: `max(a, b)` — trivially proven
- `loop_multiply`: `a * b` without overflow — proven via `u64::try_from(a as u128 * b as u128)` with range preconditions

No theorem-assistant extraction to Lean is required. If future budget types grow more complex algebraic structure (e.g., affine types, resource usage monoids), the projection to Lean can be revisited.

### StepState Transition Matrix (INV-007, VB-CORE-STATE-001)

The StepState transition relation is a finite state machine with 8 states and a defined transition relation. Verus's `proof fn` over an inductive enum and a table of allowed transitions is sufficient to prove:
- All valid transitions are accepted
- All invalid transitions are rejected
- Terminal states are self-loop only

No Lean kernel is needed.

### RunFrame Dimension Immutability (INV-007)

`step_count` and `slot_count` are set at construction and preserved through `reinitialize`. Verus can express this as a class-level `ghost` invariant tracked in the type's representation. No Lean extraction required.

---

## Waivers

- **WAIVER-LEAN-01**: No theorem-owned clauses exist in vb-qi37.4.2 bead scope. All critical Rust-local pure properties are covered by Verus at L4 or Kani at L3. Lean/Aeneas/Hax projection is a non-goal for this bead.
- **WAIVER-LEAN-02**: If a future bead introduces algebraic structures (monoids, functors, lenses) that are poorly supported by Verus's SMT backend, that bead can request a Lean kernel projection. This bead has no such requirement.
