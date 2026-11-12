# Machine Gate Report — vb-rpch

## TLC Execution Summary

| Parameter | Value |
|-----------|-------|
| Spec | `RecoveryReplayFull.tla` |
| States Explored | 443,000+ |
| Depth | 5 |
| Invariant Violations | 0 |

## Invariant Results

| Invariant | Status |
|-----------|--------|
| `TypeOK` | **PASS** |
| `TailCausalAfterSnapshot` | **PASS** |
| `ReplaySeqOrder` | **PASS** |
| `OnlyIncompleteRuns` | **PASS** |
| `NoResolvedReExecution` | **PASS** |
| `DigestVerificationOrder` | **PASS** |

## Structural Note

The spec models a pending/completed/failed tracker state machine. All transitions and state predicates verified clean through depth 5 exhaustive exploration.

## STATUS: PASS

All machine-gate criteria satisfied. TLC exhaustively explored 443k+ states at depth 5 with zero invariant violations across all 6 invariants.
