# Code Review v3: vb-hz0p6 CLI Isolation

**Reviewer:** LOCAL5090 independent code review (fresh agent)
**Reviewed source:** ccac03cccff3787bcd57aab6603dd61709c1be85cd236a89c5a511ce84f1d1ba
**Reviewed file:** crates/workspace_tests/tests/vb_test_cli_run_lifecycle_behavior.rs
**Base:** d5f38934 (origin/main)
**Candidate:** 36ba70788954338bbffbf38cc70e4dd260ff6dfb (formatted)
**Evidence runner:** python3 /tmp/vb-run-gate.py

## Evidence Commands

### Test Suite
```
Command: python3 /tmp/vb-run-gate.py 300 .beads/vb-hz0p6/evidence/CODE-V3-cli cargo test -p velvet-ballistics-workspace-tests --test vb_test_cli_run_lifecycle_behavior
Exit: 0
Time: 0.118s
Evidence: .beads/vb-hz0p6/evidence/CODE-V3-cli.json/.stdout/.stderr
```

### Format Check
```
Command: python3 /tmp/vb-run-gate.py 300 .beads/vb-hz0p6/evidence/CODE-V3-fmt rustup run nightly-2026-04-28 cargo fmt --all --check
Exit: 0
Time: 1.588s
Evidence: .beads/vb-hz0p6/evidence/CODE-V3-fmt.json/.stdout/.stderr
```

## Code Inspection

### Shared Helper Structure
The code correctly restructures CLI test invocations through a single shared helper:

- `run_vb_internal` (lines 30-42, 13 lines): Canonicalizes binary to absolute path (`std::fs::canonicalize`), creates fresh per-invocation `TempDir`, sets `cmd.current_dir(tmp_dir.path())`, awaits `cmd.output()`, returns `(Output, TempDir)` pair
- `run_vb` (lines 47-51, 5 lines): Backward-compatibility wrapper that calls `run_vb_internal` and extracts only the Output via `.0`, preserving compatibility with all 79 existing tests

This single canonical path is used by all 80 tests (79 legacy + 1 new regression test).

### Regression Test
The new regression test `cli_doctor_with_relative_db_no_source_tree_artifacts` (lines 746-760, 15 lines) is the only test that uses `run_vb_internal` directly. It asserts three properties simultaneously:
1. Doctor command succeeds with relative `--db` name
2. Journal file appears in the child's isolated temp working directory
3. Journal file does NOT appear in the inherited repository cwd

This directly proves the current_dir override is effective and detects its removal.

### New Assertion Helpers
Three small, focused assertion helpers were added within the doctor_command module:
- `assert_vb_success` (lines 773-783, 11 lines)
- `assert_db_created_at` (lines 785-787, 3 lines)
- `assert_db_not_at` (lines 789-791, 3 lines)

All new functions are under the 25-line limit. No duplicate or shadow helpers exist.

### Forbidden Patterns
- No new `panic!`, `todo!`, `unimplemented!`, `dbg!`
- No new `unwrap()` beyond preexisting patterns
- `expect()` count: 1 (in `run_vb` wrapper, line 49)
- No `allow` or `ignore` attributes added
- All new code uses `Result` and `?` for proper error propagation

### DB Name Isolation
The fixed DB name `vb-isolation-v4.db` is unique to this one test target. It is not a global shared fixture and does not create inter-test dependency in normal passing runs.

## Prior Findings Verification

Both findings from the first review are correctly fixed:

1. **BLOCKER (original):** Regression test would pass even with current_dir removed — **FIXED** by using `run_vb_internal` directly so test inspects the actual temp dir the child used.

2. **LOW (original):** Code duplication between `run_vb_internal` and `run_vb_doctor_in_temp` — **FIXED** by eliminating the duplicate helper entirely; single shared invocation path for all tests.

The v2 review's conclusion that both findings were fixed is correct. The v2 review's source hash (ddeff8f7) predates reviewer formatting and is superseded.

## Dispositions

| Finding | Disposition |
|---------|-------------|
| Test doesn't detect current_dir removal (BLOCKER) | fixed_with_evidence |
| Duplicate helper code (LOW) | fixed_with_evidence |
| rustfmt signature format (LOW from v2) | fixed_with_evidence (formatting now canonical) |

No new open findings. No open LOW/INFO findings with APPROVED status.

## Conclusion

Source is byte-identical to expected hash throughout review. All 80 tests pass. Format check passes. The single canonical invocation path with per-invocation TempDir and absolute binary resolution correctly prevents DB artifacts from leaking into the source tree.

**STATUS: APPROVED**