# LifecycleJournal TLA Report

bead_id: vb-qi37.16.5
updated_at: 2026-05-11T20:30:07Z
status: PASS_TLA_ONLY

## Model Boundary

- Module: `specs/LifecycleJournal.tla`
- Config: `specs/LifecycleJournal.cfg`
- Variables: `bead_state`, `journal`, `commands`, `crashed`
- Actions: `Init`, `Start`, `NeedAnswer`, `Fail`, `SubmitTerminal`, `SubmitInvalid`, `SubmitDuplicate`, `Process`, `Crash`, `Replay`
- Refinement: `journal` refines the append-only runtime journal vector; `bead_state` refines lifecycle typestate; `Replay` refines journal replay reconstruction.

## Bounds / Trusted Reductions

- `Beads = {b1, b2}`
- `MaxJournalLen = 10`
- `MaxAnswer = 1`
- `commands` constrained to at most one in-flight command, matching serial CLI/runtime lifecycle dispatch.
- Invalid and duplicate submissions are representative bounded probes, while integration evidence covers exhaustive CLI invalid-transition cases.
- No symmetry set used. Deadlock checking disabled per existing config (`CHECK_DEADLOCK FALSE`) because terminal states intentionally stutter.

## Command Evidence

```bash
tlc -config specs/LifecycleJournal.cfg specs/LifecycleJournal.tla
```

Outcome: PASS.

```text
Model checking completed. No error has been found.
35647 states generated, 15463 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 16.
The average outdegree of the complete state graph is 1 (minimum is 0, the maximum 9 and the 95th percentile is 7).
Finished in 03s at (2026-05-11 20:30:07)
```

Highest observed completed run state count: 35,647 generated / 15,463 distinct.

## Checked

Invariants:
- `NoOverwrite`
- `SingleCanonicalState`
- `InvalidTransitionBlocked`
- `ReplayBitIdentical`
- `TypeInvariant`

Temporal properties:
- `EventuallyTerminalOrCancelled`
- `JournalGrowth`

## Residual Formal Blocker

TLA-LIFECYCLE-001..006 are PASS under the bounded model above. State 12 is still not globally approvable because required Verus obligations remain `FAIL_LOCAL`: `verus` is not installed in PATH, and no approved Verus waiver exists in this bead.
