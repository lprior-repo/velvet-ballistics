# Test Suite Review: vb-hz0p6 CLI Isolation

**Reviewer:** LOCAL5090 independent review
**Candidate:** fd86403e3 (origin/codex/vb-8li6d-local5090-author)
**Test target:** vb_test_cli_run_lifecycle_behavior.rs

## Test Count Verification

- Tests listed by `--list`: 194 (includes binary variants)
- Tests actually run: 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
- New tests added: 1 (`cli_doctor_with_relative_db_no_source_tree_artifacts`)
- Existing tests: 79 (all preserved)

Claim of "80/80 tests pass" in author handoff is correct.

## Suite Review Gates

1. **Tests compile and execute deterministically:** PASS. Ran 3 times (baseline, mutation, final), all deterministic. Exit code 0, 80 passed.
2. **Integration tests use public API only:** PASS. Tests invoke the CLI binary via process execution.
3. **Tests assert behavior, not implementation details:** PASS. Tests assert exit codes, stdout content, and file system state — observable behavior.
4. **No ignored tests, sleeps, broad mocks, hidden shared mutable state, or silent error suppression:** PASS. Clean suite.
5. **Mutation thought experiment:** FAIL (see detailed finding below).
6. **Snapshot tests checked and intentional:** N/A — no snapshot tests.
7. **Resource-heavy commands bounded:** PASS. Tests are lightweight CLI invocations.
8. **No commented-out tests, dormant modules, ignored properties, or zero-test runs:** PASS. All 80 tests executed.

## Detailed Mutation Finding

**Gate 5 failure:** The regression test `cli_doctor_with_relative_db_no_source_tree_artifacts` (line 764) is vulnerable to a specific mutation that breaks the contract.

**Mutation applied:** Remove `cmd.current_dir(tmp_dir.path());` from `run_vb_internal` (the shared helper used by all 79 existing tests). Leave `run_vb_doctor_in_temp` unchanged.

**Expected result:** Test should fail because CLI commands would now run in the inherited repository cwd, creating database artifacts in the source tree.

**Actual result:** Test PASSED. Exit code 0, 1 passed, 0 failed.

**Why:** `run_vb_doctor_in_temp` has its own `current_dir` call. The regression test uses this duplicated helper, not the shared `run_vb_internal`. Removing the fix from the shared helper leaves 79 tests vulnerable to cwd inheritance, but the regression test does not detect it.

**Evidence:** Raw logs at `.beads/vb-hz0p6/evidence/REVIEW-MUTATION.{json,stderr,stdout}`. Source hash before and after mutation confirmed identical (ae34526c32aba1db5da6406ed0e6afcd8fb39278dfed8a805f934aa87af4a181).

## Assertion Strength Assessment

The regression test has strong assertions:
- Asserts exit code success (line 771)
- Asserts expected output content (line 781)
- Asserts Fjall journal created in child's temp dir (line 789)
- Asserts no journal in inherited repo cwd (line 800)

All four assertions are concrete and behavior-oriented. The weakness is not in assertion quality but in which helper the test exercises.

## Test Independence

Tests are independent: each invokes a fresh CLI process in its own temp directory. No shared mutable state between tests. PASS.

## Summary

| Gate | Result |
|------|--------|
| Compile and execute deterministically | PASS |
| Public API only | PASS |
| Assert behavior, not implementation | PASS |
| No ignored/sleeps/mocks/shared state/suppression | PASS |
| Mutation thought experiment | FAIL |
| Snapshot tests checked | N/A |
| Resource-heavy commands bounded | PASS |
| No dormant/ignored/zero-test runs | PASS |

**STATUS: REJECTED**

Reason: Gate 5 failed. The regression test does not detect the specific mutation that would remove the cwd isolation fix from the shared helper. This is a mutation-resistance defect in the test suite itself.