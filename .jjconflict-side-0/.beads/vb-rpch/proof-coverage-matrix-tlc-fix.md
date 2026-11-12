# Proof Coverage Matrix — vb-rpch TLC Fix Pass

| Requirement | Current artifact | Current status for this pass | Planned obligation |
|---|---|---|---|
| TLA+ well-formed model/config | `specs/tla/RecoveryReplayFull.tla`, `.cfg` | **STALE/BLOCKED**: source cfg has extra `Digest`; evidence cfg differs; current approval unsupported | `TLC-FIX-001`, `TLC-FIX-004`, `TLC-FIX-008` |
| TLA-001 ReplaySeqOrder | `ReplaySeqOrder` | **STALE**: invariant exists, but non-vacuity and identity `Sort` concern unresolved | `TLC-FIX-003`, `TLC-FIX-005`, `TLC-FIX-006`, `TLC-FIX-007` |
| TLA-002 TailCausalAfterSnapshot | `TailCausalAfterSnapshot`, `SetSnapshot` | **DEFECT**: `SetSnapshot(0,0)` outside domains; missing `snapshot_seq'` assignment/UNCHANGED | `TLC-FIX-002`, `TLC-FIX-005`, `TLC-FIX-006`, `TLC-FIX-007` |
| TLA-003 OnlyIncompleteRuns | `OnlyIncompleteRuns` | **STALE**: invariant exists, recovered-runs reachability not proven | `TLC-FIX-005`, `TLC-FIX-006`, `TLC-FIX-007` |
| TLA-004 NoResolvedReExecution | `NoResolvedReExecution` | **STALE**: invariant exists, guard reachability not proven; prior review has contradictory note “known pre-existing violation” but still PASS | `TLC-FIX-005`, `TLC-FIX-006`, `TLC-FIX-007` |
| TLA-005 RecoveryErrorExhaustive | `last_error \in {...}` only | **NOT PROVEN**: membership is TypeOK, not reachability of every error | `TLC-FIX-007` |
| TLA-006 DigestVerificationOrder | `DigestVerificationOrder` | **STALE**: invariant exists, digest-stage reachability/order semantics need non-vacuity | `TLC-FIX-005`, `TLC-FIX-006`, `TLC-FIX-007` |
| Raw evidence/report hygiene | `evidence/specs/*`, `.beads/vb-rpch/*`, root `proof-review.md` | **STALE/CONTRADICTORY**: APPROVED/PASS claims not backed by fresh final raw output against current files | `TLC-FIX-008` |
