# Architectural Drift Report: `vb_compile_mod_compile_lowering_tests`

## File Analysis

| Metric | Value |
|--------|-------|
| **File Path** | `crates/vb_compile/src/mod_compile_lowering/tests.rs` |
| **Total Lines** | 1410 |
| **Test Count** | 44 |
| **Location Category** | `tests/` — integration/unit test file |
| **Size Classification** | **LARGE** (exceeds 300-line soft cap by 4.7×) |

## Structural Observations

### Cohesion & Boundary Concerns

1. **Single-file bloat**: 1410 lines in one `tests.rs` file violates the 300-line soft cap by significant margin. This is a structural cohesion violation.

2. **Two distinct test suites cohabit**:
   - **Digest coverage suite** (lines 1–515): Tests `compute_compiled_digest` and `digest_step_primitive` for `Collect` field hashing (PO-003 through PO-014, vb-awhr).
   - **Choose-width/canonical-choose suite** (lines 517–1199+): Tests `choose_width`, `lower_canonical_choose`, `lower_choose`, `slot_from_text`, `add_body_offset`, `emit_choose_branch_body` (Plan N2–N15, vb-xi2f.13, vb-282my).

3. **Helper function scope**: Helper functions (`collect_yaml_with_field`, `make_collect`, `digest_primitive`, `choose_body_set_step`, `choose_body_do_step`) are defined in-test and pollute the module-level namespace.

4. **Cross-module imports at test level**: Line 521 imports `part_01::{body_width, choose_width}`, `part_02::lower_canonical_choose`, `part_05::slot_from_text`, `part_06::lower_choose`. These are module-internal imports leaking into the test file.

### DDD Boundary Assessment

- **Module**: `mod_compile_lowering` — compilation pipeline from YAML AST to slot-compiled IR.
- **Domain concept**: Digest hashing for reproducibility; choose/branch lowering with slot allocation.
- **Problem**: Two unrelated domain concerns are tested in the same file. Digest hashing (Collect field hashing) and Choose lowering (slot allocation) share no semantic relationship.

## Recommendation

**REFACTOR — Split into multiple focused test files**:

| New File | Purpose | Approx. Lines |
|----------|---------|---------------|
| `tests/digest_collect.rs` | `compute_compiled_digest`, `digest_step_primitive` coverage for Collect hashing | ~520 |
| `tests/choose_lowering.rs` | `choose_width`, `lower_canonical_choose`, `lower_choose` | ~680 |
| `tests/slot_allocation.rs` | `slot_from_text`, `add_body_offset`, `emit_choose_branch_body` | ~250 |

**Helpers**: Extract to `tests/helpers/mod.rs` or `tests/support/` module to avoid namespace pollution.

**Import hygiene**: Move cross-part imports to individual test functions rather than module-level.

### Risk Assessment

| Risk | Level | Note |
|------|-------|------|
| File size bloat | **HIGH** | 4.7× over soft cap |
| Cohesion | **MEDIUM** | Two unrelated domains co-located |
| Maintenance burden | **MEDIUM** | 1410-line file is harder to navigate |

### Enforcement

Per architectural-drift skill: Files exceeding 300 lines should be flagged for split review. This file requires immediate refactoring attention.
