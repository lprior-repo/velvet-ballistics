# Theorem Kernel Projection: vb-m5gp

## Boundary

- TLA+-owned temporal model: none; see `tla-spec.md`.
- Verus-owned Rust core: no new algorithmic contract is introduced by this bead; Verus is waived for the pure structural split.
- Theorem-owned kernel: none.
- Rust/runtime shell: `vb_compile` compile facade, validation, lowering, diagnostics, artifact emission, and idempotency helpers as existing Rust code moved behind private modules.
- External systems excluded from theorem proof: filesystem/build tooling, cargo/moon execution, generated Rust compiler behavior outside existing tests.

## Theorem-Owned Clauses

- None. No algebraic kernel, parser grammar theorem, arithmetic theorem, protocol lattice, or extracted model is introduced by this refactor.

## Theorem Obligations

- None.

## Waiver

- THM-WAIVER-001: Lean/Aeneas/Hax waived because behavior-preserving file extraction has no tiny theorem-critical kernel. Owner: State 3 contract. Expiry: this bead only. Compensating evidence: compile/test parity, API compatibility, Kani idempotency parity if available, Miri if local budget allows, static source governance checks, and contract verification review.

## Escalation Rule

If implementation changes semantics of validation, lowering, digesting, artifact emission, or idempotency rather than moving code, this waiver expires and State 3 must be rerun with Verus/Kani/theorem obligations scoped to the changed pure logic.
