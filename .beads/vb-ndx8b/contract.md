# VB-NDX8B: Fix UTF-8 handling in test integrity checker

## Problem
`scripts/check-test-integrity.rs` currently converts the entire git/jj diff output to UTF-8 using strict `String::from_utf8` in `tool_output()`. When the working tree diff includes binary or non-UTF-8 files (such as `.beads/evidence/` binary proof artifacts), the checker fails with a "non-UTF8 stdout" error before it can scan for test changes.

## Fix
Restructure `diff_text()` and `scan_diff()` to:
1. Obtain the diff as raw bytes from the VCS command
2. Split the diff into per-file hunks at the `diff --git` / `+++ b/` boundaries
3. Parse the old and new paths from each hunk's header lines
4. Convert each hunk to UTF-8 **only if** it involves a test path (as determined by the existing `is_test_path()` classifier)
5. If a test-path hunk contains non-UTF-8 bytes, treat it as a real error (fail closed)
6. If a non-test-path hunk contains non-UTF-8 bytes, silently skip it

This preserves the existing behavior for:
- Detecting deleted test files across all VCS-supported paths
- Detecting weakened assertions, removed test declarations, ignored/skipped tests, compile-only replacements
- Handling renames (both git and jj)
- Handling unit tests in `src/tests.rs`, `src/tests/`, `tests/`, `examples/`, `benches/`

## Test Regressions
Add self-test cases to the existing `--self-test` infrastructure:
1. Unrelated non-UTF-8 evidence file with unchanged test passes
2. Actual Rust test deletion still fails despite unrelated binary evidence
3. Weakened assertion still fails despite unrelated binary evidence
4. Invalid UTF-8 in a relevant Rust test file fails closed
5. Git and JJ rename behavior preserved where existing tests cover
