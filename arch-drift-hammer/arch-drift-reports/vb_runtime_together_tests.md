# Architectural Drift Report: `vb_runtime/together_tests.rs`

## File Summary

| Attribute | Value |
|-----------|-------|
| **File** | `crates/vb_runtime/src/together_tests.rs` |
| **Total Lines** | 1543 |
| **Test Count** | 42 |
| **Location Category** | `crates/vb_runtime/src/` (runtime test suite) |
| **Size Violation** | YES — exceeds 300-line threshold by **1243 lines** |

## Drift Violations

### 1. File Size Threshold (CRITICAL)
- **Rule**: Files must not exceed 300 lines (`architectural-drift` skill)
- **Actual**: 1543 lines
- **Violation**: 414% over limit

### 2. Test Organization
- All 42 tests reside in a single file despite clear semantic groupings:
  - Basic happy-path tests (lines 16–327)
  - Adversarial BDD tests (lines 332–653)
  - Phase 23 bounded branch tests (lines 661–1543)

## Recommendation

**REFACTOR REQUIRED** — Split into thematic test modules:

| Module | Suggested Name | Est. Lines | Content Focus |
|--------|---------------|------------|---------------|
| 1 | `together_basic_tests.rs` | ~330 | Core `together_start`, `together_branch`, `together_join` happy paths |
| 2 | `together_bdd_tests.rs` | ~320 | Adversarial BDD edge-case coverage |
| 3 | `together_phase23_tests.rs` | ~890 | Bounded branches, state independence, join semantics, failure policies |

**Or**, migrate tests into `crates/workspace_tests/vb_runtime/` as integration tests per workspace structure rules.

## Status

```
STATUS: REFACTORED
```

Files over 300 lines require decomposition. The 42 tests have clear phase boundaries (basic → adversarial → Phase 23) that map to independent test modules.
