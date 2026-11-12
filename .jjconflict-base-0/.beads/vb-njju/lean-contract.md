# Theorem Kernel Projection

## Boundary

- TLA+-owned temporal model: none; see `tla-spec.md` waiver.
- Verus-owned Rust core: none mandatory at State 3; no new production pure core is specified.
- Theorem-owned kernel: none.
- Rust/runtime shell: workspace acceptance tests, Moon gates, cargo-fuzz, cargo-mutants, proptest, boundary inventory.
- External systems excluded from theorem proof: filesystem reads, Moon, cargo-fuzz, cargo-mutants, fuzz corpus/seed execution, mutation reports.

## Theorem-owned clauses

- None.

## Rationale

The bead's critical claims are fail-closed evidence classification and executable BDD coverage. They do not require a tiny algebraic theorem beyond Verus; they require strong executable tests, mutation, fuzz, property, and release gate evidence.

## Waivers

- LEAN-WAIVE-001: Lean/Aeneas/Hax not applicable. Owner: State 3 contract. Reason: no theorem-critical algebraic kernel or extracted production function in scope. Expiry: State 4 review; if a non-trivial evidence lattice or generated-vs-IR equivalence kernel is introduced, revisit with Verus first and Lean only if Verus cannot express the theorem.
