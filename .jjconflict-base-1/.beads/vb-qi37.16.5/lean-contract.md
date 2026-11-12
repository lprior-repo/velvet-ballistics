# Theorem Kernel Projection

## Boundary

- **TLA+-owned temporal model**: Lifecycle state machine transitions, journal append-only semantics, replay correctness, invalid/duplicate/stale rejection (see `tla-spec.md`)
- **Verus-owned Rust core**: Typestate invariants in `lifecycle.rs`, journal event validity in `journal.rs`, command validation preconditions in `storage.rs`, storage-backed runtime operations
- **Theorem-owned kernel**: None — no algebraic state theorems, parser grammars, codec refinement claims, or arithmetic bound theorems requiring proof-assistant extraction beyond Verus
- **Rust/runtime shell**: CLI argument dispatch (args.rs), storage I/O (vb_storage), async runtime scheduling — excluded from formal proof
- **External systems excluded**: Storage backend (abstracted as journal interface), operating system I/O

## Theorem-Owned Clauses

None. The critical clauses for this bead decompose as:

| Clause | Layer |
|--------|-------|
| INV-001 (single canonical state) | Verus typestate proof on `lifecycle.rs` |
| PRE-002 (command validation) | Verus preconditions on `storage.rs::validate_command` |
| POST-001 (exactly-one journal event) | Verus postconditions on `journal.rs::append_event` |
| INV-002, INV-003, INV-004, POST-003/4/5 | TLA+ model checking (temporal) |

## Lean Not Required

This bead's critical behavior does not require Lean, Aeneas-to-Lean, or Hax-to-Lean for the following reasons:

1. **No algebraic theorem kernels**: The lifecycle state machine is expressed as a TLA+ state machine, not an algebraic data type requiring equational reasoning.
2. **No parser/codec invariants**: The integration test scope does not include grammar or serialization theorem proving.
3. **No arithmetic bounds theorems**: Bounded model checking (Kani) covers numeric/indexing paths; no需要对自然数不等式进行证明。
4. **Rust typestate is Verus-native**: `lifecycle.rs` typestate `enum` invariants are directly expressible in Verus spec functions.
5. **Journal event formatting is pure Verus**: `RuntimeJournalEvent` construction and validation is local to `journal.rs` and fully Verus-expressible.

## Waivers

N/A — no theorem-owned clauses exist in this bead's scope. If future beads require algebraic refinement proofs (e.g., codec equivalence), they will be added as separate theorem obligations with explicit Lean module declarations.
