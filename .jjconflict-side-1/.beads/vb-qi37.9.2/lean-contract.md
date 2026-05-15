# Theorem Kernel Projection — vb-qi37.9.2

## Boundary
- **TLA+-owned temporal model**: None — no temporal behavior in scope
- **Verus-owned Rust core**: F64 arithmetic ops in `crates/vb_expr/src/eval.rs` — `eval_add_op`, `eval_sub_op`, `eval_mul_op`, `eval_div_op`, `eval_neg_op`, and comparison ops. These are pure deterministic functions.
- **Theorem-owned kernel**: None — no algebraic kernels beyond Verus expressibility; no parser grammars, protocol lattices, or arithmetic theorems requiring Lean/Aeneas/Hax extraction.
- **Rust/runtime shell**: `eval_expr_program`, `eval_expr_program_with_store` orchestrate stack, ops, and ValueStore; not pure, covered by integration tests.
- **External systems excluded from theorem proof**: None — no I/O, network, DB, FFI in F64 eval path.

## Theorem-Owned Clauses
- **None** — No theorem kernel extraction needed. Verus is sufficient for all pure Rust-core obligations (postconditions on eval ops, finiteness invariants, IEEE 754 correspondence properties).

## Theorem Obligations
- N/A — no Lean/Aeneas/Hax obligations for this bead.

## Lean/Aeneas/Hax Scope
- **Theorem module**: N/A
- **Rust target**: N/A
- **Abstraction relation**: N/A
- **Shell exclusions**: N/A
- **Non-goals**: Algebraic theorems about F64 arithmetic ( commutativity, associativity, distributivity) are not verification targets for this bead. IEEE 754 correspondence is verified by proptest cross-validation and Kani bounded model check.

## Waivers
- **Verus owns all Rust-core obligations**: Reason = Verus can express all pure F64 arithmetic postconditions (finiteness, IEEE 754 correctness for finite inputs). Lean/Aeneas/Hax extraction not needed. Owner = State 4 (proof-planner) to assign exact Verus spec targets and commands.
- **Theorem kernel waiver**: Reason = no algebraic state transitions, protocol lattices, parser grammar theorems, or arithmetic bounds requiring proof-assistant extraction beyond Verus.
