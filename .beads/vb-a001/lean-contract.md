# Theorem Kernel Projection

## Boundary

- **TLA+-owned temporal model:** ForEachParity.tla — compiled-parity (INV-005) and termination (INV-004).
- **Verus-owned Rust core:** lower_canonical_for_each edge emission (INV-001, INV-002), drive_deterministic_full termination (INV-004 state machine).
- **Theorem-owned kernel:** none — all Rust-local proof obligations are expressible in Verus.
- **Rust/runtime shell:** CLI dispatch, postcard I/O, journal persistence, slot store — tested by Fowler tests and manual QA, not proven.
- **External systems excluded from theorem proof:** filesystem, database, wall-clock time, FFI.

## Theorem-Owned Clauses

- **None.** The rust-contract skill has determined that:
  - INV-001 (node count/ordering) and INV-002 (edge invariant) are pure Rust-local properties
    of `lower_canonical_for_each` that Verus can express via spec functions on SlotCompiler state.
  - INV-003 (reachability) is a graph traversal property expressible as a Verus postcondition
    on `validate_reachability`.
  - INV-004 (termination) is a state-machine termination property expressible as a Verus loop
    invariant on `drive_deterministic_full` with a decreasing measure.
  - INV-005 (parity) is a temporal property owned by TLA+, not by a theorem kernel.

## Waivers

- No theorem kernel is projected because all Rust-local proof obligations have a direct Verus
  encoding. A Lean/Aeneas/Hax kernel would only add overhead without additional proof power.
  If future beads touch algebraic properties (e.g., slot index arithmetic overflow at u16::MAX),
  a theorem kernel may be warranted then.
