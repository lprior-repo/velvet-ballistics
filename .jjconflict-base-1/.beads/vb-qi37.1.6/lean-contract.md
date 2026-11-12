# Theorem Kernel Projection

## Boundary
- TLA+-owned temporal model: crash cuts, ordered replay, latest-attempt filtering, snapshot-tail ordering, waits, asks, actions, collect continuity.
- Verus-owned Rust core: recovery summary/frame-seed invariants, dimension bounds, taint exactness, fail-closed classification, monotonic application of recovered facts.
- Theorem-owned kernel: none at State3.
- Rust/runtime shell: Fjall I/O, file system crash simulation, runtime primitive execution, and collect side-table hydration are verified by integration, proptest, Miri where scoped, and gauntlet gates.
- External systems excluded from theorem proof: storage engine durability internals, wall-clock time, OS process restart, and external action side effects.

## Theorem-Owned Clauses
- None. No clause currently requires Lean/Aeneas/Hax beyond TLA+ and Verus.

## Theorem Obligations
- None planned.

## Waivers
- THM-WAIVE-001: Lean/Aeneas/Hax waived for this bead because the critical properties are temporal recovery workflows or Rust-local data invariants expressible by TLA+ and Verus. Owner: State3 rust-contract. Expiry: before State4 proof writing if Verus cannot express taint/dimension monotonicity. Compensating evidence: required TLA+ obligation, Verus obligations, proptest, integration restart tests, and `moon run :verify-proof`.
