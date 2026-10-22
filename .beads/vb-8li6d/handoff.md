# vb-8li6d Handoff: CLI Test Working Directory Isolation

## Summary

Fixed `crates/workspace_tests/tests/vb_test_cli_run_lifecycle_behavior.rs` to use
per-invocation isolated temp working directories, preventing Fjall/SQLite
database files from being written into the repository source tree during tests.

## Changes Made

### 1. Modified `run_vb()` Helper (lines 27-45)

The helper now:
- Resolves the binary path to an absolute path via `std::fs::canonicalize`
- Creates a `TempDir` (from existing `tempfile` dependency) per invocation
- Sets `current_dir` to the temp directory
- Awaits the process output
- Drops `TempDir` after process completes, cleaning up all artifacts

No global `chdir` calls. No new `unwrap`/`expect`/`panic`/`unsafe`/`ignore`/`allow`
beyond existing behavior (`.expect()` on `TempDir::new()` is reasonable — if
the OS can't allocate temp space, test execution is impossible).

### 2. Added Regression Test

`doctor_command::cli_doctor_with_relative_db_no_source_tree_artifacts` verifies:
- `doctor --db test-isolation.db` runs and produces output
- No `test-isolation.db` or Fjall journal files (`.db-journal`, `.db-wal`, `.db-shm`, etc.)
  appear in the repo's test directory

### 3. Added Import

`use tempfile::TempDir;` — `tempfile` was already a dependency of
`velvet-ballistics-workspace-tests` (line 21 of Cargo.toml).

## Test Results

**80 tests passed, 0 failed, 0 ignored, 0 measured** in 0.02s.

Evidence logs in `.beads/vb-8li6d/evidence/`:
- `vb-8li6d-20261022-073031-stdout.log` — First full test run
- `vb-8li6d-20261022-073053-stdout.log` — Final clean run after lint fixes

## Verification

- No stray `test-isolation.db` or Fjall artifacts in `crates/workspace_tests/tests/`
- `cargo check` passes with zero warnings
- All existing behavioral assertions unchanged
- New regression test validates isolation property

## Root VCS Actions Needed

```bash
cd /home/lewis/src/velvet-ballistics
jj git fetch
jj workspace add vb-8li6d-local5090-impl --path ~/src/isoloated/velvet-ballistics-vb-8li6d-local5090-impl
cd ~/src/isoloated/velvet-ballistics-vb-8li6d-local5090-impl
jj describe -m "vb-8li6d: Isolate CLI test working directories"
jj commit
jj new vb-8li6d-review
```

## Files Modified

- `.beads/vb-8li6d/contract.md` — Rust contract
- `crates/workspace_tests/tests/vb_test_cli_run_lifecycle_behavior.rs` — Implementation + regression test
