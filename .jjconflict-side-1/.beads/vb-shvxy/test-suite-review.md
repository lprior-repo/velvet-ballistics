# Test Suite Review: vb-shvxy (State 10)

- **Bead**: vb-shvxy
- **Review State**: 10 (test-reviewer)

reviewer_skill: test-reviewer
reviewer_invocation_id: vb-shvxy-state10-test-reviewer-attempt1

**STATUS: REJECTED**

Full review: See `test-review.md` for complete findings, mutation analysis, and resolution guidance.

## Summary

The test suite contains 51 bash tests across 9 files plus 4 fuzz targets. Three blocker-class findings prevent approval:

1. **FIND-TR-001**: 9 tests are structural source-grep checks that survive deletion of target behavior (mutation kill rate 70%, below 90% threshold)
2. **FIND-TR-002**: B020 failure propagation path is untested (only structural grep)
3. **FIND-TR-003**: E2E tests are thin static existence checks, not pipeline execution tests

Additionally: 1 fuzz target uses `.unwrap()` violating project lint rules (FIND-TR-004), 1 test has inconsistent SKIP behavior with its sibling (FIND-TR-005), and 1 test suppresses diagnostics on failure (FIND-TR-006).

The 28 behavioral tests (out of 37 integration tests) have strong assertions: exact exit codes, substring matching on stderr, non-vacuous count checks, and JSON validity validation. The 6 proptest invariants exercise multi-input classifications correctly. The 4 fuzz targets are properly registered and structurally sound (minus the `.unwrap()` in loom_list_xtask).

Resolution requires rewriting the 9 structural-only tests as behavioral tests that execute scripts and assert outcomes, adding a real failure-propagation test for flux, and strengthening the E2E layer.
