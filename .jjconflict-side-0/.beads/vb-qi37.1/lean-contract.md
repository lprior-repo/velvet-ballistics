# Theorem Kernel Projection: vb-qi37.1

## Boundary
- TLA+-owned temporal model: journal/snapshot/restart/recovery state machine and fail-closed liveness.
- Verus-owned Rust core: unsupported-state reject lattice, no silent empty-frame success, digest-mode obligations, and pure transition invariants.
- Theorem-owned kernel: none required at State 3.
- Rust/runtime shell: Fjall I/O, snapshot byte decoding, runtime frame mutation, CLI/operator paths.
- External systems excluded from theorem proof: filesystem, Fjall compaction, wall-clock time, process crash mechanics.

## Theorem-owned clauses
- None.

## Theorem obligations
- None. The State 3 contract assigns proof obligations to TLA+ and Verus. Lean/Aeneas/Hax would be reconsidered only if Verus cannot express the unsupported-state lattice or digest-mode algebra.

## Waivers
- THM-WAIVE-001: No Lean/Aeneas/Hax kernel is required for this bead because the critical pure properties are boolean/state-transition invariants expressible in Verus, and temporal recovery behavior is better represented in TLA+.
  - Owner: State 3 rust-contract.
  - Expiry: before State 4 contract-verification review completion.
  - Compensating evidence: Verus obligations `VERUS-UNSUPPORTED-001`, `VERUS-NOEMPTY-001`, `VERUS-DIGEST-001` plus TLA+ obligations `TLA-RECOVERY-001..003`.
