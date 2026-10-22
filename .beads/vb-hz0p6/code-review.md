# Code Review: vb-hz0p6 CLI Isolation

**Reviewer:** LOCAL5090 independent review
**Candidate:** fd86403e3 (origin/codex/vb-8li6d-local5090-author)
**Base:** d5f389344 (origin/main)
**Scope:** crates/workspace_tests/tests/vb_test_cli_run_lifecycle_behavior.rs

## PHASE 1: Contract & Bead Parity

Bead vb-8li6d contract requires:
1. CLI lifecycle tests must not inherit repository cwd
2. Use shared run_vb isolation per invoke
3. Strict absolute binary resolution
4. TempDir lifetime covers awaited process
5. No global chdir
6. All 79 existing assertions preserved
7. New no-repository-artifact regression test

**Verified:** Requirements 1, 2, 3, 4, 5, 6 are satisfied in the implementation. Requirement 7 is satisfied in form but has a mutation weakness (see PHASE 3).

## PHASE 2: Farley Engineering Rigor

- Functions under 25 lines: `run_vb_internal` (12 lines), `run_vb_doctor_in_temp` (14 lines), `run_vb` (2 lines) — all compliant.
- Pure/IO separation: The test helpers are IO shell; tests use public API only. Good.
- Parameter count: All functions have ≤3 parameters. Good.

## PHASE 3: Holzman Rust (Big 6) + Mutation Resistance

**CRITICAL FINDING: Mutation resistance is broken.**

The regression test `cli_doctor_with_relative_db_no_source_tree_artifacts` (line 764) claims in its doc comment: "The test fails if the child current_dir override is removed."

**This claim is false.** I verified with an actual mutation test:

1. Applied mutation: removed `cmd.current_dir(tmp_dir.path());` from `run_vb_internal` (line 37)
2. Left `run_vb_doctor_in_temp` unchanged (it still has `current_dir`)
3. Ran the regression test: **it passed** (exit code 0, 1 passed, 0 failed)

The test only detects removal of `current_dir` from `run_vb_doctor_in_temp`, NOT from the shared `run_vb_internal` helper where the actual fix for all 79 existing tests lives.

**Root cause:** The implementation duplicated the helper logic. `run_vb_doctor_in_temp` is a near-copy of `run_vb_internal` that exists only to return the TempDir for inspection. If the shared helper regresses, 79 tests could be affected but the regression test would not catch it.

**Evidence:**
- Source hash before mutation: ae34526c32aba1db5da6406ed0e6afcd8fb39278dfed8a805f934aa87af4a181
- Mutation applied: CUT line 37 (`cmd.current_dir(tmp_dir.path());` in `run_vb_internal`)
- Result: test passed (REVIEW-MUTATION.stdout shows "1 passed; 0 failed")
- Source hash after restore: ae34526c32aba1db5da6406ed0e6afcd8fb39278dfed8a805f934aa87af4a181 (matches)

## PHASE 4: Panic Vector & Legibility

- No new `unsafe`, `unwrap`, `panic!`, `todo!`, `unimplemented!`, or `dbg!`.
- `expect` count: 1 (in `run_vb` wrapper) — same as base.
- Error propagation uses `Result`/`?` in `run_vb_internal`. Good.
- Code is readable and straightforward.

## PHASE 5: Bitter Truth

- No cleverness; boring design as required.
- No YAGNI violations.
- Absolute binary resolution is correct and necessary.

## Summary of Findings

| Severity | Finding |
|----------|---------|
| BLOCKER | Regression test does not detect removal of `current_dir` from shared `run_vb_internal` helper. Doc comment claim is false. Mutation test verified: test passes after removing the fix from the shared helper. |
| LOW | Code duplication between `run_vb_internal` and `run_vb_doctor_in_temp` (10 of 14 lines identical). |

**STATUS: REJECTED**

Reason: The regression test provides a false guarantee. The stated contract ("test fails if child current_dir override is removed") is not true for the actual shared helper. The author's mutation verification claimed success, but that is contradicted by the raw test output from the actual mutation test. The duplicated helper is the root cause; the regression test should either (a) use the shared `run_vb_internal` directly with a way to inspect the working directory, or (b) the implementation should be refactored so there is only one helper that can optionally return the TempDir.