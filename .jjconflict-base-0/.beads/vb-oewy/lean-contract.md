---
bead_id: vb-oewy
bead_title: "bdd: Full suite runner and evidence artifact contract"
phase: 3
updated_at: 2026-05-20T05:10:00Z
attempt: 1
---

# Theorem Kernel Projection — vb-oewy

## Boundary

- **TLA+-owned temporal model**: None — not a temporal system
- **Verus-owned Rust core**: `BddScenarioResult` construction, `BddSuiteResult` aggregation, evidence bundle serialization
- **Theorem-owned kernel**: None — this bead is a deterministic test runner with no algebraic theorems to project
- **Rust/runtime shell**: File I/O (evidence write), subprocess execution (cargo test), YAML serialization
- **External systems excluded**: None

## Theorem-Owned Clauses

None. This bead does not involve:
- Algebraic state machines requiring theorem-prover extraction
- Protocol lattices requiring proof-assistant formalization
- Parser grammars requiring extracted correctness proofs
- Arithmetic bounds requiring theorem-prover certification

Verus is sufficient for all Rust-local pure obligations.

## Waivers

All clauses are handled by Verus (Rust-local pure functions) and Fowler tests (behavioral verification).
