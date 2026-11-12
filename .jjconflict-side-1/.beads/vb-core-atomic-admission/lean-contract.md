# Theorem Kernel Projection

## Boundary

- TLA+-owned temporal model: accepted-run staging, commit, failure, acknowledgement, restart/readback.
- Verus-owned Rust core: pure accepted-run model, sequence equality, index derivation consistency, artifact envelope discriminator, error classification.
- Theorem-owned kernel: none at State 3.
- Rust/runtime shell: Fjall I/O, filesystem persistence, CLI output, runtime allocation, restart process, and wall-clock time.
- External systems excluded from theorem proof: Fjall internals, OS filesystem, command-line shell, async/runtime scheduling.

## Theorem-Owned Clauses

- None. No tiny algebraic theorem beyond Verus is justified by State 2 evidence.

## Theorem Obligations

- None planned. If later proof review finds Verus insufficient for the artifact/index/sequence algebra, create a separate Lean/Aeneas/Hax obligation with an explicit theorem module and refinement relation.

## Waivers

- THM-WAIVE-001: Lean/Aeneas/Hax waived for State 3 because contract clauses split cleanly into TLA+ temporal atomicity and Verus-owned Rust-local pure invariants. Owner: State 3 rust-contract. Expiry: before State 4 proof-writing review. Compensating evidence: Verus obligations VERUS-PRE-001, VERUS-INV-003, VERUS-INV-004, VERUS-INV-005, plus TLA-ATOM-001.
