# Theorem Kernel Projection: vb-core-ipc-sync-evidence

## Boundary
- TLA+-owned model: bounded safety/enabledness for queue/admission/race/timer/shutdown/slow-client/fanout behavior.
- Verus-owned Rust core: pure predicates for strict admission, capacity bounds, terminal winner, timer eligibility, and shutdown monotonicity.
- Theorem-owned kernel: none currently required.
- Rust/runtime shell excluded: sockets, channels, crossbeam internals, storage/journal I/O, OS buffers, scheduler timing, and wall-clock time.

## Theorem-Owned Clauses
- None at repaired State 3.

## Rationale
The current obligations are either temporal/protocol properties better modeled in TLA+ or Rust-local pure predicates already assigned to Verus. No tiny algebraic kernel has been identified that requires Lean/Aeneas/Hax beyond Verus.

## Waivers
- THM-WAIVE-001: Lean/Aeneas/Hax not required for current State 3 scope. Owner: State 3 rust-contract. Reason: TLA+ plus Verus are the selected formal layers; theorem-kernel extraction would be speculative. Expiry/follow-up trigger: before proof-reviewer approval if proof-planner identifies a theorem-only lattice. Compensating evidence: TLA+ obligations, Verus obligations, and explicit refinement blockers in `proof-obligations.jsonl`.
