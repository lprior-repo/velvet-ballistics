# Lean Theorem Prover Notes for vb-7m54

## Theorem Prover Scope

Lean/Aeneas/Hax is NOT required for VB-CONC-001..005 because:
1. VB-CONC-001..005 are **concurrency ordering** properties, not pure mathematical theorems
2. The Rust implementation uses lock-free data structures (ArrayQueue, rtrb RingBuffer) where formal verification requires model checking (Loom), not theorem proving
3. The root formal verification already has Verus proofs for frame transitions and budget arithmetic - the appropriate tool for Rust-local pure logic

## Verus Scope for Concurrency

Verus is the appropriate Rust-native proof tool for any pure/concurrent Rust invariants that don't require exhaustive model checking. However:
- Loom is specified as the required tool in proof_obligations.yaml
- The concurrency seams involve runtime behavior (not pure logic) that Loom is designed to verify

## What IS in Scope for Lean/Aeneas

The theorem kernel scope remains with Verus for Rust-local pure properties:
- Frame transition relation (already proven in frame_verus.rs)
- Budget arithmetic (already proven in budget_verus.rs)
- Taint lattice laws (already proven in taint_lattice.rs)

No new Lean/Aeneas/Hax obligations are created by VB-CONC-001..005.
