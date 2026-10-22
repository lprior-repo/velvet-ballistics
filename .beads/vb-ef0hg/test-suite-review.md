# Test Suite Review: vb-ef0hg

**Candidate:** 36ba70788954338bbffbf38cc70e4dd260ff6dfb
**Source hash before:** ccac03cccff3787bcd57aab6603dd61709c1be85cd236a89c5a511ce84f1d1ba
**Source hash after:** ccac03cccff3787bcd57aab6603dd61709c1be85cd236a89c5a511ce84f1d1ba
**Hash identical after restoration:** true

## Test Scope

File: `crates/workspace_tests/tests/vb_test_cli_run_lifecycle_behavior.rs`
Total tests: 80
Test count preserved: true

## Evidence Log

| Test run | Tests | Exit | Outcome | Evidence |
|----------|-------|------|---------|----------|
| TEST-V1-baseline | 80 | 0 | PASS | `.beads/vb-ef0hg/evidence/TEST-V1-b.*` |
| TEST-V1-mutant | 1 | 101 | FAIL (expected) | `.beads/vb-ef0hg/evidence/TEST-V1-m.*` |
| TEST-V1-restored | 80 | 0 | PASS | `.beads/vb-ef0hg/evidence/TEST-V1-r.*` |
| TEST-V1-fmt | N/A | 0 | PASS (fmt check) | `.beads/vb-ef0hg/evidence/TEST-V1-fmt.*` |

## Mutation Result

The shared helper `run_vb_internal` contains the critical working directory isolation line `cmd.current_dir(tmp_dir.path())`. Removing this single line causes the child process to run in the inherited repository cwd instead of an isolated temp directory.

The regression test `cli_doctor_with_relative_db_no_source_tree_artifacts` detects this mutation:
- Test passes when `current_dir` is set (original behavior)
- Test fails when `current_dir` is removed (mutant behavior)

This proves the test actually exercises the shared helper and validates the working directory isolation contract.

## Artifacts Preserved

- Leftover `vb-isolation-v4.db` created during mutation test was moved from `crates/workspace_tests/vb-isolation-v4.db` to `/tmp/vb-test-cache/vb-isolation-v4-mutation-artifact.db`
- No preexisting databases were overwritten or deleted

## Findings

All findings are resolved with evidence:

1. **Shared helper mutation fails as expected** — `cli_doctor_with_relative_db_no_source_tree_artifacts` (exit 101) confirms the test is a genuine regression guard against removing the child working directory override. Disposition: `fixed_with_evidence`.

2. **Process lifetime and binary canonicalization verified** — `run_vb_internal` first canonicalizes the binary path to absolute form and sets `current_dir` to the child-owned temp directory before invoking `output()`. The temp directory lifetime spans the child process execution, preventing premature cleanup. Disposition: `fixed_with_evidence`.

3. **Test coverage for child working directory isolation complete** — `cli_doctor_with_relative_db_no_source_tree_artifacts` asserts child success, DB presence in child-owned temp directory, DB absence in inherited repository cwd, and failure if the override is removed. Disposition: `fixed_with_evidence`.

## Status

**APPROVED**

- Shared-helper mutation fails (exit 101) as required
- Original source exactly restored (identical hash)
- All 80 tests pass after restoration (exit 0)
- Formatter check passes (exit 0)
- No open findings