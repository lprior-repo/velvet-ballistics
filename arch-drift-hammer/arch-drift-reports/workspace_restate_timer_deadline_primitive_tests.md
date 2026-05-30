# Architectural Drift Report: `restate_timer_deadline_primitive_tests.rs`

## File Summary

| Metric | Value |
|--------|-------|
| **File Path** | `crates/workspace_tests/tests/restate_timer_deadline_primitive_tests.rs` |
| **Total Lines** | 1,902 |
| **File Size** | 70,870 bytes (~69 KB) |
| **Test Count** | 141 `#[test]` functions |
| **Modules** | 11 sections organized by invariant category |
| **Lines per Test (avg)** | ~13.5 lines/test |

## Section Breakdown

| Section | Tests | Focus |
|---------|-------|-------|
| 1 | 13 | `collect_expired_keys` purity via `fire_expired` |
| 2 | 16 | `compute_delay` loop bound invariants |
| 3 | 12 | `fast_forward_cursor` bounds |
| 4 | 24 | TimerWheel documented invariants (5 categories) |
| 5 | 14 | RetryState invariants |
| 6 | 14 | `evaluate_retry` invariants |
| 7 | 8 | `is_failure_retriable` purity |
| 8 | 11 | `RetryPolicy` construction and invariants |
| 9 | 4 | `exhaustion_error` |
| 10 | 14 | `RetryPolicyLimits` validation |
| 11 | 11 | Value semantics (DelayStrategy, RetryPolicyError, etc.) |

## Location Category

**`workspace_tests`** — This file resides in `crates/workspace_tests/tests/`, indicating it is a cross-crate integration/primitive-level test file that exercises public APIs across `vb_core` and `vb_runtime` boundaries.

## Architectural Observations

1. **File Size**: At 1,902 lines, this file significantly exceeds the 300-line threshold specified in architectural drift rules. This is 6.3× over the recommended maximum file size.

2. **Cohesion**: The file is well-organized into 11 thematic modules, each testing a specific invariant set. However, the sheer volume suggests these could be split into separate test files per module.

3. **Test Quality**: Tests are exhaustive, use exact assertions (no `is_ok()`/`is_err()` shortcuts), and document invariant categories. This is exemplary.

4. **Boundary Compliance**: Tests correctly span `vb_core` and `vb_runtime` via public API only — no internal APIs violated.

## Recommendation

**REFACTOR REQUIRED** — Split into 3–4 topic-specific test files:

| Suggested File | Sections | Est. Lines |
|----------------|----------|------------|
| `timer_wheel_tests.rs` | Sections 1, 4 | ~700 lines |
| `retry_policy_tests.rs` | Sections 2, 5, 6, 7, 8, 9, 10, 11 | ~900 lines |
| `retry_cursor_tests.rs` | Section 3 | ~300 lines |

**Rationale**: While the current organization is logically coherent, the 1,902-line monolithic test file creates:
- Cognitive overload for reviewers
- Slower CI compilation if changed
- Risk of merge conflicts in multi-developer workflows

The test content itself is architecturally sound — only the file decomposition needs correction.

## Verdict

| Check | Status |
|-------|--------|
| File size < 300 lines | ❌ FAIL (1,902 lines) |
| DDD cohesion (single domain concept per file) | ⚠️ MARGINAL (well-modularized but too large) |
| Boundary integrity | ✅ PASS |
| Test quality | ✅ PASS |
| No `unsafe`/`unwrap`/`panic` | ✅ PASS |

**Overall**: Architectural drift detected — file must be decomposed.
