# Architectural Drift Report: `vb_qi37_4_2_strict_runtime_admission.rs`

## File Summary

| Metric | Value |
|--------|-------|
| **File** | `crates/workspace_tests/tests/vb_qi37_4_2_strict_runtime_admission.rs` |
| **Lines** | 1535 |
| **Size** | 49.2 KB |
| **Location** | `crates/workspace_tests/tests/` (workspace integration test) |

## Test Count

| Category | Count |
|----------|-------|
| Unit `#[test]` functions | 16 |
| `proptest!` block inner tests | 5 |
| **Total Test Functions** | **21** |

### Test Inventory

1. `given_missing_artifact_when_strict_run_created_then_artifact_not_found_before_allocation`
2. `given_malformed_bytes_when_strict_run_created_then_decode_failed_with_rejected_digest`
3. `given_gate_count_zero_two_fourteen_or_sixteen_when_strict_run_created_then_gate_mismatch_denies`
4. `given_non_durable_artifact_when_strict_run_created_then_durable_proof_flag_denies`
5. `given_digest_mismatch_when_strict_run_created_then_digest_mismatch_denies`
6. `given_stale_artifact_when_strict_run_created_then_stale_certificate_denies`
7. `given_missing_excess_prefix_or_action_mismatched_capability_then_capability_denied`
8. `given_valid_accepted_artifact_when_admitted_then_admission_record_contains_digest_certificate_profile`
9. `given_budget_over_capacity_when_admission_with_budget_runs_then_resource_capacity_error_is_preserved`
10. `proptest_gate_count_acceptance_is_singleton_canonical_15` (proptest)
11. `given_raw_or_malformed_storage_bytes_when_strict_run_created_then_decode_failed_matrix_denies`
12. `given_invalid_envelope_semantic_matrix_when_strict_run_created_then_typed_invalid_diagnostic_denies`
13. `given_cli_ipc_runtime_error_mapping_when_serialized_then_error_category_digest_and_cause_are_preserved`
14. `given_any_admission_error_when_runtime_returns_then_no_frame_run_or_drive_state_allocated`
15. `given_strict_journaled_runtime_when_constructed_then_storage_backed_artifact_store_is_required`
16. `given_valid_accepted_artifact_when_runtime_admits_then_yaml_json_decoder_is_not_called`
17. `given_existence_only_artifact_check_when_strict_admission_then_bypass_is_denied`
18. `proptest_capability_profiles_admit_if_and_only_if_sets_are_identical` (proptest)
19. `proptest_fail_closed_envelope_predicate_denies_any_invalid_field` (proptest)
20. `proptest_digest_equality_is_required_across_requested_record_and_envelope` (proptest)
21. `proptest_diagnostic_mapping_is_injective_over_admission_error_categories` (proptest)

## Architectural Assessment

### Size Compliance

| Rule | Limit | Actual | Status |
|------|-------|--------|--------|
| Max lines per file | 300 | 1535 | **VIOLATION** |
| Max test file size | — | 49.2 KB | Elevated |

**This file is 5.1× the recommended maximum line count.**

### DDD Cohesion Analysis

The file tests `vb_runtime::admission` strict runtime admission policies with high cohesion around:
- Artifact envelope validation (gate count, proof flags, digest matching)
- Capability-based access control
- Resource budget admission
- Error categorization and diagnostic mapping

### Boundary Compliance

- **✓** Uses only `vb_core`, `vb_runtime`, `vb_storage` public APIs
- **✓** Tests stay within `workspace_tests` integration layer
- **✓** No `unsafe`, `unwrap`, `expect`, `panic`, `dbg` found (line 448 has `panic!` in error path — acceptable in test scaffolding)

## Recommendations

1. **SPLIT RECOMMENDED** — The 1535-line file should be decomposed into:
   - `vb_qi37_4_2_strict_runtime_admission_diagnostics.rs` — diagnostic mapping tests
   - `vb_qi37_4_2_strict_runtime_admission_capabilities.rs` — capability/proptest tests
   - `vb_qi37_4_2_strict_runtime_admission_envelope.rs` — envelope validation tests
   - `vb_qi37_4_2_strict_runtime_admission_integration.rs` — shard/integration tests

2. **DRIFT RISK: LOW** — Test coverage is exhaustive and behaviorally sound. The file is oversized but architecturally correct in its domain boundaries.

3. **MAINTAIN** — The `proptest` property tests (5 generated cases) provide strong semantic coverage. Keep them co-located with the unit tests they exercise.

---
*Report generated: 2026-05-29*  
*Tool: architectural-drift analyzer*
