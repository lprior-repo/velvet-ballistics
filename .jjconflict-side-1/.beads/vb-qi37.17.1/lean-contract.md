# Theorem Kernel Projection

## Boundary

- **TLA+-owned temporal model**: None (no temporal behavior).
- **Verus-owned Rust core**: `build_incident_report` and `build_repair_hints` are pure functions. Verus could express postconditions on these, but the bead scope is limited to compile fixes + unwrap fixes + tests, not proof introduction.
- **Theorem-owned kernel**: N/A. No algebraic state transitions or protocol lattices beyond what Verus could express.
- **Rust/runtime shell**: `cmd_incident` — I/O, JSON serialization, FjallJournal access.
- **External systems excluded from theorem proof**: FjallJournal, serde_json, std::fmt.

## Theorem-Owned Clauses

- **None**. Verus owns the Rust-local proof obligations for pure functions; no tiny theorem kernel projection is needed.

## Verus-Owned Clauses (future, out of scope for this bead)

- `build_incident_report` postconditions: INV-003, INV-004
- `build_repair_hints` postconditions: INV-005

**Verus scope deferred** to a follow-up bead. This bead's scope is compile fix + unwrap fix + tests.

---

**Written by**: rust-contract agent
**Bead**: vb-qi37.17.1
**Date**: 2026-05-17
