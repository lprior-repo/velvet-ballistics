# STATE.md — vb-ypnk: quality: Add evidence bundle format and writers

## Bead
- **ID**: vb-ypnk
- **Title**: quality: Add evidence bundle format and writers
- **Status**: in_progress
- **Source checkout**: /home/lewis/src/velvet-ballistics
- **Isolated workspace**: /home/lewis/src/velvet-work/go-skill-vb-ypnk
- **Isolation verified**: YES (workspace path not inside source checkout)
- **Claimed**: 2026-05-18

## Current State: State 1 — Claim, isolate, baseline
- Workspace created: go-skill-vb-ypnk
- Baseline captured: moon ci --force shows 7 completed, 5 failed, 11 skipped
- Pre-existing compile errors in: xtask (4), vb_cli test (2), vb_storage test (21)

## State Progression
| State | Status | Notes |
|-------|--------|-------|
| 1 | COMPLETE | Claim + isolate + baseline — workspace at /home/lewis/src/velvet-work/go-skill-vb-ypnk |
| 2 | COMPLETE | Explore — codebase-map.md + delivery-scope.jsonl written |
| 3 | COMPLETE | Contract — contract.md written (12 requirements, 7 invariants, 5 types) |
| 4 | COMPLETE | Proof plan — proof-strategy.md + proof-plan-review-input.jsonl + proof-obligations.planned.jsonl (8 obligations, all required) |
| 5 | COMPLETE | Proof write — Kani harnesses, proptest properties, Miri tests, reports written. All compile. |
| 6 | PENDING | Proof review |
| 7 | PENDING | Test plan |
| 8 | PENDING | Test write |
| 9 | PENDING | Test review |
| 10 | PENDING | Implementation |
| 11 | PENDING | Execute gates |
| 12 | PENDING | Black-hat review |
| 13 | PENDING | Evidence + truth-serum |
| 14 | PENDING | Landing |
| 15 | PENDING | Cleanup |

## Retry Budget
- Total attempts remaining: 7
- Last failure: N/A (State 1 in progress)
- Next repair target: N/A

## Risks
- Pre-existing compile errors in xtask, vb_cli, vb_storage may affect evidence capture scope
- Bead depends on vb-6f02 (check if resolved)
