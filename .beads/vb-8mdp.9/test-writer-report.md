# Test Writer Report — vb-8mdp.9 State 9

**Date:** 2026-05-30
**Agent:** test-writer (femdation child)
**Source checkout:** `/home/lewis/src/velvet-ballistics`
**Isolated workspace:** `/home/lewis/src/femdation-vb-8mdp.9`

## Summary

All 17 behavior test groups implemented. 36 new test functions across 8 crates. All tests pass.

## Test Count by Behavior

| Behavior | Obligation | Crate | File | New Tests | Status |
|----------|-----------|-------|------|-----------|--------|
| B-01 | PO-003 | vb_core | errors.rs | 0 (existing coverage sufficient) | PASS |
| B-02 | PO-005 | vb_runtime | tests_basic.rs | 7 | PASS |
| B-03/B-04/B-17 | PO-008/PO-008b/PO-026 | vb_ipc | tests.rs | 1 (group verification) | PASS |
| B-05 | PO-011 | vb_validate | proptest_validation_error_codes.rs | 1 | PASS |
| B-06 | PO-012 | workspace_tests | section17_runtime_code_reverse_parity.rs | 2 | PASS |
| B-07 | PO-012b | workspace_tests | section17_runtime_code_coverage_report.rs | 3 | PASS |
| B-08 | PO-013 | vb_runtime | tests_conversion_refinement.rs | 3 | PASS |
| B-09 | PO-014 | vb_runtime | tests_conversion_refinement.rs | 3 | PASS |
| B-10 | PO-015 | vb_runtime | tests_conversion_refinement.rs | 2 | PASS |
| B-11 | PO-016 | vb_compile | error_variant_tests.rs | 2 | PASS |
| B-12 | PO-017 | vb_compile | error_variant_tests.rs | 2 | PASS |
| B-13 | PO-020 | vb_core | proptest_registry_consistency.rs | 1 | PASS |
| B-14 | PO-021 | vb_core | errors.rs | 3 | PASS |
| B-15 | PO-024 | vb_runtime | tests_conversion_refinement.rs | 3 | PASS |
| B-16 | PO-025 | vb_cli | error_chain_integration.rs | 3 | PASS |
| — | — | — | — | — | — |
| **Total** | **13 unique obligations** | **8 crates** | **10 files** | **36 test functions** | **ALL PASS** |

## Test Function Details

### B-02: RuntimeError runtime_code mappings (7 functions)
- `runtime_error_runtime_code_journal_full_maps_to_queue_full`
- `runtime_error_runtime_code_unsupported_async_strict_ack_maps_to_storage_error`
- `runtime_error_runtime_code_admission_artifact_digest_mismatch_maps_to_admission_durability_error`
- `runtime_error_runtime_code_admission_artifact_stale_maps_to_admission_durability_error`
- `runtime_error_runtime_code_admission_digest_mismatch_maps_to_admission_durability_error`
- `runtime_error_runtime_code_engine_drive_failed_maps_to_action_failed`
- `runtime_error_runtime_code_returns_none_for_unmapped_variants`

### B-03/B-04/B-17: IpcError runtime_code semantic groups (1 function)
- `ipc_error_runtime_code_semantics_groups` — enumerates all 14 IpcError variants, verifies exact group counts: 8 IPC_FRAME_INVALID, 2 IPC_PAYLOAD_TOO_LARGE, 1 QUEUE_FULL, 3 None

### B-05: Section 16 reverse parity (1 function)
- `section16_reverse_parity_validation_error` — verifies all 46 Section 16 code names map to at least one ValidationError variant

### B-06: Section 17 reverse parity (2 functions)
- `section17_reverse_parity_mapped_codes_have_sources` — all 19 mapped codes have runtime_code() sources
- `section17_reverse_parity_unmapped_codes_have_no_sources` — 14 unmapped codes have zero runtime_code() sources

### B-07: Section 17 coverage report (3 functions)
- `section17_coverage_report_mapped_codes_match_runtime`
- `section17_coverage_report_unmapped_codes_stay_unmapped`
- `section17_coverage_report_counts_are_correct`

### B-08: Core→Runtime::Core propagation (3 functions)
- `propagation_core_to_runtime_core_preserves_fieldful_variant_through_box`
- `propagation_core_to_runtime_core_preserves_diagnostic_code`
- `propagation_core_to_runtime_core_preserves_multiple_fieldful_variants`

### B-09: EngineDriveFailed propagation (3 functions)
- `propagation_engine_drive_failed_preserves_run_id`
- `propagation_engine_drive_failed_preserves_core_error_source`
- `propagation_engine_drive_failed_returns_correct_diagnostic_code`

### B-10: Journal→Runtime propagation (2 functions)
- `propagation_journal_to_storage_journal_append_preserves_variant_through_arc`
- `propagation_journal_to_storage_journal_append_preserves_fieldful_variant`

### B-11: Validation→Compile propagation (2 functions)
- `propagation_validation_to_compile_validation_preserves_duplicate_key_variant`
- `propagation_validation_to_compile_validation_preserves_multiple_variants`

### B-12: Workflow→Compile propagation (2 functions)
- `propagation_workflow_to_compile_workflow_preserves_empty_nodes_variant`
- `propagation_workflow_to_compile_workflow_preserves_multiple_variants`

### B-13: CODE_REGISTRY bijection (1 function)
- `registry_bijection_unique_names_and_codes` — 234 unique symbolic names, 234 unique numeric codes

### B-14: CoreError Display determinism (3 functions)
- `core_error_display_determinism_static_message_variants`
- `core_error_display_determinism_field_variants`
- `core_error_display_determinism_cross_invocation_stability`

### B-15: Error::source() chain (3 functions)
- `error_source_chain_returns_some_for_core_wrapping_variant`
- `error_source_chain_returns_some_for_storage_journal_append_variant`
- `error_source_chain_returns_none_for_non_wrapping_variants`

### B-16: Core→Runtime Display chain (3 functions)
- `core_to_runtime_display_chain_integrity_fieldful_variant`
- `core_to_runtime_display_chain_engine_drive_failed`
- `core_to_runtime_display_chain_cross_layer_stability`

## Existing Test Repairs

Two existing tests had stale counts that were fixed:

1. **`core_error_runtime_codes_are_unique`** (vb_core/src/errors.rs): Added `CAPABILITY_DENIED_RUNTIME_CODE`, count 13→14
2. **`runtime_error_runtime_codes_are_unique`** (vb_runtime/src/error/tests_basic.rs): Added `ADMISSION_DURABILITY_ERROR_RUNTIME_CODE`, count 3→4

## Files Modified

| File | Action |
|------|--------|
| `crates/vb_core/src/errors.rs` | B-14 Display tests appended; B-01 uniqueness count fixed |
| `crates/vb_core/tests/proptest_registry_consistency.rs` | B-13 bijection test appended |
| `crates/vb_runtime/src/error/tests_basic.rs` | B-02 runtime_code tests appended; uniqueness count fixed |
| `crates/vb_runtime/src/error/tests_conversion_refinement.rs` | B-08, B-09, B-10, B-15 propagation/source tests appended |
| `crates/vb_ipc/src/tests.rs` | B-03/B-04/B-17 semantic groups test appended |
| `crates/vb_validate/tests/proptest_validation_error_codes.rs` | B-05 Section16 reverse parity appended |
| `crates/vb_compile/src/tests/error_variant_tests.rs` | B-11, B-12 propagation tests appended |
| `crates/vb_cli/tests/error_chain_integration.rs` | B-16 Display chain tests appended |
| `crates/workspace_tests/tests/section17_runtime_code_reverse_parity.rs` | **NEW FILE** — B-06 |
| `crates/workspace_tests/tests/section17_runtime_code_coverage_report.rs` | **NEW FILE** — B-07 |
| `crates/workspace_tests/Cargo.toml` | Added 2 new `[[test]]` sections |

## Assertion Quality

All 36 new tests use exact assertions:
- `assert_eq!` with specific expected strings
- `matches!()` with exact variant + field destructuring
- `assert!` on boolean conditions with descriptive failure messages
- Zero uses of bare `is_ok()`/`is_err()`

## Evidence Commands

```bash
# vb_core (4 tests)
cargo test -p vb_core --lib -- core_error_display_determinism
cargo test -p vb_core --test proptest_registry_consistency -- registry_bijection_unique_names_and_codes

# vb_runtime (18 tests)
cargo test -p vb_runtime --lib -- "propagation_core_to_runtime_core|propagation_engine_drive_failed|propagation_journal_to_storage_journal_append|error_source_chain|runtime_error_runtime_code"

# vb_ipc (1 test)
cargo test -p vb_ipc --lib -- ipc_error_runtime_code_semantics_groups

# vb_validate (1 test)
cargo test -p vb_validate --test proptest_validation_error_codes -- section16_reverse_parity

# vb_compile (4 tests)
cargo test -p vb_compile --lib -- "propagation_validation_to_compile|propagation_workflow_to_compile"

# vb_cli (3 tests)
cargo test -p velvet-ballistics --test error_chain_integration -- core_to_runtime_display_chain

# workspace_tests (5 tests)
cargo test -p velvet-ballistics-workspace-tests --test section17_runtime_code_reverse_parity
cargo test -p velvet-ballistics-workspace-tests --test section17_runtime_code_coverage_report
```

## Exit Criteria

- [x] All 17 behavior test groups implemented (B-01 through B-17)
- [x] 36 new test functions, 2 existing tests repaired
- [x] All tests compile and pass (0 failures)
- [x] All assertions are exact (assert_eq!, matches!, no is_ok/is_err)
- [x] Tests extend existing files per test-plan, not replacing
- [x] Cross-crate tests use real production types, no mocks
- [x] Existing proptest suites (186 tests) continue to pass
- [x] Evidence commands documented for each crate
