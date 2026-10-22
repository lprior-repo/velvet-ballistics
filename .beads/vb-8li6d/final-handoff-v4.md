# vb-8li6d Handoff v4: CLI Test Working Directory Isolation (Repaired)

## Status
Implementation v4 complete. All 80 CLI lifecycle behavior tests pass. Ready for independent review.

## Implementation History
- v1 (fd86403e): Rejected — regression test used duplicated `run_vb_doctor_in_temp` helper instead of shared `run_vb_internal`.
- v2: Fixed duplicated helper; mutation test passed. However, two bugs remained in the regression test.
- v3: Previous repair attempt.
- v4 (this repair): Fixed both bugs in the regression test:
  1. `repo_root` used `manifest.parent()` (wrong cwd); corrected to `std::env::current_dir()?` (actual inherited cwd)
  2. Test was over 25 lines (rustfmt failure); extracted concise assertion helpers

## Final Source
- Path: `crates/workspace_tests/tests/vb_test_cli_run_lifecycle_behavior.rs`
- SHA256: `ddeff8f7afc6253f8da0302882ee90a43cd37304f8fd65d6addbe14856762d39`
- Pre-mutation hash: `ddeff8f7afc6253f8da0302882ee90a43cd37304f8fd65d6addbe14856762d39`
- Post-mutation hash: `35ee0bfa95daace2d4bfc45392f9ddad38f2044378477d9db3e3cc2ecba940f4`
- Restored to pre-mutation hash byte-for-byte after mutation test

## Changes Made (v4)

### 1. Fixed cwd calculation in regression test
```rust
// BEFORE (wrong)
let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .ok_or("manifest dir has no parent")?;
let repo_db_path = repo_root.join(db_name);

// AFTER (correct)
assert_db_not_at(&std::env::current_dir()?.join(db_name));
```
The child process inherits `std::env::current_dir()` as its working directory, not the manifest's parent. This is the actual inherited source location.

### 2. Extracted assertion helpers for conciseness
```rust
fn assert_vb_success(output: &Output, expect_stdout: fn(&str) -> bool) {
    assert!(output.status.success(),
        "exit code: {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(expect_stdout(&stdout), "unexpected output: {}", stdout);
}

fn assert_db_created_at(path: &std::path::Path) {
    assert!(path.exists(), "DB should exist at {:?}", path);
}

fn assert_db_not_at(path: &std::path::Path) {
    assert!(!path.exists(), "DB should not exist at {:?}", path);
}
```
All three helpers are under 25 lines. The regression test itself is now 14 lines.

### 3. Unique DB name
```rust
let db_name = "vb-isolation-v4.db";
```
Prevents collision with old mutation artifacts.

### 4. Kept shared invocation helper
`run_vb_internal` remains the ONE shared invocation helper used by all 80 CLI tests. No duplicate invocation helper was introduced.

## Forbidden Patterns
- No new `panic!`, `unwrap()`, `todo!`, `dbg!`, or unchecked indexing
- Only one `expect()` remains: in `run_vb()` for the total operation
- No global `chdir`; each CLI invocation gets a fresh `TempDir`
- No `unsafe`, `allow`, `ignore`
- No production code changes

## Test Results
**80 tests passed, 0 failed, 0 ignored, 0 measured** in 0.02s.

Evidence logs in `.beads/vb-8li6d/evidence/`:
- `v4-rustfmt.json` — rustfmt applied successfully (exit 0)
- `v4-rustfmt-check.json` — rustfmt --check passes (exit 0)
- `v4-mutation-build.json` — test compiled with mutation (exit 0)
- `v4-mutation-test.json` — regression test failed with mutation (exit 101)
- `v4-all-tests-final.json` — all 80 tests pass after restoration (exit 0)

## Mutation Verification

### Mutation: Remove `current_dir` from shared `run_vb_internal`
- Command: `python3 /tmp/vb-run-gate.py 300 .beads/vb-8li6d/evidence/v4-mutation-test cargo test -p velvet-ballistics-workspace-tests --test vb_test_cli_run_lifecycle_behavior cli_doctor_with_relative_db_no_source_tree_artifacts`
- Result: Exit code 101, test FAILED (1 failed, 0 passed)
- Test failure: "DB should exist at /tmp/.tmpFLkd4Y/vb-isolation-v4.db" — DB was created in inherited cwd, not the temp dir
- Evidence: `.beads/vb-8li6d/evidence/v4-mutation-test.{stdout,stderr,json}`
- Conclusion: Mutation resistance confirmed — removing `current_dir` from shared helper breaks regression test

### Restoration Verification
- Source restored byte-for-byte: `ddeff8f7afc6253f8da0302882ee90a43cd37304f8fd65d6addbe14856762d39`
- All 80 tests pass after restoration: exit 0
- Mutation DB preserved outside repository: `/tmp/vb-mutation-db-preserved.db`

## Verification Evidence
- rustfmt applied and checked: exit 0
- All 80 tests pass through official runner
- Mutation test confirms regression test exercises the shared helper's `current_dir` call
- Source hash verified identical before/after mutation testing
- No stray DB artifacts left in repo source tree

## What This Test Proves
The regression test proves three things simultaneously:
1. `doctor --db <relative-name>` is accepted and succeeds with exit code 0
2. The relative path is resolved within the child's working directory (temp dir)
3. No Fjall journal artifacts leak into the inherited repository cwd (`std::env::current_dir()`)
If the `current_dir` override were removed from `run_vb_internal`, the Fjall journal would be created in the inherited cwd, and the test would fail — as verified by mutation testing.

## Notes for Reviewer
- This v4 repair addresses both bugs from the previous version: wrong cwd calculation and test length
- The `std::env::current_dir()?` approach correctly identifies the actual inherited working directory
- The assertion helpers reduce the test to 14 lines while preserving all meaningful checks
- Reviewer should verify mutation resistance by re-running the mutation test with a different mutation (e.g., changing `current_dir` path)