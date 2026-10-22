# Draft Code Review: scripts/check-test-integrity.rs

**Reviewer:** LOCAL3090 (independent)
**Bead:** vb-bxzxu
**Status:** REJECT FOR DRAFT (3 real bugs found)
**Date:** 2026-10-22

## Scope

Reviewed frozen draft at commit efee79de (parent of current isolated workspace). Original source preserved; no edits made. Reviewed per-file byte filtering, selftest infrastructure, and hash-bound justification mechanism per vb-ndx8b contract.

## Evidence

- Compiled binary: `/tmp/check-test-integrity` (rustc --edition=2021)
- Self-test: `./check-test-integrity --self-test` → exit 0 (all 23 cases pass)
- Real-base run: `TEST_INTEGRITY_BASE=da633fe0bbde9e414173d5f7aabe314a37f8752d ./check-test-integrity` → exit 1, 626 findings (vs reference 17; reference predates vb_cli→velvet_ballistics rename)
- Evidence files: `.beads/vb-bxzxu/evidence/draft-selftest.txt`, `.beads/vb-bxzxu/evidence/real-base-run.txt`

## Findings

### Real Bugs (Security/Semantic)

**1. `has_compile_only` substring false positive (line 330-341)**

`has_compile_only` checks `lower.contains("no_run")` which matches any function name or identifier containing "no_run" as a substring, not just the `#[no_run]` attribute.

**Minimal repro:** Test named `parse_status_accepts_no_runtime_defaults()` is flagged as `CompileOnlyReplacement` because "no_runtime" contains "no_run".

**Expected:** No finding (test has real assertions)
**Actual:** `CompileOnlyReplacement|crates/velvet_ballistics/src/args/tests/parse_misc.rs|fn parse_status_accepts_no_runtime_defaults() {`

**Fix:** Match `#[no_run]` as a token, not substring. Same for other compile-only markers.

**2. `has_ignore_or_skip` comment false positives (line 320-328)**

`has_ignore_or_skip` checks `lower.contains(" skip")` which matches any line containing the word "skip" or "skips", including comments.

**Minimal repro:** Comment `// and then submit the artifact under Relaxed policy (which skips checksum` is flagged as `IgnoredOrSkippedTest`.

**Expected:** No finding (comment, not code-level skip)
**Actual:** `IgnoredOrSkippedTest|crates/velvet_ballistics/tests/admission_evidence_integration/chunk_002.rs|// and then submit the artifact under Relaxed policy (which skips checksum`

**Evidence from real-base run:** 5+ comment false positives observed in output.

**Fix:** Strip comments before checking, or match only code-level patterns (`#[ignore]`, `return;` at statement boundary, etc.).

**3. `is_test_path` substring matching (line 228-240)**

Uses `format!("/{path}").contains(part)` for path segment matching. Matches substrings, not path components.

**Minimal repro:** Path `crates/foo/workspace_tests_backup/bar.rs` matches `workspace_tests` segment.

**Expected:** Not a test path (different directory)
**Actual:** Treated as test path

**Impact:** Conservative (false positives only), but could cause noisy findings on non-test files with similar names.

**Fix:** Use path component matching (split on `/` and compare components).

### Not Bugs (Intentional Strictness)

- `has_ignore_or_skip` matching `return;` in tests: intentional. Any `return;` in a test requires justification to suppress. Correct for silent-skip detection.
- `has_ignore_or_skip` matching `skip(`: intentional. Matches `skip()` calls.
- Mass findings (626 vs 17 reference): expected. Reference predates vb_cli→velvet_ballistics rename. Checker correctly detects all deleted test files.

### Positive Observations

- **Per-file byte filtering works:** Non-UTF-8 test file detection passes self-test and correctly identifies NUL/FF bytes.
- **Rename-out detection works:** `run_rename_out_case` self-test passes; deleted test files are correctly flagged.
- **Justification mechanism is hash-bound:** All 11 justification self-tests pass (valid, stale, wrong-base, unknown-path, forbidden-kind, malformed, duplicate, code-level-skip).
- **Fail-closed behavior:** Unknown paths, stale hashes, forbidden kinds all cause exit 2 (error) or exit 1 (finding), never silent pass.
- **No lossy header decode:** Uses strict `std::str::from_utf8` for relevant files; no `from_utf8_lossy`.
- **No unchecked indexing:** Uses `strip_prefix`, `split_once`, and Option handling throughout.
- **Self-tests assert exact findings:** Not just exit codes. `expect_findings` compares kind+path tuples.

## Conclusion

**REJECT FOR DRAFT.** Three real bugs cause false positives that would require unnecessary justification records. The `has_compile_only` and `has_ignore_or_skip` substring matching bugs are particularly problematic because they flag legitimate tests as problematic, defeating the purpose of the checker.

The core architecture (per-file diff, hash-bound justifications, fail-closed behavior, self-test infrastructure) is solid. The bugs are in the signal-detection heuristics, not the core logic.

## Recommended Next Steps

1. Fix `has_compile_only` to match `#[no_run]` as a token
2. Fix `has_ignore_or_skip` to exclude comments and match code-level patterns only
3. Fix `is_test_path` to use path component matching
4. Re-run self-tests and real-base run
5. Compare findings to reference (expect closer match after fixes)
