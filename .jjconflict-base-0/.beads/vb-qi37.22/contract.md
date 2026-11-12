bead_id: vb-qi37.22
phase: 3
attempt: 1-of-7

# Contract

## Requirements

- R1: xtask exposes required command families: ai-context, ai-plan, ai-check, ai-evidence, invariants, scans, cert-check, perf, replay, crash, diff, mutants, loom, kani, fuzz, prop, repro, test-plan, review, why-failed.
- R2: `contracts/` contains machine-readable contracts-as-data schemas.
- R3: evidence bundle implementation exists and is consumed by quality gates.
- R4: this aggregate dependency bead closes only after its implementation dependency beads are closed.

## Non-change invariant

This resolution introduces no production source change. It only records evidence that the aggregate scope is already satisfied by closed dependency beads and current files.
