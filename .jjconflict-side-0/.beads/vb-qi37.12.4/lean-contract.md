# Theorem Kernel Projection: vb-qi37.12.4

## Boundary

- TLA+-owned temporal model: none.
- Verus-owned Rust core: waived for current State 3 because no Rust-local classifier/exception-validator artifact exists; mandatory follow-up if such Rust logic is introduced.
- Theorem-owned kernel: none identified.
- Rust/runtime shell: shell script, filesystem traversal, Moon task graph, clippy invocation, report generation.
- External systems excluded from theorem proof: Moon, Cargo/clippy, bash, filesystem, operating-system exit status.

## Theorem-Owned Clauses

- None.

## Theorem Obligations

- None. The contract contains no algebraic kernel, protocol lattice, parser grammar theorem, numeric theorem, or refinement claim that requires Lean/Aeneas/Hax beyond simpler executable evidence.

## Waivers

- LEAN-WAIVER-001: Lean/Aeneas/Hax waived for all clauses. Waived layer: `lean/aeneas/hax`. Owner: State 3 rust-contract. Reason: no theorem-critical algebraic kernel, parser grammar theorem, protocol lattice, or numeric theorem exists in current scope. Limitation: theorem assistants would only re-state shell/static-gate behavior without a proof-friendly core model. Expiry/follow-up: if downstream implementation creates a reusable Rust classifier/exception-validation lattice whose totality, mutual exclusion, or refinement cannot be expressed by Verus/Kani/proptest. Compensating evidence: executable obligations `GATE-CLASSIFIER-001`, `GATE-EXC-VALIDATION-001`, `GATE-DETERMINISM-001`, and `GATE-FAIL-CLOSED-001`.
