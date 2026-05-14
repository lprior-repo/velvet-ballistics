bead_id: vb-qi37.16.2
phase: state-12-tla

STATUS: PASS

Command: `tlc -config specs/ResumeStateMachine.cfg specs/ResumeStateMachine.tla`

Bounds/constants:
- `RunIds = {r1, r2}`
- `MaxJournalLength = 4`

Checked invariants:
- `TypeOK`
- `ValidTransition`
- `NoDoubleRunning`
- `FailedNotResumable`
- `JournalImmutable`
- `JournalAppendBeforeSuccess`

TLC result:
- No error found.
- 850 states generated.
- 313 distinct states found.
- Complete graph depth 13.
- Worker count: 1.

Limitations:
- Safety-only bounded model. No separate liveness property was checked in this run.
