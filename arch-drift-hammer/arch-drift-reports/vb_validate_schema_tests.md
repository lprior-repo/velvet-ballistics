# Architectural Drift Report: `schema_tests.rs`

**File:** `crates/vb_validate/src/schema_tests.rs`  
**Analyzed:** 2026-05-29

---

## Summary

| Metric | Value |
|--------|-------|
| **Total Lines** | 1490 |
| **Total Tests** | 93 |
| **Location Category** | `crates/vb_validate/src/` (production crate, unit test module) |
| **Size Threshold** | ⚠️ EXCEEDS 300-line guideline by 5× |

---

## File Structure Analysis

### Test Organization (5 Sections)

| Section | Lines | Tests | Purpose |
|---------|-------|-------|---------|
| Helper functions | 1–19 | — | `make_workflow`, `make_step`, `valid_workflow_doc` |
| Schema validation tests | 42–190 | 16 | Version, trigger, ID validation |
| BDD exact-assertion tests | 195–800 | 27 | Schema contract assertions |
| Accessor/query tests | 806–1006 | 23 | `get_string`, `get_mapping`, `get_sequence` accessors |
| Adversarial BDD tests | 1012–1490 | 44 | Validation bypass attack surface |

---

## Architectural Drift Findings

### 🚨 DRIFT: File Size Violation

**Rule:** Files should not exceed 300 lines.  
**Actual:** 1490 lines (497% of threshold)

**Impact:**
- Cognitive overload for reviewers
- Violates Scott Wlaschin DDD cohesion principle (one concept per file)
- Harder to parallelize review across teams

### ✅ NO DRIFT: Test Placement

- File lives in `crates/vb_validate/src/` — production crate ✓
- Unit tests co-located with source (`schema_tests.rs` alongside schema modules) ✓
- `workspace_tests/` remains clean for cross-crate integration tests ✓

### ✅ NO DRIFT: Naming Convention

- Follows `*_tests.rs` pattern for test modules ✓
- Uses canonical `velvet-ballistics/v1` language version ✓

### ⚠️ DRIFT: Test Count Concentration

- 93 tests in a single file creates monolithic test suite
- Suggests `vb_validate` schema module may have too many responsibilities
- Consider splitting into `schema_validation_tests.rs`, `schema_accessor_tests.rs`, `schema_adversarial_tests.rs`

---

## Recommendation

**REFACTOR (Medium Priority)**

Split `schema_tests.rs` into 3 cohesive test modules:

```
crates/vb_validate/src/
├── schema_validation_tests.rs   (~500 lines, ~30 tests)
├── schema_accessor_tests.rs      (~350 lines, ~23 tests)  
└── schema_adversarial_tests.rs  (~500 lines, ~40 tests)
```

**Rationale:**
- Restores 300-line file size compliance
- Improves test isolation and parallel CI
- Aligns with DDD "one aggregate per file" principle applied to tests
- Enables selective test runs (`cargo test schema_validation`)

---

## Verification Commands

```bash
# Count lines
wc -l crates/vb_validate/src/schema_tests.rs

# Count tests
rg '#\[test\]' crates/vb_validate/src/schema_tests.rs | wc -l

# Verify no cross-crate deps in test file
rg '^use crate::vb_' crates/vb_validate/src/schema_tests.rs
```
