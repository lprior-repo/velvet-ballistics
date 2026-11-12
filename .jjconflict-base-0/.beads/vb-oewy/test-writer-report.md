---
bead_id: vb-oewy
bead_title: "bdd: Full suite runner and evidence artifact contract"
phase: 9
updated_at: 2026-05-20T05:40:00Z
attempt: 1
---

# Test Writer Report — vb-oewy

## Test File

`crates/workspace_tests/tests/bdd_runner_tests.rs`

## Failing-First Evidence

These tests were written before the bdd_runner module was integrated into workspace_tests.
Expected failures:
1. Missing `bdd_runner` module import in `workspace_tests/src/lib.rs`
2. Missing `serde_yaml` dev-dependency in workspace_tests

## Test Coverage

| Obligation ID | Test Name | RRO Coverage |
|---|---|---|
| POST-001 | `test_suite_result_total_invariant` | RRO-001 |
| POST-001 | `test_suite_result_total_invariant_zero_counts` | RRO-001 |
| POST-001 | `test_suite_result_total_invariant_only_passed` | RRO-001 |
| POST-003 | `test_status_exhaustive_match` | RRO-002 |
| POST-003 | `test_status_equality` | RRO-002 |
| POST-003 | `test_status_inequality` | RRO-002 |
| POST-005 | `test_evidence_bundle_yaml_roundtrip` | RRO-004 |
| POST-005 | `test_scenario_result_yaml_roundtrip` | RRO-004 |
| POST-005 | `test_failed_scenario_serialization_preserves_error` | RRO-004 |
| POST-004 | `test_failed_scenario_carry_error` | RRO-003 |
| POST-004 | `test_passed_scenario_has_no_error` | RRO-003 |
| POST-004 | `test_skipped_scenario_has_no_error` | RRO-003 |
| POST-006 | `test_error_variant_is_correct_for_discovery_failed` | RRO-005 |
| POST-006 | `test_error_display_deterministic` | RRO-005 |
| INV-001 | `test_all_catalog_scenarios_have_results` | RRO-006 |
| INV-001 | `test_scenario_id_matches_catalog` | RRO-006 |
| INV-001 | `test_catalog_scenario_has_given_when_then` | RRO-006 |
| INV-003 | `test_no_shared_state_pollution` | RRO-007 |
| INV-003 | `test_executor_context_clone_is_independent` | RRO-007 |
| INV-004 | `test_schema_version_enforced` | RRO-008 |

## Dependencies Used

- `serde_json` — available via `serde.workspace = true` in workspace_tests
- `acceptance_catalog` — from `vb_workspace_tests::acceptance_catalog`

## Missing Integration Steps

1. Add `bdd_runner` module to `workspace_tests/src/lib.rs`: `pub mod bdd_runner;`
2. Add `serde_yaml` to workspace_tests dev-dependencies
3. Add `xtask` as a dev-dependency for evidence bundle types (or inline the types)

## Red Phase Evidence

Tests are written to fail before the bdd_runner module is integrated. Expected compile error: `cannot find module vb_workspace_tests::bdd_runner`.
