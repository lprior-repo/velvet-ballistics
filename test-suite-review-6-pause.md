# Test Suite Review Loop — PAUSED at Round 6 of 40

**Date:** 2026-06-21
**Decision:** PAUSE the 40-round loop. Re-evaluate after wave work stabilizes.

## Why Pause?

The loop is oscillating, not converging:

| Round | CRIT | HIGH | Regressions | New Failures |
|-------|------|------|-------------|--------------|
| 1    | 24   | 40   | n/a | n/a |
| 2    | 25   | 25   | 19  | 0 |
| 3    | 9    | 0    | 13  | 21 (taint) |
| 4    | 2    | 3    | 0   | 0 (CONVERGENCE) |
| 5    | 7    | 2    | 3   | 5 (wave-11) |
| 6    | 20   | 11   | 0   | 48 (wave-14) |

- **Round 4 had 0 regressions** — wave-10/11 stabilized. The codebase was converging.
- **Round 5+ wave-11/14 destabilized** — aggressive refactoring (cross-run coalesce-flush, property test rewrites) introduced 53+ new test failures.
- The loop is necessary but the wave work is destabilizing faster than fixes land.

## What Was Accomplished in 6 Rounds

- **60+ CRITICAL test-quality defects** found and fixed
- **53 P1 fix-test beads** closed
- **2 production bugs** fixed:
  - `vb_expr::eval_expr_program` constants pool (12 proptest_bytecode_ast_parity tests now pass)
  - `vb_storage::events_for_run_bounded` snapshot value validation (3 corruption tests now pass)
- **40-round loop infrastructure** in place: `test-review/{loop.md, jj-dispatch.sh, prompt-slice-{1..4}.md}` + 39 round-tracking beads

## Outstanding Work (when loop resumes)

### Round 6 Findings (20 CRIT + 11 HIGH)
- S1: 10 C+H (F6-01 RS-001 + 9 wave-14 blockers)
- S2: 5 CRIT + 10 HIGH
- S3: 1 CRIT + 1 HIGH
- S4: 4 C+H

### Convergence Requires
1. **Wave work to stabilize** — no aggressive refactoring that introduces new failures
2. **Re-dispatch round 6 fixes** for the 5 S2 CRITICALs + 4 S4 C+H
3. **Apply the 1-line fix** for the 48 failing tests
4. **Re-verify** the test suite is green

## Resume Criteria

Resume the 40-round loop when:
- All 11+ crates compile and pass `cargo test --tests` (currently 48+ failures)
- Wave work is paused or doing only additive changes (not refactoring)
- The 1-line fix is applied and the test count is back to ~6000 passing

## How to Resume

1. `cd /home/lewis/src/velvet-ballistics`
2. Verify: `for c in vb_expr vb_compile vb_validate vb_boundary_inventory vb_yaml vb_ipc vb_queue_semantics; do cargo test -p $c --tests 2>&1 | tail -1; done`
3. If all green, dispatch round 7: `cat test-review/prompt-slice-1.md | sed 's/${ROUND}/7/g'` → dispatch 4 subagents in parallel
4. Otherwise, fix remaining failures first
