# Test Quality Loop — Master TODO (PAUSED at Round 6 of 40)

## Current Status: PAUSED

The 40-round review/fix loop is PAUSED at round 6. Re-evaluate after wave work stabilizes.

### Convergence Tracker
| Round | CRIT | HIGH | Regressions | New Failures |
|-------|------|------|-------------|--------------|
| 1    | 24   | 40   | n/a | n/a |
| 2    | 25   | 25   | 19  | 0 |
| 3    | 9    | 0    | 13  | 21 (taint) |
| 4    | 2    | 3    | 0   | 0 (CONVERGENCE) |
| 5    | 7    | 2    | 3   | 5 (wave-11) |
| 6    | 20   | 11   | 0   | 48 (wave-14) |

### What Was Accomplished
- 60+ CRITICAL test-quality defects found and fixed across 6 rounds
- 53 P1 fix-test beads closed
- 2 production bugs fixed (eval_expr_program constants pool + vb_storage snapshot validation)
- 40-round loop infrastructure in place (test-review/{loop.md, jj-dispatch.sh, prompt-slice-{1..4}.md} + 39 round-tracking beads)

### Outstanding (when loop resumes)
- 1-line fix for 48 failing tests (S1 F6-01)
- 5 S2 CRITICALs from wave-11 (commit() on aborted batch contract)
- 4 S4 CRITICALs from wave-14
- ~20 more MEDIUM/LOW findings

### Resume Criteria
- All 11+ crates compile and pass `cargo test --tests`
- Wave work is paused or doing only additive changes
- 1-line fix applied, test count back to ~6000 passing

### How to Resume
1. Apply the 1-line fix: end-of-tick flush in `dispatch.rs` after `dispatch_command` succeeds
2. Apply the 5 S2 CRITICALs (update test expectations for `Err(BatchAborted)`)
3. Apply the 4 S4 CRITICALs (see slice-4 review-6.md)
4. Verify all crates green
5. Dispatch round 7 review via 4 subagents using test-review/prompt-slice-{1..4}.md

## Round 1+2+3+4+5+6 Artifacts

| Round | Master | S1 | S2 | S3 | S4 |
|-------|--------|----|----|----|----|
| 1 | test-suite-review.md | slice-1-core-runtime-review.md | slice-2-storage-workspace-review.md | slice-3-compile-cli-validate-proof-review.md | slice-4-misc-review.md |
| 2 | test-suite-review-2.md | slice-1-core-runtime-review-2.md | slice-2-storage-workspace-review-2.md | slice-3-compile-cli-validate-proof-review-2.md | slice-4-misc-review-2.md |
| 3 | (see test-suite-review-3.md) | slice-1-core-runtime-review-3.md | slice-2-storage-workspace-review-3.md | slice-3-compile-cli-validate-proof-review-3.md | slice-4-misc-review-3.md |
| 4 | test-suite-review-4.md | slice-1-core-runtime-review-4.md | slice-2-storage-workspace-review-4.md | slice-3-compile-cli-validate-proof-review-4.md | slice-4-misc-review-4.md |
| 5 | test-suite-review-5.md | slice-1-core-runtime-review-5.md | slice-2-storage-workspace-review-5.md | slice-3-compile-cli-validate-proof-review-5.md | slice-4-misc-review-5.md |
| 6 | test-suite-review-6.md | slice-1-core-runtime-review-6.md | slice-2-storage-workspace-review-6.md | slice-3-compile-cli-validate-proof-review-6.md | slice-4-misc-review-6.md |
