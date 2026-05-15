# Theorem Kernel Projection: vb-engine-yaml

## Boundary

- TLA+-owned temporal model: accepted-artifact lifecycle, persist-before-ack, ingress/backpressure, run lifecycle, recovery/replay.
- Verus-owned Rust core: numeric IDs, resource bounds, checked access, budgets, taint lattice, step-state transition preservation, recovery validity predicates, capability/artifact model.
- Theorem-owned kernel: none required at State 3.
- Rust/runtime shell: Fjall I/O, Postcard decode shell, CLI rendering, direct API/IPC socket interactions, wall-clock scheduling, filesystem/process execution for gates.
- External systems excluded from theorem proof: Fjall compaction/storage internals, OS scheduling, terminal output, Moon task orchestration.

## Theorem-Owned Clauses

- None.

## Rationale

The critical pure clauses currently fit Verus better than Lean/Aeneas/Hax:

- Resource bound arithmetic and checked access are Rust-local proof obligations.
- Taint and capability lattices are small enough for Verus proof functions.
- Recovery validity predicates can be modeled in Verus over abstract durable records.
- Temporal lifecycle correctness belongs in TLA+ rather than a theorem assistant.

Lean/Aeneas/Hax may be introduced later only if a tiny extracted kernel is needed for digest-envelope refinement, artifact schema equivalence, or a lattice theorem that Verus cannot express without unacceptable trusted boundaries.

## Waivers

- LEAN-WAIVER-001: No Lean/Aeneas/Hax theorem kernel is required for State 3. Owner: proof-planner. Reason: Verus and TLA+ cover the scoped critical properties. Expiry: before proof-writer finalizes formal artifacts. Compensating evidence: Verus obligations for Rust-local pure invariants plus TLA+ obligations for lifecycle behavior.
