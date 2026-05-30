# Architectural Drift Report: `vb_test_validate_diagnostic_behavior.rs`

**File:** `crates/workspace_tests/tests/vb_test_validate_diagnostic_behavior.rs`
**Date:** 2026-05-29
**Analyst:** architectural-drift agent

---

## 1. Size Metrics

| Metric | Value |
|--------|-------|
| **Total Lines** | 1537 |
| **Test Count** | 134 |
| **Module Count** | 11 |
| **Blank Lines** | ~85 |
| **Comment Lines** | ~60 |
| **Code Lines** | ~1392 |
| **Lines per Test (avg)** | ~11.5 |

---

## 2. Structural Analysis

### 2.1 File Location Compliance
```
✓ Correct location: crates/workspace_tests/tests/
✓ Follows workspace_tests/ naming convention
✓ Integration test placement (not unit test in crate)
```

### 2.2 Module Organization

| Module | Tests | Lines | Purpose |
|--------|-------|-------|---------|
| `schema_error_codes` | 11 | ~78 | E01xx diagnostic code mapping |
| `reference_error_codes` | 4 | ~30 | E02xx diagnostic code mapping |
| `control_flow_error_codes` | 8 | ~56 | E03xx diagnostic code mapping |
| `type_taint_error_codes` | 12 | ~84 | E04xx diagnostic code mapping |
| `gate_error_codes` | 19 | ~188 | E05xx diagnostic code mapping |
| `contract_discovery_error_codes` | 3 | ~28 | E06xx diagnostic code mapping |
| `error_message_formatting` | 52 | ~576 | Exact message format assertions |
| `diagnostic_structure_invariants` | 5 | ~223 | Cross-variant structural invariants |
| `diagnostic_code_display` | 3 | ~30 | Display format (E-prefixed hex) |
| `diagnostic_code_parsing` | 11 | ~77 | FromStr round-trip tests |
| `code_range_partitioning` | 6 | ~84 | High-nibble range validation |

**Total:** 11 modules, 134 tests

### 2.3 DDD Cohesion Assessment

| Criterion | Status | Notes |
|-----------|--------|-------|
| Single responsibility | ✓ | Each module tests one diagnostic concern |
| Boundary cohesion | ✓ | `vb_validate::diagnostic` public API tested |
| No cross-crate leakage | ✓ | Only imports `vb_core::diagnostic`, `vb_validate::ValidationError` |
| Error domain alignment | ✓ | Maps to ValidationError taxonomy (E01xx–E06xx) |
| Invariant testing | ✓ | `diagnostic_structure_invariants` covers all variants |

---

## 3. Architectural Drift Findings

### 3.1 Size Gate Compliance

| Gate | Threshold | Actual | Status |
|------|-----------|--------|--------|
| Max file lines | 300 | 1537 | **VIOLATION** |
| Max module lines | 150 | 576 (error_message_formatting) | **VIOLATION** |

**Severity:** MEDIUM

The `error_message_formatting` module at 576 lines exceeds the 150-line module threshold by ~3.8x. This module contains 52 near-identical test cases that could be refactored.

### 3.2 Test Repetition Pattern

The `error_message_formatting` module exhibits copy-paste repetition:

```rust
// Pattern repeated 52 times with minor variations
#[test]
fn <error_variant>_message_exact_format() {
    let diag = vb_validate::diagnostic::diagnostic_from_error(&ValidationError::<Variant> { ... });
    assert_eq!(&*diag.message, "expected message");
}
```

**Recommendation:** Consider parameterized tests using `try_build` or a test generator macro to reduce repetition from 52 cases to ~5 parameterized test functions.

### 3.3 Positive Architectural Signals

1. **Well-organized ranges:** E01xx–E06xx correctly partitions error taxonomy
2. **Invariant testing:** `all_validation_errors()` helper covers all 50+ variants exhaustively
3. **Code parsing gaps documented:** Tests correctly note E05xx/E06xx cannot round-trip via `FromStr` due to `is_supported_code()` gaps
4. **No unsafe/unwrap/panic:** Clean test code
5. **Clear module docstrings:** Explains test scope and purpose

---

## 4. Recommendation

| Priority | Action | Rationale |
|----------|--------|-----------|
| **HIGH** | Refactor `error_message_formatting` module | 576 lines violates <300 file / <150 module gates |
| MEDIUM | Split `diagnostic_structure_invariants` if it grows | `all_validation_errors()` helper is excellent but module is 223 lines |
| LOW | Consider parameterized tests for code-range modules | Could reduce 6 modules × avg 60 lines to 1 parameterized module |

### Refactoring Approach

The `error_message_formatting` module could be reduced via:

```rust
// Instead of 52 individual tests:
#[test]
fn error_messages_are_well_formed_for_all_variants() {
    let cases = all_validation_errors();
    for error in cases {
        let diag = vb_validate::diagnostic::diagnostic_from_error(&error);
        assert!(!diag.message.is_empty(), "{error:?}");
        // Could check message format patterns here
    }
}
```

However, the **exact content assertions** (52 tests) provide valuable regression coverage. The current structure is **acceptable** if the file-size waiver exists, but **not ideal** per strict <300 line gate.

---

## 5. Summary

| Aspect | Assessment |
|--------|------------|
| File location | ✓ Correct |
| Test count | 134 (high coverage) |
| Architectural cohesion | ✓ Strong |
| Size compliance | ✗ Violates <300 line file gate |
| Module size | ✗ `error_message_formatting` at 576 lines |
| Test quality | ✓ Excellent (exact assertions, good invariants) |
| **Overall** | **ACCEPTABLE with refactoring recommendation** |

---

*Generated by architectural-drift agent*
