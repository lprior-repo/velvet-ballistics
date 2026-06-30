# Theorem Kernel Projection: vb-f04l

## Boundary

- TLA+-owned temporal model: loop/fanout/retry/wait/ask lifecycle progress and deadlock freedom.
- Verus-owned Rust core: dense indexes, bounded numeric conversions, slot coverage, deterministic lowering plan, and graph shape refinements.
- Theorem-owned kernel: none mandatory at State3.
- Rust/runtime shell: YAML parsing, builder allocation details, validation API calls, runtime event delivery, wall-clock time, storage, and generated Rust.
- External systems excluded from theorem proof: scheduler, action execution, event source, human input, filesystem, CLI, and storage.

## Theorem-Owned Clauses

- None required now. Verus owns INV-001, INV-002, INV-003, INV-004, INV-005, PRE-007, and POST-006 through POST-012 at contract time.

## Optional Theorem Escalation Criteria

- THM-DENSE-001 may be created only if a later proof-planner or proof-writer demonstrates that Verus cannot express recursive preorder body expansion without excessive trusted assumptions.
- Required future target, if escalated: an approved tiny theorem kernel for recursive body expansion preserving dense node IDs and target membership.
- No executable Lean command is contracted now because no Lean project or module exists in this workspace and theorem proof is waived for this bead-local State3 contract.

## Waivers

- LEAN-WAIVER-001: No theorem kernel is mandatory for State3 because dense-index, target-range, slot-coverage, and primitive-shape properties are Rust-local and are assigned to Verus plus Kani/proptest/test evidence. Owner: State3 contract. Expiry: before implementation approval if strengthened Verus remains unable to express recursive body expansion. Compensating evidence: `verus verification/verus/v1_primitive_lowering.rs`, concrete Kani/proptest/property tests, and `moon ci`.
