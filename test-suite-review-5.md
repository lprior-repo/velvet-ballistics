# Master Test Suite Review — Round 5 of 40

**Date:** 2026-06-21
**Reviewer:** test-reviewer
**Mode:** Round 5 — convergence continues; verify round-1+2+3+4 fixes; find new defects.

## STATUS: REJECTED (wave-11 regression)

Rounds 1-4 found and fixed 60+ CRITICAL defects. Round 5 finds:

- **7 CRITICAL** new findings (2 in S1, 5 in S2 from wave-11)
- **2 HIGH** new findings (S1)
- **3 round-1+2+3+4 regressions** (S1: F4-02/F4-03 reverted by wave-11; F4-01 partial)
- **~20 MEDIUM/LOW** new findings

## Per-Slice Rollup

| Slice | New CRIT | New HIGH | Regressions | Status |
|-------|----------|----------|-------------|--------|
| 1 (vb_core+vb_runtime) | 2 | 2 | 3 | REJECTED |
| 2 (vb_storage+workspace_tests) | 7 (5 wave-11 NEW) | 0 | 0 | REJECTED |
| 3 (vb_compile+vb_cli+vb_validate) | 0 | 0 | 0 | REJECTED (MEDIUM/LOW) |
| 4 (misc) | 0 | 0 | 0 | REJECTED (LOW) |
| **TOTAL** | **9** | **2** | **3** | |

## Convergence Tracker

| Round | CRITICAL | HIGH | Regressions | Notes |
|-------|----------|------|-------------|-------|
| 1    | 24 | 40 | n/a | Baseline |
| 2    | 25 | 25 | 19 | wave-5/6/7 reverted |
| 3    | 9  | 0  | 13 | wave-8/9 reverted |
| 4    | 2  | 3  | 0  | **CONVERGENCE — wave-10/11 stable** |
| 5    | 7  | 2  | 3  | wave-11 introduced 5 NEW CRITICAL + reverted 3 |

**Net trend: 24 → 25 → 9 → 2 → 7 CRITICALs.** Wave-11 is destabilizing again.

## Wave-11 Damage

- **5 NEW CRITICAL test-vs-production contract regressions** in S2:
  - `commit()` on aborted batch now returns `Err(BatchAborted)` but 5 external tests still expect `Ok(())`
  - `proptest_vb_vzcuf_PS_004.rs` and `journal_side_index_contracts.rs` affected
- **3 round-4 regressions** in S1: F4-02/F4-03 (lru_ring smoke), F4-01 (cancel_run partial)

## Top 5 Round-5 Fixes

| # | Fix | Effort | Catches |
|---|-----|--------|---------|
| 1 | Fix 5 wave-11 NEW CRITICAL: update test expectations to match `Err(BatchAborted)` or revert production | ~30 min | 5 broken tests |
| 2 | Re-apply F4-02 + F4-03 (lru_ring silent suppression + smoke Err) | ~10 min | LRU consistency invisible |
| 3 | Complete F4-01: cancel_run journal event capture | ~15 min | reason-propagation contract |
| 4 | S1 NEW CRITICAL: 5 vb_core test failures from wave-11+ production changes | ~1 hour | engine test contracts |
| 5 | S1 NEW HIGH: F5-07/08 round-4 regression follow-ups | ~15 min | LRU overflow variant check |

**Total cleanup for Top 5: ~2.5 hours.**

## Verdict

**STATUS: REJECTED.** 7 CRITICAL + 2 HIGH + 3 regressions. Wave-11 has destabilized after wave-10's stability. The loop is still active and necessary.

## Round 5 → Round 6 Plan

1. **Round 5.5 (CRITICAL priority)**: Fix the 5 wave-11 NEW CRITICAL + re-apply 3 regressions.
2. **Round 6**: Address remaining CRITICALs + HIGHs. By end of round 6, expect ≤3 CRITICAL + ≤3 HIGH.
3. **Rounds 7-10**: Drive remaining CRITICALs + HIGHs to zero.
4. **Rounds 11-20**: MEDIUMs → <10.
5. **Rounds 21-30**: LOWs to zero.
6. **Rounds 31-40**: Final APPROVED with OBSERVATION-only.
