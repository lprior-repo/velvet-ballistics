# Theorem Kernel Projection

## Boundary

- TLA+-owned: submit/admission/drive temporal behavior and fail-closed sequencing.
- Verus-owned: exact capability matching, cardinality exactness, validated extraction of required capabilities, and no-prefix grant lattice.
- Theorem-owned kernel: none currently required.
- Rust/runtime shell: storage I/O, postcard decode, Fjall persistence, shard queueing, journal append, and UI serialization.
- External systems excluded: filesystem, Fjall compaction, wall-clock time, async scheduling.

## Theorem-owned clauses

- None for State 3. Verus is the correct first proof surface for the pure Rust-local model.

## Conditional Lean obligation

If Verus cannot express the no-prefix exact-grant lattice with cardinality exactness, introduce only this tiny theorem kernel:

- Contract clauses: INV-001, INV-002.
- Planned module: `verification/lean/CapabilityGrant.lean`.
- Theorem shape: `exact_pair_authorizes_iff_name_and_action_equal` and `cardinality_exact_rejects_extra_or_missing`.
- Model: finite sets/lists of `(CapabilityName, ActionId)` pairs.
- Refinement: Rust `Capability` validates into the abstract pair model; `CapabilitySet::grants` refines pair membership under exact equality.
- Shell exclusions: storage, runtime admission I/O, shard drive, UI, and journal effects.
- Evidence command after proof-writing discovery: `lake build` if a Lean project exists, otherwise the proof planner must record a blocker instead of inventing a command.

## Waivers

- Lean waived for State 3 because no beyond-Verus theorem kernel is presently necessary.
