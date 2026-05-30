# Architectural Drift Report: `bdd_validation_tests.rs`

**File:** `crates/workspace_tests/tests/bdd_validation_tests.rs`  
**Analyzed:** 2026-05-29  
**Skill:** architectural-drift

---

## Summary

| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| Total Lines | 1415 | 300 | **VIOLATION** |
| Test Count | 55 | — | — |
| Line/test Ratio | 25.7 | — | Healthy |

---

## Findings

### 1. File Size Violation

The file contains **1415 lines**, exceeding the **300-line maximum** by **4.7×**.

This is a test file, not production code, so the strict production limit is relaxed, but the sheer size makes the file difficult to navigate and maintain.

### 2. Structural Observations

- **Helper constructors** (lines 21–88): `make_parts`, `finish_node`, `nop_node`, `do_node`, `make_contract` — 5 helpers totaling ~70 lines
- **Test organization**: Tests are grouped by BDD scenario (B1–B15) with clear section headers
- **No DDD violations detected**: Test-only file, uses only `vb_core` and `vb_validate` imports correctly
- **No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, or `unimplemented` found**

---

## Recommendations

### Split into thematic modules

The file covers 15 BDD scenario groups (B1–B15) across 55 tests. Recommended split:

```
tests/
  bdd_validation/
    mod.rs                    # 50 lines - module re-exports
    bdd_g01_g05_basic.rs      # ~200 lines - B1: basic validate tests
    bdd_g06_g09_pipeline.rs   # ~200 lines - B2-B5: pipeline tests
    bdd_g10_g11_slot_gates.rs # ~250 lines - B8-B11: slot reference tests
    bdd_g12_g15_structural.rs # ~300 lines - B12-B17: node-kind tests
    bdd_g07_expression.rs     # ~180 lines - B12-B14: expression stack tests
    bdd_g08_accessor.rs       # ~180 lines - B15-B17: accessor path tests
    bdd_g09_slot_refs.rs      # ~250 lines - B18-B23: slot reference tests
    bdd_g10_node_kind.rs      # ~280 lines - B25-B32: node-kind structural tests
    bdd_g11_loop_body.rs      # ~200 lines - B34-B37: loop body graph tests
    bdd_g12_action_contract.rs # ~200 lines - B39-B41: action contract bijection tests
    bdd_g13_slot_cycles.rs    # ~220 lines - B43-B45: slot cycle detection tests
    bdd_g14_slot_type.rs      # ~200 lines - B47-B48: slot type compatibility tests
    bdd_g15_determinism.rs    # ~200 lines - B50-B51: determinism separation tests
    bdd_error_variants.rs     # ~200 lines - B53-B56: error handling tests
```

### Keep together (acceptable alternative)

If the team prefers a single file for test discovery simplicity, the current structure is acceptable since:
- Tests are well-grouped by BDD scenario
- Helper functions are reusable
- No unwrap/panic in test code
- Line/test ratio (25.7) is healthy for BDD tests

---

## Verdict

**STATUS: REFACTOR-CANDIDATE**

The file exceeds the 300-line soft limit by 4.7×. For a **test file**, this is a minor concern — the primary risk is maintainability, not architectural drift. No unsafe patterns, no primitive obsession, no workflow violations.

Recommend splitting if future tests are added, otherwise acceptable as-is.
