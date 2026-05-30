# Architectural Drift Report: `workspace_tests/benches/velvet_ballistics.rs`

**File:** `crates/workspace_tests/benches/velvet_ballistics.rs`
**Date:** 2026-05-29
**Analyst:** architectural-drift agent

---

## 1. File Size Summary

| Metric | Value |
|--------|-------|
| **Total Lines** | 2412 |
| **Line Limit Exemption** | Bench files are exempt from the 300-line rule |
| **Size Category** | Large benchmark suite (justified by scope) |

---

## 2. Benchmark Inventory

| # | Benchmark Group | Count | Benchmarks |
|---|-----------------|-------|------------|
| 1 | `parse_yaml_benches` | 2 | `parse_yaml_small`, `parse_yaml_1mb` |
| 2 | `compile_and_validate_benches` | 4 | `validate_minimal`, `compile_ir_minimal`, `compile_ir_1000_steps`, `validate_1000_steps` |
| 3 | `expression_benches` | 4 | `expr_eq_symbol`, `expr_number_compare`, `expr_boolean_chain`, `expr_arithmetic` |
| 4 | `slot_and_transition_benches` | 9 | slot read/write, step_once, run_until_blocked variants, save_chain (10/1000 steps), choose (true/false), finish |
| 5 | `storage_and_ipc_benches` | 10 | memory ingress (3), postcard encode/decode, ipc frame encode/decode, fjall append, replay (1000 events) |
| 6 | `ir_execution_benches` | 4 | `ir_execution_1_step`, `ir_execution_1000_steps`, `ir_execution_choose_100`, `ir_execution_expr` |
| 7 | `taint_scalar_expr_bench` | 1 | `eval_expr_scalar_arithmetic_taint` |
| 8 | `taint_slot_loading_bench` | 2 | `eval_expr_slot_load_all_clean`, `eval_expr_slot_load_mixed_taint` |
| 9 | `taint_build_object_bench` | 3 | `build_object_2_fields_taint`, `build_object_8_fields_taint`, `build_object_16_fields_taint` |
| 10 | `taint_build_list_bench` | 3 | `build_list_2_items_taint`, `build_list_8_items_taint`, `build_list_16_items_taint` |
| 11 | `taint_full_workflow_bench` | 2 | `full_workflow_all_clean`, `full_workflow_mixed_taint` |
| 12 | `submit_artifact_benches` | 3 | `submit_artifact_relaxed`, `submit_artifact_journaled`, `submit_artifact_strict` |
| 13 | `budget_compute_benches` | 4 | `budget_compute_small_workflow`, `budget_compute_save_chain_10`, `budget_compute_save_chain_1000`, `budget_validate_default_policy` |
| 14 | `evidence_chain_benches` | 3 | `evidence_chain_accumulate_100_events`, `evidence_chain_accumulate_1000_events`, `evidence_chain_snapshot_100_events` |
| 15 | `admission_gate_benches` | 4 | `admit_run_relaxed`, `admit_run_strict_artifact_present`, `admit_run_multiple_action_caps`, `admit_run_empty_caps` |
| 16 | `capability_check_benches` | 7 | any_workflow, action_match_first, action_miss, empty_denies, mixed_set, admission_gate (2 variants) |
| | **TOTAL** | **63** | |

---

## 3. DDD Cohesion Analysis

### Module Organization
- **✓ Well-structured**: Benchmark groups align with domain boundaries (yaml, compile, expression, runtime, storage, taint, admission)
- **✓ Helper functions co-located**: `save_chain_workflow`, `choose_slot_workflow`, `finish_workflow`, `taint_*_workflow` helpers live near their benchmark groups
- **✓ Clear separation**: Budget helpers (`budget_utilization_percent`, `latency_within_budget`, `checked_iter`) form a reusable module

### Import Cohesion
- Imports from `vb_core`, `vb_runtime`, `vb_storage`, `vb_ipc`, `vb_expr` are appropriate for cross-crate integration benchmarks
- No evidence of circular dependencies

---

## 4. Structural Observations

### Strengths
1. **Comprehensive coverage**: 16 benchmark groups covering the full workflow lifecycle
2. **Explicit metadata**: `BENCH_METADATA` string with tool, durability, mode, latency profile
3. **Budget enforcement**: `checked_iter` wrapper enforces latency budgets per iteration
4. **Test coverage**: Unit tests for budget helpers (lines 130-156)
5. **Clear naming**: Benchmark IDs contain fixture, surface, and metadata

### Observations (Not Violations)
1. **Repetitive workflow construction**: `save_chain_workflow`, `choose_slot_workflow`, `finish_workflow` share similar `CompiledWorkflow::try_from_parts` patterns — could be unified but current form is readable
2. **Helper proliferation**: Multiple `taint_*_workflow` helpers suggest complex test matrix — appropriate for benchmarks
3. **Workflow construction is verbose**: Manual `CompiledNode` construction is necessary for precise benchmark control

---

## 5. Recommendation

| Aspect | Verdict |
|--------|---------|
| **Size** | ✅ **APPROVED** — Benchmarks are explicitly exempt; 2412 lines is justified by 63 benchmarks |
| **Cohesion** | ✅ **APPROVED** — Well-organized by domain, clear helper patterns |
| **Drift Risk** | ✅ **LOW** — No architectural violations detected |
| **Action** | **NO REFACTORING REQUIRED** |

### Summary
This benchmark file is a well-structured, comprehensive suite covering yaml parsing, compilation, expression evaluation, runtime execution, storage/IPC, taint propagation, artifact submission, budget computation, evidence chains, admission gates, and capability checks. The 300-line rule does not apply to benchmark files. No architectural drift detected.
