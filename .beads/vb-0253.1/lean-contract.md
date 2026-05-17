# Lean Theorem Kernel Projection - vb-0253.1

## Boundary
- **TLA+-owned temporal model**: None (see tla-spec.md)
- **Verus-owned Rust core**: Queue capacity invariants, length correctness
- **Theorem-owned kernel**: None - Verus is sufficient for Rust-local invariants
- **Rust/runtime shell**: Queue enqueue/dequeue operations
- **External systems excluded**: None

## Theorem-Owned Clauses
- None - Verus can handle all Rust-local invariants for this bead

## Waivers
- Lean/Aeneas/Hax waived. This bead is about bounded queue invariants which are straightforward to verify with Verus. No algebraic state transitions, no protocol lattices, no refinement proofs beyond Verus capability.
