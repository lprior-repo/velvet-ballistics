# Theorem Kernel Projection — vb-rpch

## Boundary

- **TLA+-owned temporal model**: Journal replay sequence, snapshot-plus-tail ordering, digest verification pipeline, incomplete-run discovery, terminal event detection — owned by `specs/RecoveryReplay.tla`
- **Verus-owned Rust core**: All pure Rust-local critical clauses — `UnsupportedRecoveryState::union`, `ActionReplayTracker` monotonicity, `DigestCheck` hierarchy, dimension bounds, seed construction invariants, replay attempt filtering
- **Theorem-owned kernel**: None required for this bead
- **Rust/runtime shell**: I/O, Fjall journal access, wall-clock time, async scheduling — excluded from formal proof
- **External systems excluded**: Fjall internal storage, OS filesystem sync, network

---

## Theorem-Owned Clauses

**None.** The algebraic properties of `UnsupportedRecoveryState::union` (commutativity, associativity, idempotency, no contradictory state) are correctly proven as Verus `proof fn union_correctness` within the Rust-local proof surface. No Lean/Aeneas/Hax extraction is warranted at this time.

The `ActionReplayTracker::is_resolved` monotonicity property is also fully captured by a Verus `proof fn tracker_monotonic`. No secondary theorem-prover kernel is needed.

---

## Lean/Aeneas/Hax Non-Applicability Rationale

The recovery domain is a deterministic state machine with bounded integer counters (step indices, slot indices, sequence numbers, attempt numbers). All critical invariants are:

1. Expressible as Verus `spec fn` / `proof fn` over the Rust types
2. Bounded (u16 dimensions, finite event sequences)
3. Tightly coupled to Rust runtime frame construction

The gap between Rust-level invariants and any abstract algebraic model (e.g., a lattice of `UnsupportedRecoveryState` values) is too small to justify Lean/Aeneas/Hax extraction. If future beads require extracted protocol refinement proofs or algebraic state machine refinements, a lean-contract.md will be authored at that time with proper module/theorem/refinement boundaries.

---

## Waivers

- No theorem-owned clauses exist in this bead. All pure Rust-local properties are covered by Verus obligations recorded in `proof-obligations.jsonl`.
- GAP-3 items (ActionAbiMismatch, PolicyDigestMismatch) are excluded from proof obligations pending vb-ty9.
