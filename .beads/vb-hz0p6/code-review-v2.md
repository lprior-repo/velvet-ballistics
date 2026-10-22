# Code Review v2: vb-hz0p6 CLI Isolation (Re-Review)

**Reviewer:** LOCAL5090 independent review (fresh agent)
**Author source:** 9531760cf607cd634f7b82c0916ae9f371d88538 (merged into workspace)
**Base:** d5f389344 (origin/main)
**Reviewed workspace SHA:** cbd2d2101 (working merge includes author's fix)
**Reviewed source hash:** ddeff8f7afc6253f8da0302882ee90a43cd37304f8fd65d6addbe14856762d39
**Scope:** crates/workspace_tests/tests/vb_test_cli_run_lifecycle_behavior.rs only

## Original Rejection Findings

### 1. BLOCKER: Regression test does not detect removal of current_dir from shared run_vb_internal
**Finding (original):** Regression test `cli_doctor_with_relative_db_no_source_tree_artifacts` used a duplicated `run_vb_doctor_in_temp` helper. Mutation removing `cmd.current_dir()` from shared `run_vb_internal` passed the test.
**Fixed with evidence:**
- Source now has NO `run_vb_doctor_in_temp`; regression test calls `run_vb_internal` directly (line 750)
- Independent mutation test performed: removed `cmd.current_dir(tmp_dir.path());` from `run_vb_internal` (line 37)
- Result: test FAILED with "DB should exist at /tmp/.tmpXXXX/vb-isolation-v4.db" — DB created in inherited cwd instead
- Source restored byte-for-byte (verified SHA matches ddeff8f7)
- **Status: FIXED — verified by independent mutation test**

### 2. LOW: Code duplication between run_vb_internal and run_vb_doctor_in_temp
**Finding (original):** 10 of 14 lines identical between two helpers.
**Fixed with evidence:**
- `run_vb_doctor_in_temp` no longer exists
- `run_vb_internal` returns `(Output, TempDir)` pair; `run_vb` wrapper extracts output via single `.expect()`
- Single shared invocation path for all 80 CLI tests
- **Status: FIXED — duplicate helper eliminated**

## Independent Verification Results

### Mutation Test (independent)
```
Mutation: CUT line 37 (cmd.current_dir(tmp_dir.path());)
Test run: cargo test ... cli_doctor_with_relative_db_no_source_tree_artifacts
Result: FAILED (exit 101) — "DB should exist at /tmp/.tmpXXXX/vb-isolation-v4.db"
Restoration: Verified SHA ddeff8f7 matches after edit
```

### Full Test Suite
```
cargo test -p velvet-ballistics-workspace-tests --test vb_test_cli_run_lifecycle_behavior
Result: 80 passed; 0 failed; 0 ignored; 0 measured (finished in 0.01s)
Note: 1 leftover mutation artifact DB cleaned from crates/workspace_tests/vb-isolation-v4.db
```

### Rustfmt Check
```
cargo fmt -p velvet-ballistics-workspace-tests --check
Result: Clean (no output = no diffs)
```

### Forbidden Patterns
- No new `unsafe`, `panic!`, `todo!`, `unimplemented!`, `dbg!`
- No new `unwrap()`
- `expect()` count: 1 (in `run_vb` wrapper — preexisting)
- No `allow`, `ignore` attributes added
- All new code uses `Result` and `?` for error propagation

### Design Review
- `run_vb_internal`: 12 lines, canonicalizes binary to absolute path, creates fresh TempDir per invocation, sets current_dir, awaits output, returns (Output, TempDir)
- `run_vb`: 4-line wrapper for backward compatibility with 79 existing tests
- `assert_vb_success`, `assert_db_created_at`, `assert_db_not_at`: 3 small assertion helpers (under 8 lines each)
- Regression test: 14 lines (excluding doc comment), tests 3 properties simultaneously (success, temp dir DB creation, inherited cwd clean)

## New Findings

### 1. LOW: rustfmt function signature format
The regression test function signature has non-canonical formatting (closing paren on separate line). rustfmt reformats it to single-line. Applied and verified. Not a functional issue.

### 2. INFO: Mutation test cleanup responsibility
The mutation test run creates a DB artifact in the inherited cwd. Author documented preservation strategy; reviewer verified cleanup. Suggestion: add a `defer` or explicit cleanup in mutation testing workflow to prevent this class of flakiness. Not a blocker for this bead.

## Conclusion

Both original findings are genuinely fixed and independently verified. The regression test now uses the shared helper directly, and mutation testing confirms it detects the specific regression the fix addresses.

**STATUS: APPROVED**