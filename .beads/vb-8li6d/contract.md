# vb-8li6d: CLI Test Working Directory Isolation

## Problem

`crates/workspace_tests/tests/vb_test_cli_run_lifecycle_behavior.rs` invokes the
velvet-ballistics CLI via `run_vb()` without setting an isolated working
directory. Commands that create database files (SQLite/Fjall journals) write
to the repo's `crates/workspace_tests/tests/` directory, contaminating the
source tree with `journal-db`, `nonexistent-db`, and related artifacts.
This also invalidates benchmark freshness checks.

## Solution

Modify `run_vb()` to use a per-invocation `TempDir` (from the existing
`tempfile` dependency) as the command's working directory. Each CLI invocation
gets a fresh, isolated cwd; the temp directory drops and cleans up after the
process completes.

## Acceptance Criteria

- `run_vb()` creates a `TempDir` per invocation and sets it as `current_dir`
- Binary path is resolved to an absolute path before invocation
- Process is fully awaited before temp directory cleanup
- No global `chdir` calls
- All existing behavioral assertions unchanged
- No new `unwrap`/`expect`/`panic`/`unsafe`/`ignore`/`allow` beyond existing
- No production code changes
- Regression test: `doctor --db <relative>` in isolated cwd produces no
  Fjall artifacts in the repo source tree

## Regression Test

Add a test that invokes `velvet-ballistics doctor --db test.db` in an
isolated temp directory and asserts:
1. Process runs and produces output (exit code may be non-zero for missing db)
2. No `test.db` or Fjall journal files appear in the repo's test directory

## Files Modified

- `crates/workspace_tests/tests/vb_test_cli_run_lifecycle_behavior.rs`
  - Update `run_vb()` helper with TempDir isolation
  - Add regression test for cwd isolation
