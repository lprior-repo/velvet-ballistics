# Theorem Kernel Projection: vb-qi37.2

## Boundary
- TLA+-owned temporal model: admission/capacity reservation, acknowledgment ordering, fail-closed lifecycle, deterministic step-exhaustion outcome.
- Verus-owned Rust core: budget arithmetic, monotonicity, boundedness, ValueStore cap invariant, StepBudget invariant.
- Theorem-owned kernel: none required at State 3.
- Rust/runtime shell: storage, runtime submission, run-state creation, and execution loop orchestration.
- External systems excluded from theorem proof: YAML compiler, CLI, filesystem, wall-clock, storage backend, UI.

## Theorem-Owned Clauses
- None.

## Theorem Obligations
- No Lean/Aeneas/Hax obligation is planned because the scoped critical clauses are expressible as Verus and TLA+ obligations using existing identified verification surfaces.

## Waivers
- THM-WAIVER-001: Lean/Aeneas/Hax not required for this bead at State 3. Owner: rust-contract. Reason: no tiny theorem kernel beyond Verus was identified in State 2 artifacts; arithmetic monotonicity/bounds are Verus-owned and lifecycle is TLA+-owned. Expiry: before proof-reviewer approval if Verus cannot express a required arithmetic/bounds property. Compensating evidence: Verus obligations plus Kani/proptest/fuzz defense-in-depth.
