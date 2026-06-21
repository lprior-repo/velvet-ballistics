# Test Review/Fix Loop — 40-Round Plan

## Purpose

Drive the velvet-ballistics test suite from REJECTED (Round 1: 24 CRITICAL, 40 HIGH)
to APPROVED (Round 40: observation-only) via 40 rounds of adversarial review, fix-test
bead filing, parallel fix dispatch, build verification, and JJ push.

Each round is a closed loop: **review → bead → fix → verify → push → close**.

## Round Status

| Round | Status | Review Artifact | Fix Beads | Verify | Push |
|-------|--------|-----------------|-----------|--------|------|
| 1     | IN PROGRESS (review done; fixes in flight) | `.evidence/test-review/slice-{1..4}-*.md` (34C / 33H / 26M / 21L) + `test-suite-review.md` | 1 P1 (vb-lynec) + 7 P2 (vb-2yzii, vb-42jci, vb-jtic4, vb-nkymh, vb-qt3uy, vb-sgfcb, vb-w0n0t) | pending | pending |
| 1.5   | PLANNED | re-dispatch slice-1 to close F-05..F-10 mutation gaps | (same vb-lynec + new P1s for F-05..F-08) | TBD | TBD |
| 2     | NOT STARTED | `slice-{core-runtime,storage-workspace,compile-cli-validate-proof,misc}-review-2.md` | TBD (`vb-8o0ul`) | TBD | TBD |
| 3     | NOT STARTED | TBD | TBD (`vb-f4t4j`) | TBD | TBD |
| 4     | NOT STARTED | TBD | TBD (`vb-psb5f`) | TBD | TBD |
| 5     | NOT STARTED | TBD | TBD (`vb-rxwux`) | TBD | TBD |
| 6     | NOT STARTED | TBD | TBD (`vb-gm7n7`) | TBD | TBD |
| 7     | NOT STARTED | TBD | TBD (`vb-kt8e1`) | TBD | TBD |
| 8     | NOT STARTED | TBD | TBD (`vb-2bknr`) | TBD | TBD |
| 9     | NOT STARTED | TBD | TBD (`vb-mo33q`) | TBD | TBD |
| 10    | NOT STARTED | TBD | TBD (`vb-dl2o1`) | TBD | TBD |
| 11-19 | NOT STARTED | TBD | TBD | TBD | TBD |
| 20    | NOT STARTED | TBD | TBD (`vb-dbwmx`) | TBD | TBD |
| 21-29 | NOT STARTED | TBD | TBD | TBD | TBD |
| 30    | NOT STARTED | TBD | TBD (`vb-wkpi3`) | TBD | TBD |
| 31-39 | NOT STARTED | TBD | TBD | TBD | TBD |
| 40    | NOT STARTED | TBD | TBD (`vb-ejwgp`) | TBD | TBD |

Bead IDs for rounds 2-40 are filed in `bd list -p 3 --label test-review` (39 P3 task beads).

## Loop Protocol

For each round N (1-40):

1. **Review** (test-reviewer): re-dispatch 4 subagents in parallel (slice 1-4) against
   the post-fix code from round N-1. Output: `slice-{1..4}-NAME-review-N.md` + master
   `test-suite-review-N.md`.
2. **File beads** (test-reviewer): for each new CRITICAL finding, file a P1 fix-test
   bead. For each new HIGH, file a P2.
3. **Fix** (test-writer): dispatch parallel subagents to fix the new CRITICAL
   findings.
4. **Verify** (build-verifier): `cargo test` on all affected crates.
5. **Push** (build-verifier): `jj describe` + `jj git push`.
6. **Close beads** (test-reviewer): close fix-test beads with evidence.

## Convergence Targets

- **Round 10**: all CRITICALs CLOSED
- **Round 20**: all HIGHs CLOSED
- **Round 30**: <10 MEDIUMs
- **Round 40**: APPROVED, OBSERVATION-only

## Per-Round Subagent Prompts

The 4 subagent prompts (re-usable) live next to this file:

- `prompt-slice-1.md` — review vb_core + vb_runtime
- `prompt-slice-2.md` — review vb_storage + workspace_tests
- `prompt-slice-3.md` — review vb_compile + vb_cli + vb_validate + vb_proof_kernels
- `prompt-slice-4.md` — review vb_expr + vb_ipc + vb_yaml + vb_queue_semantics
  + vb_boundary_inventory + vb_benchmark + vb_test_util + vb_doc
  + vb_ajc40_flux + vb_verification

Each prompt instructs the subagent to:

1. Sweep all test files in its slice with `rg` for banned patterns.
2. Deep-read highest-density files (largest line count, highest mutation risk).
3. Run `cargo test -p <crate> --tests` on a representative sample.
4. Write findings to `.evidence/test-review/slice-NAME-review-${ROUND}.md`.
5. Report: STATUS, count of CRITICAL/HIGH/MEDIUM/LOW, top 5 fixes.

## Banned Patterns (enforced across all slices)

- `assert!(result.is_ok())` / `assert!(result.is_err())` — hides variant info.
- `match result { Ok(_) => .., Err(_) => .. }` — erases payload.
- `Some(_)` in pattern matches — discards the captured value.
- `unwrap()` / `expect()` in behavior asserts — panics on failure lose assertion meaning.
- `#[ignore]` without an `// reason:` annotation — silently skipped tests.
- `std::thread::sleep` / `tokio::time::sleep` in tests — non-deterministic.
- `let _ = expr;` — discards a value silently.
- `assert_eq!(a, a)` tautologies.
- `#[cfg(feature = "kani")]` harnesses that hardcode data instead of using
  `kani::Arbitrary` / `kani::any()`.

## Mutation Thought Experiment (mandatory per finding)

For every CRITICAL/HIGH, the reviewer must answer:

> If we mutate the production code to the 3 most dangerous variants (off-by-one,
> missing-bounds-check, default-on-error), would the existing test catch it?
>
> If **no**, this is a mutation gap → CRITICAL.

## Automation

`./test-review/jj-dispatch.sh <round>` validates that all 5 artifacts exist for a
given round before reporting readiness to file fix-test beads. Use it as a guard
between steps 1 and 2.

## See Also

- `test-suite-review.md` — Round 1 master review (workspace root)
- `.evidence/test-review/slice-{1..4}-*.md` — Round 1 slice reviews
- `.beads/` — bead tracker (39 round-2-40 tracking beads filed by infra-builder)
- `test-review/prompt-slice-{1,2,3,4}.md` — re-usable subagent prompts
- `test-review/jj-dispatch.sh` — artifact validator
