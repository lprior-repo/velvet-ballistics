# Theorem Kernel Projection

## Boundary
- **TLA+-owned temporal model**: Runtime lifecycle state machine, resume state transition ordering, journal immutability, fail-closed error behavior (covered in tla-spec.md)
- **Verus-owned Rust core**: Pure state transition predicates, journal data structure invariants, typestate field presence, append ordering within a single journal, ShardCommand::Resume handling
- **Theorem-owned kernel**: None. No tiny algebraic or extracted model requires Lean/Aeneas/Hax beyond Verus
- **Rust/runtime shell**: CLI argument parsing, structured output formatting, storage backend I/O, async scheduling, wall-clock time
- **External systems excluded from theorem proof**: FJALL/LSM-tree storage backend, CLI terminal I/O

---

## Theorem-Owned Clauses
None. All Rust-local pure critical behavior is expressible in Verus and does not require Lean/Aeneas/Hax extraction.

---

## Theorem Obligations
None. No theorem kernel projection is required for this bead.

---

## Verus-Owned Rust-Local Proof Obligations

The following contract clauses are proven by Verus within the Rust codebase:

| Clause ID | Description | Verus Target |
|-----------|-------------|--------------|
| INV-001 | Runtime state machine transition validity | `vb_runtime::shard::lifecycle::Shard::handle_resume` |
| INV-002 | Journal append-only invariant | `vb_runtime::journal::RuntimeJournal::append` |
| INV-003 | ResumeResult field presence | `vb_runtime::shard::types::ResumeResult` |
| PRE-002 | Resumability predicate | `vb_runtime::shard::types::RuntimeState::is_resumable` |
| PRE-003 | Hydration completeness check | `vb_runtime::journal::RuntimeJournal::is_hydration_complete` |
| POST-001 | Journal append-before-success ordering | `vb_runtime::journal::RuntimeJournal::append` + lifecycle handler |
| POST-004 | Durability evidence append | `vb_runtime::journal::RuntimeJournal::append` |

---

## Waivers
None. Verus is sufficient for all Rust-local pure/core critical behavior in this bead's scope.

**Rationale for no Lean/Aeneas/Hax**: The resume transition logic is a straightforward state machine with no exotic algebraic structures (lattices, monads, parsers, or arithmetic bounds) that would benefit from proof-assistant extraction. Verus can express all pure state predicates, data structure invariants, and temporal ordering constraints within its spec/proof language.
