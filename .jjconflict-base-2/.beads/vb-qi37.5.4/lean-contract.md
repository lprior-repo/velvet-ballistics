# Theorem Kernel Projection — vb-qi37.5.4

## Boundary

- **TLA+-owned temporal model**: Decision table as pure function specification (reference model, not model-checked)
- **Verus-owned Rust core**: All pure Rust local logic — decision table, key taint checks, invariants, type safety
- **Theorem-owned kernel**: None — no algebraic theorems requiring Lean/Aeneas/Hax extraction
- **Rust/runtime shell**: I/O, scheduling, storage — not applicable to idempotency gate logic
- **External systems excluded**: None — idempotency gate operates purely on in-memory data

---

## Theorem-Owned Clauses

**None**. The idempotency gate logic is entirely within Verus's proof surface:

1. The decision table is a pure function on finite enum domains — exhaustively provable in Verus with `proof_by_cases` or `assert_by_query`.
2. The key taint propagation is a deterministic traversal — provable with loop invariants in Verus.
3. No algebraic state machine, protocol lattice, parser grammar, or arithmetic bound theorems are needed.

---

## Lean/Aeneas/Hax Waiver

Lean/Aeneas/Hax theorem proving is **not required** for this bead.

Rationale:
- The decision table has 5 branches over 3 enum fields (45 total combinations) — exhaustively verifiable by Kani harness or Verus `proof_by_cases`.
- No algebraic structures (monoids, semigroups, Kleene star, etc.) are involved.
- No parser, codec, or grammar invariants — the idempotency gate operates on typed enums.
- No refinement between abstraction layers beyond what Verus specs can express.
- No arithmetic bounds beyond what Verus `assert` statements and `requires`/`ensures` can capture.

If a future bead requires a tiny theorem kernel (e.g., proving the taint lattice join is idempotent), it would be added as a separate obligation with its own proof obligation entry.

---

## Verus Ownership Statement

All Rust-local pure critical behavior for this bead is owned by **Verus**, not by external theorem provers:

- Decision table confluence (INV-003)
- IdempotencyViolation single-variant return (INV-004)
- 3 static rejection variants are mutually exclusive
- Key slot taint propagation invariants
- Proof that `check_idempotency_gates` and `is_statically_idempotent_contract` agree

Verus obligations are listed in `proof-obligations.jsonl` with `layer: verus`.
