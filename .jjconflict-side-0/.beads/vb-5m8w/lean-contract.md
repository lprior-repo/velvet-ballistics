# Theorem Kernel Projection

## Boundary
- TLA+-owned temporal model: scheduler slices, exhaustion, suspension, resume, evidence ordering, and liveness.
- Verus-owned Rust core: `StepBudget` bounded arithmetic, decrement/no-underflow, no-panic, and run-loop state preservation around zero budget.
- Theorem-owned kernel: optional only; no mandatory Lean/Aeneas/Hax theorem is required for State 3.
- Rust/runtime shell: concrete async/shard scheduling, storage, evidence emission types, and external events.
- External systems excluded: wall-clock time, persistence backend internals, user action delivery, waits, and asks.

## Theorem-Owned Clauses
- None mandatory.

## Optional Theorem Obligation
### THM-OPTIONAL-001
- Contract clauses: INV-001, INV-002, INV-003.
- Candidate theorem: bounded decrement either preserves `0..=MAX_STEP_BUDGET` after subtracting one from a positive value or returns exhausted at zero without changing value.
- Use only if Verus cannot express the arithmetic/refinement cleanly.
- Shell exclusions: I/O, async scheduling, storage, wall-clock time, evidence transport.
- Evidence command: blocked until a real Lean/Hax/Aeneas module exists; do not invent one.

## Waivers
- Lean waiver for this bead: Verus plus TLA+ are the primary proof layers; no tiny theorem kernel is required before proof planning.
