# Test Repair Guide: vb-scxh State 8

STATUS: APPROVED

No State 8 repair required after the repaired harness review.

## Routing

- owner_state: State 11
- rerun_from: State 11 raw evidence/audit execution

## Preserved Downstream Requirements

- State 11 must capture exact raw BD output and exact 12 false-closure IDs.
- State 11 must resolve or preserve the safety-anchor `BLOCK_LOCAL` result.
- State 11 must capture Moon CI artifact-path evidence and fresh-rerun marker before accepting CI evidence.
- State 11 must capture exact `35/35 unviable` mutation markers while keeping mutation non-adequacy classified as `FAIL_UNVIABLE` / `DEFERRED`.
- State 12 must still reject subagent-only claims and block close/unblock while any required lane is missing, blocked, stale, or deferred.
