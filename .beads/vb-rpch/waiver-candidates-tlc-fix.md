# Waiver Candidates — vb-rpch TLC Fix Pass

No waiver is approved by proof-planner.

Candidate for proof-plan-reviewer consideration only:

| Candidate | Type | Allowed? | Conditions |
|---|---|---:|---|
| Primary `RecoveryReplayFull.cfg` does not exhaust due to state explosion | proof-resource / non-behavior | maybe | Writer/formal-verifier must first repair model defects, complete smoke cfg, complete non-vacuity checks, run primary cfg with raw final/interrupt output, and label result `PARTIAL_BFS`, not PASS/exhaustive. |

Rejected waiver categories:

- Waiving `SetSnapshot(0,0)` domain defect — **not allowed**.
- Waiving missing `snapshot_seq'` assignment/UNCHANGED — **not allowed**.
- Waiving source/evidence cfg divergence — **not allowed**.
- Waiving non-vacuity for all invariants — **not allowed**.
