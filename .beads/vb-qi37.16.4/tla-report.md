# TLA+ Report — vb-qi37.16.4

STATUS: PASS

## Command

```bash
tlc -config specs/AskAnswerLifecycle.cfg specs/AskAnswerLifecycle.tla
```

## Bounds

- `MaxRunId = 1`
- `MaxStepIdx = 3`
- `MaxSeqNo = 4`
- `MaxJournalEvents = 24`
- No symmetry reduction.
- No state/action constraint in cfg.
- TLC worker count: 1.

## Checked

Invariants:

- `NoDuplicateAskAnswered`
- `TypeOK`
- `ValidAskState`
- `PendingSubset`
- `MonotonicSeqNo`
- `AnswerPersistenceOrder`

Temporal properties:

- `EventuallyAnswered`
- `EventuallyAdvanced`

Fairness:

- `WF_vars(AnswerAny)`
- `WF_vars(AdvanceAny)`

## Result

```text
Model checking completed. No error has been found.
868 states generated, 361 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 13.
The average outdegree of the complete state graph is 1 (minimum is 0, maximum 3, 95th percentile 3).
Finished in 00s at (2026-05-11 20:29:46)
```

## Repair summary

- `SubmitAsk` now admits only the next monotonic sequence number while `SeqNoCounter[run] < MaxSeqNo`.
- `Terminal` gives TLC an explicit stuttering terminal state when all runs are idle and counters are exhausted.
- Liveness fairness is attached to progress actions (`AnswerAny`, `AdvanceAny`) rather than the whole `Next` disjunction, so replay/terminal stutter cannot starve accepted work.

## Refinement

`AnsweredLog` refines Rust journal ordering: `AnswerAsk` appends `SlotWritten` (`"sw"`) before `AskAnswered` (`"aa"`) for the same `(run, step, seq)`. `SeqNoCounter` refines the per-run monotonic sequence counter. `ReplayAnswer` remains an idempotent no-op for already answered tickets.
