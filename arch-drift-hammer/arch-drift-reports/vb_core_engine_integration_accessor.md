# Architectural Drift Report: `vb_core_engine_integration_accessor.rs`

## File Summary

| Field | Value |
|-------|-------|
| **Path** | `crates/vb_core/src/engine/tests/integration_accessor.rs` |
| **Total Lines** | 1564 |
| **Total Tests** | 61 (`#[test]` functions) |
| **Location Category** | `engine/tests/` — Integration test module for accessor evaluation |
| **Size Status** | ❌ **VIOLATION** — Exceeds 300-line threshold by 5.2× |

## Analysis

### Size Violation
The file is **1564 lines**, far exceeding the mandated **< 300 lines** per file rule from the architectural-drift skill.

### Test Distribution
- **56 standard `#[test]` functions** covering:
  - Root value loading (with/without store)
  - Object field traversal
  - List index access
  - Nested path traversal (field + index combinations)
  - Error cases (null/scalar field/index traversal, OOB, missing fields)
  - Determinism verification
  - Edge cases (empty lists, max depth chains, u32::MAX reserved index)
- **5 `proptest!` driven property tests** inside `#[cfg(test)] mod proptests`

### Structural Observations
1. **Helper functions** (`test_store`, `ensure_equal`, `test_frame`, `accessor_workflow`, `accessor_workflow_with_opts`) occupy lines 22–314 — these could be extracted to a shared test-fixture module
2. **Test organization by comment sections** (lines 317, 427, 507, 530, 648, 714, 776, 853, 908, 1001, 1063, 1127, 1213, 1263, 1453) shows clear thematic groupings that map to separate files
3. **Proptests** are already isolated in a `mod proptests` block but cannot be split further due to the monolithic file

## Recommendation

**SPLIT REQUIRED** — Refactor into at minimum 5 files:

| Split File | Content | Est. Lines |
|------------|---------|------------|
| `accessor_basic.rs` | Root loading, scalar types, identity path | ~300 |
| `accessor_traversal.rs` | Field/index path operations, nested traversal | ~350 |
| `accessor_errors.rs` | All error case tests (OOB, not found, type errors) | ~400 |
| `accessor_determinism.rs` | Determinism, repeatability, edge case bounds | ~250 |
| `accessor_proptests.rs` | All proptest property tests | ~200 |

**Rationale:**
- Each resulting file stays well under the 300-line threshold
- Mirrors the existing comment-section groupings in the source
- Allows parallel test execution improvement
- Aligns with DDD cohesion principle: each test module tests one behavior surface

**Status:** `REFACTOR REQUIRED`
