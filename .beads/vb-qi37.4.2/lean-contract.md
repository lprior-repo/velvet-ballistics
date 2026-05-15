# Theorem Kernel Projection

## Boundary

- TLA+-owned temporal model: admission lifecycle, denial atomicity, gate mismatch denial, exact profile denial, legacy bypass denial.
- Verus-owned Rust core: accepted-envelope predicates, exact capability/cardinality predicates, digest equality predicates, and non-panicking validation structure.
- Theorem-owned kernel: none at State 3.
- Rust/runtime shell: Fjall I/O, postcard byte decoding, CLI/IPC, runtime constructors, journal append, wall-clock staleness.
- External systems excluded from theorem proof: storage backend, filesystem, process boundaries, Moon task scheduler.

## Theorem-Owned Clauses

- None. No tiny algebraic kernel currently exceeds Verus expressiveness.

## Verus Supersedes Lean Here

- PRE-005 and INV-006 map to existing `verification/verus/capability_artifact_model.rs` proof functions.
- PRE-002, PRE-003, PRE-004, INV-001, INV-003 require a Verus accepted-envelope predicate model or explicit proof-planner waiver before proof writing.

## Waivers

- Lean/Aeneas/Hax waiver for all clauses. Owner: rust-contract State 3. Reason: the scoped properties are finite admission lifecycle safety and Rust-local predicate checks expressible in TLA+ and Verus. Expiry: reopen only if proof-review rejects Verus expressiveness for digest/gate/envelope predicates. Compensating evidence: TLA+ safety model, Verus predicate proofs, Kani/proptest/fuzz/integration layers.
