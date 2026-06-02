# Research Notes: arch: Enumerate all first-party Rust files over 300 lines

**Bead:** vb-zxgb
**Date:** 2026-06-01 (corrected)
**Researcher:** Lewis

## Executive Summary

- **Total files exceeding 300 lines:** 498
- **Hot paths** (non-test-like, >300 lines): 131 files
- **Cold paths** (test-like, >300 lines): 367 files

## Methodology

Used exact `is_excluded_source_path()` and `is_test_like_source_path()` functions from `scripts/check-source-length.sh`:

### Exclusions (is_excluded_source_path)
```bash
target/*|.jj/*|.beads/*|.evidence/*|.cargo_temp/*|arch-drift-*/*|*/target/*|*/.jj/*|*/.beads/*|*/.evidence/*|*/.cargo_temp/*
cargo-home/*|cargo_home/*|.cargo/registry/*|*/cargo-home/*|*/cargo_home/*|*/.cargo/registry/*
```

### Cold Test/Diagnostic Paths (is_test_like_source_path)
```bash
*/tests.rs|*/*_tests.rs|*/*tests*.rs|*/tests/*|*/tests/**/*|*/tests*/*|*/tests*/**
*/kani*.rs|*/kani/*|*/kani/**/*|*/verification/*|*/verification/**/*|verification/*|verification/**/*|*/proptest*.rs|*/benches/*|*/benches/**/*
```

### Hot Paths
Everything NOT matching `is_excluded_source_path` AND NOT `is_test_like_source_path` AND exceeding 300 physical lines.

## Accurate Enumeration

### Hot Paths (131 files > 300 lines, non-test-like)

Top 30 largest hot files:
| Lines | File |
|-------|------|
| 2393 | crates/vb_core/src/budget.rs |
| 2070 | crates/vb_core/src/diagnostic.rs |
| 2021 | crates/vb_storage/src/batch.rs |
| 1756 | crates/vb_validate/src/gates.rs |
| 1242 | crates/vb_core/src/frame.rs |
| 1101 | crates/vb_core/src/ids/mod.rs |
| 1046 | crates/vb_core/src/workflow/validation.rs |
| 1028 | crates/vb_proof_kernels/src/resource_budget.rs |
| 1016 | crates/vb_expr/src/eval.rs |
| 1005 | crates/vb_ipc/src/kani_flag_validation.rs |
| 989 | crates/vb_storage/src/recovery/replay/summary.rs |
| 894 | crates/vb_compile/src/compile/mod.rs |
| 837 | crates/vb_cli/src/explain_validation.rs |
| 805 | crates/vb_validate/src/schema_fields.rs |
| 792 | crates/vb_core/src/kani_idempotency_gates.rs |
| 782 | crates/vb_ipc/src/server/helpers.rs |
| 779 | crates/vb_core/src/workflow/types.rs |
| 774 | crates/vb_expr/src/eval/evaluate.rs |
| 738 | crates/vb_core/src/errors.rs |
| 724 | crates/vb_compile/src/ast/parse.rs |
| 722 | crates/vb_core/src/action.rs |
| 676 | crates/vb_runtime/src/engine/action.rs |
| 675 | crates/vb_cli/src/commands_ai_context.rs |
| 672 | crates/vb_ipc/src/server/trace.rs |
| 655 | crates/vb_compile/src/expression.rs |
| 653 | crates/vb_proof_kernels/src/vb_kyyf_normalization.rs |
| 651 | crates/vb_storage/src/admission.rs |
| 638 | crates/vb_validate/src/diag_render.rs |
| 609 | crates/vb_core/src/kani_workflow_arbitrary.rs |
| 606 | crates/vb_storage/src/recovery/types.rs |

### Cold Paths (367 files > 300 lines, test-like)

Top 30 largest test/diagnostic files:
| Lines | File |
|-------|------|
| 7743 | crates/vb_storage/src/tests.rs |
| 7227 | crates/vb_core/src/budget/tests.rs |
| 4224 | crates/vb_core/src/replay/tests.rs |
| 3994 | crates/vb_cli/tests/cli_integration.rs |
| 3753 | crates/vb_runtime/src/collect_tests.rs |
| 3716 | crates/vb_runtime/src/primitives/collect/tests.rs |
| 3432 | crates/vb_storage/src/recovery/tests.rs |
| 2886 | crates/vb_storage/src/codec/tests.rs |
| 2864 | crates/vb_compile/src/mod_compile_lowering/tests.rs |
| 2860 | crates/vb_runtime/tests/recovery_bdd_tests.rs |
| 2740 | crates/vb_expr/src/eval_tests.rs |
| 2607 | crates/vb_core/tests/section36_mandatory_coverage.rs |
| 2596 | crates/vb_compile/tests/v1_primitive_lowering.rs |
| 2568 | crates/vb_runtime/src/engine/tests.rs |
| 2520 | crates/vb_validate/src/type_taint_tests.rs |
| 2442 | crates/vb_storage/src/journal/tests.rs |
| 2412 | crates/workspace_tests/benches/velvet_ballistics.rs |
| 2311 | crates/workspace_tests/tests/vb_qi37_2_4_integration_budget_errors.rs |
| 2226 | crates/vb_workspace_tests/tests/vb_kyyf_cross_run_determinism.rs |
| 2226 | crates/vb_ipc/src/server/impl_tests.rs |
| 2210 | crates/vb_compile/src/tests/error_variant_tests.rs |
| 2166 | crates/vb_core/src/replay/step_tests.rs |
| 2087 | crates/vb_runtime/tests/recovery_hydration_tests.rs |
| 2086 | crates/vb_core/src/value_store/tests.rs |
| 1974 | crates/vb_workspace_tests/tests/restate_timer_deadline_primitive_tests.rs |
| 1972 | crates/vb_compile/src/expression_bytecode_tests.rs |
| 1971 | crates/vb_storage/src/vb_2bok_durability_gate_tests.rs |
| 1932 | crates/vb_cli/src/app_impl_tests.rs |
| 1927 | crates/vb_core/src/action/tests.rs |
| 1915 | crates/vb_ipc/src/tests.rs |

## Key Findings

1. **Test density**: 74% of large files (367/498) are test/diagnostic code
2. **Hot path concentration**: The 131 hot files represent the core code needing architectural attention
3. **Exception ledger bloat**: `.config/source-length-exceptions.txt` has ~480 entries

## Verification

```bash
# Count all tracked .rs files minus exclusions, >300 lines
git ls-files "*.rs" | [filter via is_excluded_source_path] | [wc -l on filtered files >300 lines]

# Results verified:
# Total: 498
# Hot: 131
# Cold: 367
```

## Notes

This research epic produces enumeration data as a research artifact. The data is intended for:
- Architectural drift detection planning
- Refactoring prioritization
- Hot/cold path analysis for performance work
