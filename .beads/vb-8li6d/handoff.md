# vb-8li6d Handoff: CLI Test Working Directory Isolation

## Status
Implementation v2 complete. All 80 CLI lifecycle behavior tests pass. Ready for independent review.

## Implementation History
- v1 (fd86403e): Rejected — regression test used duplicated `run_vb_doctor_in_temp` helper instead of shared `run_vb_internal`; mutation removing `current_dir` from shared helper passed because regression test did not exercise the shared helper.
- v2 (this repair): Fixed by making `run_vb_internal` return `(Output, TempDir)` so regression test uses the SAME shared helper as all 79 other CLI tests. No duplicated helper.

## Final Source
- Path: `crates/workspace_tests/tests/vb_test_cli_run_lifecycle_behavior.rs`
- SHA256: `65fd7a16b5b6f9708258f16b1acea9f7fa34495c37d96602a8df759a908ffc4c`
- Pre-mutation hash: `949329ec3b3ff0305eea7ac4713ee08c92cead5d0417d988cc15a84a6fcead4d`
- Post-mutation hash: `35ee0bfa95daace2d4bfc45392f9ddad38f2044378477d9db3e3cc2ecba940f4`
- Restored to pre-mutation hash after mutation test

## Changes Made (v2)

### 1. Refactored `run_vb_internal` to return owned TempDir
```rust
fn run_vb_internal(args: &[&str]) -> Result<(Output, TempDir), Box<dyn std::error::Error>> {
    let binary = vb_binary();
    let absolute_binary = std::fs::canonicalize(&binary)?;
    let tmp_dir = TempDir::new()?;
    let mut cmd = Command::new(absolute_binary);
    cmd.current_dir(tmp_dir.path());
    cmd.args(args);
    let output = cmd.output()?;
    Ok((output, tmp_dir))
}
```
Function is 15 lines. Owns all canonicalize/TempDir/Command/current_dir/output operations. TempDir lifetime covers the entire child process execution.

### 2. Updated `run_vb` wrapper
```rust
fn run_vb(args: &[&str]) -> Output {
    run_vb_internal(args)
        .expect("failed to execute velvet-ballistics")
        .0
}
```
One preexisting expect for the total operation. Extracts Output from tuple. Kept alive until process done via returned TempDir.

### 3. Removed duplicated `run_vb_doctor_in_temp` helper
No separate doctor helper. No extra abstraction/boolean flag/test-only duplicate seam.

### 4. Updated regression test to use shared helper
```rust
fn cli_doctor_with_relative_db_no_source_tree_artifacts()
-> Result<(), Box<dyn std::error::Error>> {
    let db_name = "vb-isolation-test.db";
    let (output, tmp_dir) = run_vb_internal(&["doctor", "--db", db_name])?;
    // ... assertions using returned tmp_dir and actual std::env::current_dir()? inherited source location
}
```
Invokes the SAME helper used by all 79 other CLI tests. Inspects returned TempDir and actual inherited source location (NOT manifest.parent). Unique DB name per test to avoid collision.

## Forbidden Patterns
- No new `panic!`, `unwrap()`, `todo!`, `dbg!`, or unchecked indexing
- Only one `expect()` remains: in `run_vb()` for the total operation
- No global `chdir`; each CLI invocation gets a fresh `TempDir`
- No `unsafe`, `allow`, `ignore`
- No production code changes

## Test Results
**80 tests passed, 0 failed, 0 ignored, 0 measured** in 0.02s.

Evidence logs in `.beads/vb-8li6d/evidence/`:
- `FULL-TEST-SUITE.{stdout,stderr,json}` — First full run after repair
- `FINAL-TEST-SUITE.{stdout,stderr,json}` — Final run after formatting fixes
- `RUSTFMT-CHECK-FINAL.{stdout,stderr,json}` — rustfmt --check passes (exit 0)

## Mutation Verification

### Mutation 1: Remove `current_dir` from shared `run_vb_internal`
- Command: `python3 /tmp/vb-run-gate.py 120 .beads/vb-8li6d/evidence/MUTATION-REGRESSION-TEST cargo test --manifest-path crates/workspace_tests/Cargo.toml --test vb_test_cli_run_lifecycle_behavior doctor_command::cli_doctor_with_relative_db_no_source_tree_artifacts`
- Result: Exit code 101, test FAILED (1 failed, 0 passed)
- Test failure: "doctor should create Fjall journal at relative path in child cwd: /tmp/.tmpXXX/vb-isolation-test.db"
- Evidence: `.beads/vb-8li6d/evidence/MUTATION-REGRESSION-TEST.{stdout,stderr,json}`
- Conclusion: Mutation resistance confirmed — removing `current_dir` from shared helper breaks regression test

## Verification Evidence
- rustfmt check: exit 0 on final source
- All 80 tests pass through official runner
- Mutation test confirms regression test exercises the shared helper's `current_dir` call
- No stray DB artifacts left in repo source tree

## What This Test Proves
The regression test proves three things simultaneously:
1. `doctor --db <relative-name>` is accepted and succeeds with exit code 0
2. The relative path is resolved within the child's working directory (temp dir)
3. No Fjall journal artifacts leak into the inherited repository cwd
If the `current_dir` override were removed from `run_vb_internal`, the Fjall journal would be created in the repo root, and the test would fail — as verified by mutation testing.

## Notes for Reviewer
- Independent review candidate fd86403e was rejected (see `implementation-handoff.md` for superseded historical notes)
- This v2 repair addresses both rejection findings: removed duplicated helper, regression test now uses shared helper
- Reviewer should verify mutation resistance by re-running the mutation test with a different mutation (e.g., changing `current_dir` path)
