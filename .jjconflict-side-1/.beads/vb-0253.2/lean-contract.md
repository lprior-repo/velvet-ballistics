# Lean Theorem Kernel Projection - vb-0253.2

## Boundary
- **TLA+-owned temporal model**: Queue protocol via TLA+
- **Verus-owned Rust core**: Capacity invariants, FIFO ordering
- **Theorem-owned kernel**: None - Verus sufficient
- **Rust/runtime shell**: MemoryIngress implementation
- **External systems excluded**: None

## Theorem-Owned Clauses
- None - Verus handles Rust-local properties

## Waivers
- Lean/Aeneas/Hax waived. Verus can handle ingress queue invariants and ordering.
