# Test Quality Loop — Master TODO

## Goal

Drive the velvet-ballistics test suite from **REJECTED** (Round 1 surfaced 34
CRITICAL + 33 HIGH + 26 MEDIUM + 21 LOW across the 4 slices) to **APPROVED**
(observation-only) over 40 rounds of adversarial review, fix-test bead filing,
parallel fix dispatch, build verification, and jj push.

## Round Status

| Round | Status | Review Artifacts | Fix Beads | Verified | Pushed |
|-------|--------|------------------|-----------|----------|--------|
| 1     | IN PROGRESS — review done; fixes in flight | `test-suite-review.md` + `.evidence/test-review/slice-{1..4}-*.md` | 1 P1 + 7 P2 (see table below) | pending | pending |
| 1.5   | PLANNED — re-dispatch slice-1 to close F-05..F-10 mutation gaps | same slice-1 prompt + delta findings | new P1s for F-05..F-08 | TBD | TBD |
| 2     | NOT STARTED | `.evidence/test-review/slice-{core-runtime,storage-workspace,compile-cli-validate-proof,misc}-review-2.md` | TBD (`vb-8o0ul`) | TBD | TBD |
| 3-10  | NOT STARTED | TBD | TBD | TBD | TBD |
| 11-20 | NOT STARTED | TBD | TBD | TBD | TBD |
| 21-30 | NOT STARTED | TBD | TBD | TBD | TBD |
| 31-40 | NOT STARTED | TBD | TBD | TBD | TBD |

Round 2..40 tracking beads live at priority P3 with label `test-review`.
`bd list -p 3 --label test-review` returns 39 task beads (one per round).

### Round 1 Fix Beads (filed)

| Bead ID | Priority | Title | Status |
|---------|----------|-------|--------|
| vb-lynec | P1 | fix-test: S1-C9/C10 — recovery_bdd_tests.rs:2728,2883 concrete post-conditions | open |
| vb-nkymh | P2 | fix-test: S1-H1..H10 — vb_runtime smoke is_ok/is_err -> variant match (10 sites) | open |
| vb-sgfcb | P2 | fix-test: S1-H11..H17 — vb_core+vb_runtime smoke unwrap+concrete combos (7 sites) | open |
| vb-qt3uy | P2 | fix-test: S1-H18..H20 — proptest variant match (3 proptest files) | open |
| vb-42jci | P2 | fix-test: S2-H1..H10 — vb_storage smoke -> concrete (10 HIGH sites) | open |
| vb-jtic4 | P2 | fix-test: S3-H1..H10 — vb_compile/cli HIGH smoke (10 sites) | open |
| vb-2yzii | P2 | fix-test: S3-H11..H12 — vb_cli json shape + lifecycle error variant (2 HIGH sites) | open |
| vb-w0n0t | P2 | fix-test: S4-H1..H8 — misc HIGH smoke (8 sites) | open |

Total: 1 P1 + 7 P2. The CRITICAL sites in slices 2/3/4 (4+12+4 = 20 CRITICALs) are
unfiled at fix-test priority — they overlap with the bug-hunt P0 beads under
`vb-1rqz7.*` (storage bug-hunt) and `vb-7n5h8` (vb_runtime test failures).
Triage decision deferred to next dispatch.

## Per-Round Protocol

For each round N:

1. **Review** — re-dispatch 4 subagents (slices 1-4) using `test-review/prompt-slice-{1..4}.md`
   templates against post-round-(N-1) code. Outputs: `.evidence/test-review/slice-NAME-review-N.md`.
2. **File beads** — for each new CRITICAL finding, file a P1 fix-test bead; for each new HIGH,
   file a P2. Use `bd create -p 1|2 -t task --assignee test-reviewer --label test-review`.
3. **Fix** — dispatch parallel `test-writer` subagents to address the new P1/P2 beads. Fixers
   read the finding's `Recommended fix` column in the slice review and apply the smallest
   concrete patch that survives the documented mutation thought experiment.
4. **Verify** — `cargo test -p <crate> --tests` on each affected crate. Must compile + green.
5. **Push** — `jj describe` with the round-N summary + `jj git push`.
6. **Close beads** — close fix-test beads with `bd close <id> --notes "<evidence>"`.
7. **Update this file** — flip Round N row from NOT STARTED to COMPLETE; flip Round N+1 to IN PROGRESS.

## Convergence Targets

- **Round 10**: all CRITICALs CLOSED (Round 1 baseline: 34 CRITICAL across 4 slices)
- **Round 20**: all HIGHs CLOSED (Round 1 baseline: 33 HIGH across 4 slices)
- **Round 30**: <10 MEDIUMs open (Round 1 baseline: 26 MEDIUM)
- **Round 40**: APPROVED, OBSERVATION-only (Round 1 baseline: 21 LOW + 19 OBS)

## Round 1 Specifics (current state)

Slice review produced these finding totals (from `rg` over `^\| .* \| <SEV> \|`
rows in each slice review):

| Slice | Crates | Files | CRITICAL | HIGH | MEDIUM | LOW |
|-------|--------|-------|----------|------|--------|-----|
| 1 — core-runtime | vb_core + vb_runtime | 261 | 10 (F-01..F-10) | 11 | 6 | 3 |
| 2 — storage-workspace | vb_storage + workspace_tests | 313 | 8 | 10 | 11 | 8 |
| 3 — compile-cli-validate-proof | vb_compile + vb_cli + vb_validate + vb_proof_kernels | 619 | 12 | 12 | 9 | 10 |
| 4 — misc | vb_expr + vb_ipc + vb_yaml + vb_queue_semantics + vb_boundary_inventory + vb_benchmark + vb_test_util + vb_doc + vb_ajc40_flux + vb_verification | n/a | 4 (S4-001..S4-004) | 0 | 0 | 0 |
| **Total** | | **~1300+** | **34** | **33** | **26** | **21** |

Round 1 review summary by slice:

- **S1 (vb_core+vb_runtime)**: 10 CRITICAL sites (F-01..F-10). Mutation gap on every
  `assert!(result.is_ok())` / `assert!(result.is_err())` without a concrete variant match.
  F-09 and F-10 (recovery_bdd_tests.rs:2883 / :2728) covered by vb-lynec (P1).
  F-01..F-04 covered by HIGH beads (vb-nkymh + vb-sgfcb). F-05..F-08 still pending — likely
  need separate P1 filings for round 1.5.
- **S2 (vb_storage+workspace_tests)**: 8 CRITICAL sites. Tautology assertions on
  compile-error / recovery / process-lock paths. All folded into vb-42jci (HIGH) — the
  CRITICALs overlap with the bug-hunt P0 family (`vb-1rqz7.*`).
- **S3 (vb_compile+vb_cli+vb_validate+vb_proof_kernels)**: 12 CRITICAL sites. The biggest
  pattern is `if let Ok(Command::X) = parsed { real } else { assert!(parsed.is_ok()) }` — 68+
  sites in `vb_cli/args/tests/*.rs`. Folding into vb-jtic4 (HIGH) + the bug-hunt P0 family.
- **S4 (vb_expr+vb_ipc+vb_yaml+etc)**: 4 CRITICAL sites. Orphaned
  `and_or_short_circuit_tests.rs` (1619 lines, S4-001), local-reimplementation in
  `vb_ajc40_flux/tests/density_tests.rs` (S4-002), `crossbeam_channel` misuse in
  `vb_ipc/src/tests.rs:445` (S4-003), FIFO proptest with discarded frame data (S4-004).
  These are unfiled as fix-test beads — needs triage vs. existing P1s.

Pre-existing build breakages (NOT part of this TODO, separate workstreams):
- 245+ errors in vb_runtime from WIP commits (tracked by `vb-7n5h8`)
- `crossbeam_channel` usage in vb_ipc tests (S4-003 fix addresses one site)
- Section 46 no-short-circuit gap (S4-001 fix adds tests)

## Tools

- `test-review/jj-dispatch.sh N` — validate round N artifacts are in place; prints
  CRITICAL/HIGH/MEDIUM/LOW counts per slice.
- `test-review/prompt-slice-{1,2,3,4}.md` — re-usable subagent prompts (templated `${ROUND}`).
- `test-review/loop.md` — round tracker with bead IDs.
- `test-suite-review.md` (workspace root) — round 1 master review.
- `.evidence/test-review/slice-*.md` — round 1 slice reviews.

## Invocation

To run round N:

```bash
# 1. Validate prior-round artifacts are in place.
bash test-review/jj-dispatch.sh N

# 2. Dispatch 4 subagents in parallel, one per slice, using the prompt templates.
#    Each writes: .evidence/test-review/slice-NAME-review-N.md
#    Use the test-reviewer agent per bead: bd update vb-XXXXX --claim

# 3. Synthesize master: test-suite-review-N.md (workspace root).

# 4. File fix-test beads for new CRITICALs (P1) and HIGHs (P2).

# 5. Dispatch parallel test-writer subagents to fix.

# 6. cargo test on each affected crate.

# 7. jj describe "test-review: round N — <summary>" && jj git push.

# 8. Close fix-test beads with evidence; flip round-N row in this file.
```

## Status Definitions

- **NOT STARTED** — no slice review exists for this round.
- **IN PROGRESS** — slice reviews exist; fixes being dispatched.
- **COMPLETE** — slice reviews done, fix-test beads filed, fixes applied, `cargo test`
  green, jj push landed, beads closed.
- **APPROVED** — slice review STATUS line is APPROVED (no CRITICAL or HIGH); loop terminates
  early.

## Notes

- The bug-hunt P0 beads (`vb-1rqz7.*` family, `vb-7n5h8`) address different defects than
  the slice-review CRITICALs but overlap heavily. The fix-test beads in this loop focus on
  the test-quality defects (smoke-vs-concrete, mutation gaps); the bug-hunt beads focus on
  production-code defects (cancel markers, recovery ordering, codec integrity). Both
  workstreams share the same verification gate (`cargo test -p <crate>`).
- This TODO file is the single source of truth for the loop's progress. Update the round
  status table at every state transition.
