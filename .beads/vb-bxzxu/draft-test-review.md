# Draft Test Review: scripts/check-test-integrity.rs --self-test

**Reviewer:** LOCAL3090 (independent)
**Bead:** vb-bxzxu
**Status:** REJECT FOR DRAFT (test coverage gaps)
**Date:** 2026-10-22

## Evidence

- Ran: `/tmp/check-test-integrity --self-test`
- Result: exit 0, all 23 self-test cases pass
- Output captured: `.beads/vb-bxzxu/evidence/draft-selftest.txt`

## Self-Test Cases (23 total)

### Core diff tests (11)
1. `delete` - workspace test deletion → DeletedTestFile + DeletedTestDeclaration + WeakenedAssertion ✓
2. `ignore` - added `#[ignore]` → IgnoredOrSkippedTest ✓
3. `weaken` - assert_eq → is_some() → WeakenedAssertion ✓
4. `strengthen` - added extra assert → pass ✓
5. `silent-return` - added `return;` → IgnoredOrSkippedTest ✓
6. `module-rs-delete` - deleted `src/action/tests.rs` → 3 findings ✓
7. `module-dir-weaken` - weakened test in `src/action/tests/` → WeakenedAssertion ✓
8. `rename-out` - renamed test to archive → 3 findings ✓
9. `non-utf8-relevant` - NUL byte in test file → NonUtf8TestFile ✓
10. `unrelated-binary-passes` - binary evidence file → pass ✓
11. `binary-with-test-delete` - binary + test deletion → 3 findings ✓

### Justification mechanism tests (12)
12. `justify-pre` - sanity: fixture yields WeakenedAssertion ✓
13. `justify-valid` - valid record → exit 0 ✓
14. `justify-stale-test` - stale current blob → exit 1 ✓
15. `justify-stale-base` - stale base blob → exit 1 ✓
16. `justify-stale-replacement` - stale replacement blob → exit 1 ✓
17. `justify-stale-review` - stale review artifact → exit 1 ✓
18. `justify-unknown-base` - wrong base commit → exit 2 ✓
19. `justify-unknown-path` - unknown path → exit 2 ✓
20. `justify-forbidden-nonutf8` - NonUtf8TestFile justified → exit 2 ✓
21. `justify-malformed` - wrong column count → exit 2 ✓
22. `justify-duplicate` - duplicate identity → exit 2 ✓
23. `justify-forbidden-skip` - code-level skip without replacement → exit 2 ✓

## Test Coverage Gaps

### Missing: `has_compile_only` false positive regression test

No test case verifies that a test named with "no_run" as substring (e.g., `test_no_runtime_defaults`) is NOT flagged as `CompileOnlyReplacement`. The bug found in code review (substring matching) is not caught by any self-test.

**Recommended test:** Add a test that creates a test function named `test_no_runtime_defaults()` with real assertions, and verifies no `CompileOnlyReplacement` finding is produced.

### Missing: `has_ignore_or_skip` comment false positive regression test

No test case verifies that a comment containing "skip" or "skips" is NOT flagged as `IgnoredOrSkippedTest`. The bug found in code review is not caught.

**Recommended test:** Add a test that adds a comment `// this test skips nothing` to a test and verifies no `IgnoredOrSkippedTest` finding is produced.

### Missing: `is_test_path` path component regression test

No test case verifies that a path like `crates/foo/workspace_tests_backup/bar.rs` is NOT treated as a test path.

**Recommended test:** Add a test that modifies a non-test file with a name containing "workspace_tests" as a substring and verifies no test-related findings are produced.

### Missing: JJ fallback tests

All self-tests use `Vcs::Git`. No tests exercise the `Vcs::Jj` code paths (changed_files, path_diff_bytes, base_file_bytes). The JJ fallback is untested.

**Recommended test:** Add JJ-specific tests using temp JJ repositories.

### Missing: Edge case tests

- Empty diff (no changes) → should pass
- Binary file renamed (not a test) → should pass
- Test file with only whitespace changes → should pass
- Test file with only comment changes → should pass (if comments are not flagged)
- Multiple findings in single file → all should be reported
- Justification with multiple replacement paths → all must match

### Positive observations

- Self-tests use exact finding matching (kind+path), not just exit codes
- Setup failures propagate (no `.ok()` masking)
- Real temp git commits via ELF-resolved git binary (bypasses guard wrapper)
- Justification tests cover all failure modes (stale, malformed, forbidden, duplicate, unknown)
- Tests run in isolation (separate temp dirs)

## Conclusion

**REJECT FOR DRAFT.** While all 23 self-tests pass, the test suite has coverage gaps that allow the three bugs found in code review to slip through. The `has_compile_only` and `has_ignore_or_skip` substring matching bugs are not caught because there are no regression tests for these edge cases.

The justification mechanism tests are thorough and cover all specified failure modes. The core diff tests are solid. The gaps are in the signal-detection heuristic tests and JJ fallback tests.

## Recommended Next Steps

1. Add regression tests for the three bugs found in code review
2. Add JJ fallback tests
3. Add edge case tests for empty diff, binary renames, whitespace-only changes
4. Re-run self-tests after adding new cases
