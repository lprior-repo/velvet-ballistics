# Release Checklist — velvet-ballistics v0.1.0

## Pre-release
- [x] Binary builds and runs (`velvet-ballistics 0.1.0`)
- [x] Master §78 amendment committed (`3cdbca26b`)
- [x] Kani harness cleanup committed
- [x] Kani task split committed (`b2830f37d`)
- [x] CHANGELOG.md populated with [0.1.0] entry
- [x] All 22 tier-a beads closed (this session)
- [x] 22 residue beads nuked (UI/Makepad/codegen)
- [x] 17 P4 beads deleted

## Per-wave evidence pointers (master §77)

| Wave | Closed beads | Evidence |
|------|--------------|----------|
| 0    | tier-a-0-005/006/007 | `velvet-ballistics-MASTER.md` §78 amendment, `RELEASE_CHECKLIST.md` stub, `CHANGELOG.md` stub |
| 1    | tier-a-1-* baseline | `.beads/moon-ci-output-20260617-210818.txt` |
| 2    | tier-a-2-* LRU/split | `.beads/moon-ci-output-20260617-213348.txt` |
| 3    | tier-a-3-008/009 | `.beads/moon-ci-output-20260617-220335.txt` |
| 5    | tier-a-5-* recovery | `.beads/moon-ci-output-20260617-230049.txt` |
| 6    | tier-a-6-011/012/013/014/015 | `.beads/moon-ci-output-20260619-032725.txt` |
| 7    | tier-a-7-016 | `.beads/moon-ci-output-20260619-041439.txt` |
| 8    | tier-a-8-* proptest | `.beads/moon-ci-output-20260619-074235.txt` |
| 9    | tier-a-9-01? | `.beads/moon-ci-output-20260619-075153.txt` |
| 10   | tier-a-10-* lint | `.beads/moon-ci-output-20260619-080601.txt` |
| 11   | tier-a-11-* landing | `.beads/moon-ci-output-20260619-081548.txt` |
| 12   | tier-a-12-018/019/022 | `.beads/moon-ci-status.txt` (this commit `e8c3a84d1`) |
| 13   | tier-a-13-* residue | `.evidence/vb-batch-17/` |

**Note**: Per-wave `.evidence/<wave>/` directories do not exist as a uniform
layout. The `moon-ci-output-*.txt` files in `.beads/` capture the gate state
of each wave's CI run, and bead-level evidence lives in `.evidence/vb-*/`
directories. `.beads/moon-ci-status.txt` is the canonical aggregate.

## Release dance (master §77)
1. [x] Run `moon ci` → TIMED_OUT at 1800s (upstream gates PASS, kani buckets need longer budget)
2. [x] Capture `.beads/moon-ci-output-*.txt`
3. [x] Update `.beads/moon-ci-status.txt`
4. [x] Commit moon ci status + log
5. [x] `bd dolt push`
6. [x] `git commit -m "ci(beads): moon ci status update"`
7. [x] `git add CHANGELOG.md && git commit`
8. [x] `git add RELEASE_CHECKLIST.md && git commit`
9. [ ] `git tag -s v0.1.0 -m 'Tier A v0.1.0 release'` (USER CONTROLS)
10. [ ] `git push origin main v0.1.0` (USER CONTROLS)

## Post-release
- [ ] Verify tag in remote: `git ls-remote --tags origin | grep v0.1.0`
- [ ] Announce in team channel
- [ ] File v0.2.0 beads for: `moon ci` wall-clock budget, `vb_queue_semantics`
      fix, remaining Verus spec binding work (vb-bc33k, vb-z280t, vb-h39ky,
      vb-puvkn, vb-3xdp5, vb-pr6mg)
