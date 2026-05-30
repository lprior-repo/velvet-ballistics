# Architectural Drift Report: vb_test_validate_policy_enforce_behavior.rs

**File**: `crates/workspace_tests/tests/vb_test_validate_policy_enforce_behavior.rs`
**Date**: 2026-05-29
**Status**: REFACTOR REQUIRED

---

## Metrics

| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| Total Lines | **1479** | 300 max | ❌ VIOLATION (4.9x) |
| Test Count | **42** | N/A | Informational |
| File Size | ~43 KB | N/A | Too large |

---

## Violations

### 1. Line Count Violation (CRITICAL)
- **Required**: ≤ 300 lines per file
- **Actual**: 1479 lines
- **Excess**: 1179 lines over limit (393% of threshold)

---

## Structure Analysis

### Test Organization by Gate

| Gate | Test Count | Lines (approx) | Section |
|------|-----------|-----------------|---------|
| Gate 7 (Expression Stack) | 3 | ~100 | Lines 71-169 |
| Gate 8 (Accessor Paths) | 4 | ~130 | Lines 171-300 |
| Gate 9 (Slot References) | 3 | ~90 | Lines 302-391 |
| Gate 10 (Node Kind) | 4 | ~140 | Lines 393-532 |
| Gate 11 (Loop Body Graph) | 6 | ~250 | Lines 534-782 |
| Gate 13 (Slot Cycles) | 3 | ~130 | Lines 784-914 |
| Gate 14 (Slot Type Consistency) | 2 | ~90 | Lines 916-1009 |
| Gate 15 (Determinism Proof) | 3 | ~140 | Lines 1011-1153 |
| Enforcement Actions | 3 | ~90 | Lines 1155-1248 |
| Bypass Attempts | 4 | ~140 | Lines 1250-1381 |
| Violation Reporting | 7 | ~100 | Lines 1383-1479 |

### DDD Cohesion Assessment
- **Cohesion**: HIGH — Tests are well-grouped by validation gate/policy
- **Single Responsibility**: VIOLATED — File handles 8+ distinct validation gates
- **Primitive Obsession**: None detected — Uses proper newtypes (`StepIdx`, `SlotIdx`, `SymbolId`, etc.)

---

## Recommendations

### Split into 8 Focused Test Modules

```
crates/workspace_tests/tests/
├── vb_validate_gate_07_stack_tests.rs      (~150 lines, 3 tests)
├── vb_validate_gate_08_accessor_tests.rs   (~150 lines, 4 tests)
├── vb_validate_gate_09_slot_ref_tests.rs   (~100 lines, 3 tests)
├── vb_validate_gate_10_node_kind_tests.rs  (~150 lines, 4 tests)
├── vb_validate_gate_11_loop_tests.rs       (~260 lines, 6 tests)
├── vb_validate_gate_13_cycle_tests.rs       (~140 lines, 3 tests)
├── vb_validate_gate_14_type_tests.rs        (~100 lines, 2 tests)
├── vb_validate_gate_15_determinism_tests.rs (~150 lines, 3 tests)
├── vb_validate_enforcement_tests.rs         (~100 lines, 4 tests)  [bypass + enforcement]
└── vb_validate_violation_report_tests.rs    (~100 lines, 7 tests)
```

### Rationale
1. Each gate has distinct validation responsibility (DDD: bounded context per gate)
2. 150-260 lines per file stays within comfortable range of 300-line limit
3. Enables parallel test runs and focused debugging
4. Aligns with existing `gate_*_hammer.md` naming convention in sibling reports

### Shared Fixtures
Move shared helpers to a `vb_validate_policy_test_helpers.rs` module:
- `make_parts()`
- `finish_node()`
- `validate_gate_11_only()`
- `validate()`

---

## Summary

| Aspect | Assessment |
|--------|------------|
| Lines Count | ❌ 1479 >> 300 |
| Test Count | ✅ 42 (well-organized) |
| DDD Cohesion | ✅ High (by gate) |
| Primitive Obsession | ✅ None |
| File Size | ❌ Too large |

**Recommendation**: **SPLIT REQUIRED** — Break into 8-10 focused test files by gate, extract shared fixtures to a helper module.
