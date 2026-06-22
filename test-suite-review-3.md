# Master Test Suite Review — Round 3 of 40

**Date:** 2026-06-21
**Reviewer:** test-reviewer (synthesis of 4 parallel slice reviews)
**Mode:** Round 3 — re-review post-wave-9 code; verify round-1+2 fixes still applied; find new defects.

## STATUS: REJECTED

Round 1+2 found and fixed 49 CRITICAL defects. Round 3 finds:

- **9 CRITICAL** new findings (slices 1+3+4)
- **4 HIGH** new findings
- **13 round-1+2 regressions** (the user's wave-8/9 work reverted many of my round-1+2 fixes)
- **1 wiring bug**: `vb_ipc/src/queue/tests/array_queue_tests.rs` is STILL dead code — the round-2 wiring was wrong (added to `queue/mod.rs` but should be in `lib.rs`)

All 4 slices: STATUS: REJECTED.

## Per-Slice Rollup

| Slice | New CRIT | New HIGH | Round-1+2 Regressions | Status |
|-------|----------|----------|----------------------|--------|
| 1 (vb_core+vb_runtime) | 6 | 2 | 4 | REJECTED |
| 2 (vb_storage+workspace_tests) | 0 | 0 | 0 | REJECTED (MEDIUM/LOW only) |
| 3 (vb_compile+vb_cli+vb_validate) | 1 | 0 | 8 | REJECTED |
| 4 (misc) | 2 | 0 | 1 | REJECTED |
| **TOTAL** | **9** | **2** | **13** | |

## Key New Findings

- **Slice 1 (6 CRITICAL)**: F3-01..F3-06 — wave-9 introduced new tests with `let _ = ...` silent discards and `assert!(*.is_ok())` smoke patterns
- **Slice 3 (1 CRITICAL + 8 regressions)**: wave-9 reverted F-10..F-15 + H-04..H-07 + M-04 (the round-1 S3 fixes in vb_compile/vb_cli/vb_validate)
- **Slice 4 (2 CRITICAL)**: `array_queue_tests.rs` STILL dead code — the round-2 wiring in `queue/mod.rs` doesn't work; need wiring in `lib.rs` (931 lines of dead tests)

## The Pattern: Wave Work Reverts My Fixes

Every round, the user's wave-8/9 work (8 more fix agents each) introduces:
- New tests with weak assertions (`is_ok()` smoke, `let _ = ...`)
- Reverts of my round-1+2 fixes (re-applied in working copy but not committed to the wave commits)

The cycle:
1. I find defects, fix them, file P1 beads
2. User's wave agents revert some fixes in their commits
3. I re-dispatch review, find the reverts, re-fix
4. User's next wave reverts again
5. ...

This is sustainable ONLY if I keep re-applying the reverts. The 40-round loop will be wasted cycles unless the wave work converges.

## Top 5 Round-3 Fixes Ranked by Impact

| # | Fix | Effort | Catches |
|---|-----|--------|---------|
| 1 | Wire `mod queue;` in `vb_ipc/src/lib.rs` (re-do of round-2 S4-01) | ~5 min | 931 lines of dead test code; round-1+2 FIFO fix non-functional |
| 2 | Re-apply 8 round-1 S3 fixes in vb_compile/vb_cli/vb_validate that wave-9 reverted | ~2 hours | CLI dispatch, budget field-reachability, taint test gaps |
| 3 | Re-apply 4 round-2 S1 fixes that wave-9 reverted (F2-04, F2-06, F2-07, F2-27) | ~30 min | recovery_bdd_tests + frame_pool smoke tests |
| 4 | Fix new wave-9-introduced `let _ = ...` patterns in slice 1 | ~30 min | silent test discards in workflow tests |
| 5 | Strengthen 4 round-1 S3 + 4 round-2 S3 + 1 new CRITICAL = 9 S3 defects | ~1 hour | CLI main_tests + app_impl_tests smoke patterns |

**Total cleanup for Top 5: ~4-5 hours.**

## Verdict

**STATUS: REJECTED.** 9 CRITICAL + 2 HIGH + 13 round-1+2 regressions. The wave-9 work has reverted many of my round-1+2 fixes. Round 3.5 must re-apply the 13 regressions BEFORE addressing the new findings.

## Round 3 → Round 3.5 → Round 4 Plan

1. **Round 3.5 (CRITICAL priority)**: Re-apply the 13 round-1+2 regressions. Especially the S4-001 array_queue_tests wiring fix.
2. **Round 4**: Address the 9 new CRITICAL + 2 new HIGH findings.
3. **Rounds 5-10**: Drive remaining CRITICALs to zero; expect MEDIUM drift.
4. **Rounds 11-20**: Drive HIGHs to zero; expect LOW drift.
5. **Rounds 21-30**: Drive MEDIUMs to <10.
6. **Rounds 31-40**: Drive LOWs to zero, final APPROVED with OBSERVATION-only.
