# Architectural Drift Report: vb_kyyf_cross_run_determinism.rs

## File Summary

| Metric | Value |
|--------|-------|
| **Location** | `crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs` |
| **Total Lines** | 2226 |
| **File Size** | 78,170 bytes (~76 KB) |
| **Test Count** | 16 tests |
| **Category** | Cross-crate Integration Test (workspace_tests) |

## Analysis

### Structure
- **Module kind**: Integration test file for cross-run determinism verification
- **Domain**: BDD-style acceptance testing for `vb-kyyf` (proof kernel determinism)
- **Dependencies**: `vb_core`, `vb_proof_kernels`, `vb_runtime`, `vb_storage`

### Architectural Observations

1. **Size Warning**: At 2226 lines, this file exceeds the 300-line soft cap from architectural-drift rules by ~7x
2. **Cohesion**: Multiple concerns mixed:
   - Scenario validation logic
   - CLI subprocess execution
   - Journal/replay testing
   - Evidence artifact generation
3. **DDD Alignment**: Tests `vb-kyyf` bead behavior across public surfaces of `vb_runtime` and `vb_storage`
4. **Boundary Concerns**: Cross-crate integration tests appropriately located in `workspace_tests`

### Location Category
- **Category**: `workspace_tests/` — Cross-crate integration tests (appropriate placement)

## Recommendations

| Priority | Recommendation |
|----------|----------------|
| **HIGH** | Split into multiple focused test files by scenario (BDD-KYYF-001, BDD-KYYF-002, BDD-KYYF-003, BDD-KYYF-004) |
| **HIGH** | Extract shared helper functions into `workspace_tests/src/` module |
| **MEDIUM** | Consider moving evidence artifact generation to separate utilities crate |

## Metrics Summary

- **Lines**: 2226 ⚠️ (exceeds 300-line guideline)
- **Test Functions**: 16
- **Lines per Test**: ~139 (average, including helpers)
- **Drift Status**: Requires refactoring for structural cohesion
