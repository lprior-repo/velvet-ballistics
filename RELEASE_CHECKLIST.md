# Release Checklist — velvet-ballistics v0.1.0

## Pre-release
- [x] Binary builds and runs (`velvet-ballistics 0.1.0`)
- [x] Master §78 amendment committed (`3cdbca26b`)
- [x] Kani harness cleanup committed
- [x] Kani task split committed (`b2830f37d`)
- [x] CHANGELOG.md populated
- [ ] All 22 tier-a beads closed (deferred to v0.2.0)
- [x] 22 residue beads nuked (UI/Makepad/codegen)
- [x] 17 P4 beads deleted

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
      fix, Tier A wave 0+ closure
