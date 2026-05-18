# Theorem Kernel Projection

## Boundary
- TLA+-owned temporal model: verified budget and reservation must precede run admission.
- Verus-owned Rust core: checked arithmetic, sum/max/multiply composition rules, boundedness-policy refinements, and monotonic aggregate usage.
- Theorem-owned kernel: none required for this bead.
- Rust/runtime shell: shard queues, storage, CLI, and diagnostics rendering are excluded from theorem proof.
- External systems excluded: YAML parser, Fjall, IPC, and Moon orchestration.

## Theorem-Owned Clauses
- None.

## Rationale
The bead-local claims are expressible as Rust-local arithmetic/refinement obligations plus one temporal admission ordering model. Verus is the correct proof surface for bounded integer arithmetic and data-structure invariants. TLA+ is the correct proof surface for admission-before-run ordering. Lean/Aeneas/Hax would add overhead without a smaller theorem kernel beyond Verus.

## Waivers
- LEAN-WAIVER-001: No Lean/Aeneas/Hax obligation for vb-qi37.2.4. Owner state: 3. Reason: no algebraic kernel beyond Verus-owned checked arithmetic and TLA+-owned admission ordering. Expiry: when a later reviewer identifies a proof obligation not expressible in Verus/TLA+. Compensating evidence: required Verus, Kani/proptest, TLA+, and gauntlet proof obligations in `proof-obligations.jsonl`.

## Status / Evidence Summary
- Status: planned waiver; independent reviewer must approve or reject the waiver.
