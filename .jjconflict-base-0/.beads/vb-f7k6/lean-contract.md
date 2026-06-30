# Theorem Kernel Projection

## Boundary
- TLA+ owns temporal scheduling, overflow/suspend, due fire, stale rejection, terminal immutability.
- Verus owns Rust-local checked arithmetic and pure index/freshness transition refinements where expressible.
- Lean/Aeneas/Hax has no mandatory State 3 obligation.
- Shell excluded: async runtime, mailbox, persistence, wall-clock `Instant`, allocation.

## Theorem-Owned Clauses
None mandatory.

Optional THM-001 if Verus is insufficient:
- Clauses: INV-003, INV-004, INV-005, INV-008.
- Lean module: `TimerWheel.IndexModel` if introduced later.
- Theorem: `transition_preserves_projection_equivalence`.
- Model: finite active timer set plus two map projections.
- Refinement: Rust timer wheel validates to abstract model before transition and reifies after transition.
- Evidence command: blocked until State 4 decides Lean is necessary.

## Waiver
Mandatory Lean proof waived for State 3 because TLA+ plus Verus/Kani/proptest/Loom is the correct first-line evidence. Owner: State 4 proof planner. Expiry: before implementation if Verus cannot express the index projection or arithmetic obligations without vacuum proof.
