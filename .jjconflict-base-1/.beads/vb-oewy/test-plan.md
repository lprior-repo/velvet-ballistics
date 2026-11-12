---
bead_id: vb-oewy
bead_title: "bdd: Full suite runner and evidence artifact contract"
phase: 8
updated_at: 2026-05-20T05:35:00Z
attempt: 1
---

# Test Plan — vb-oewy

## Test Target

`crates/workspace_tests/tests/bdd_runner_tests.rs`

## Derivation

Tests are derived from:
- `contract.md` — preconditions, postconditions, invariants
- `traceability-matrix.jsonl` — clause-to-test mapping
- `rust-refinement-obligations.jsonl` — proof-to-test bridge
- `proof-to-rust-map.md` — source and test evidence paths

## Test Obligations

| Obligation | Test Name | Proof/Refinement ID |
|---|---|---|
| POST-001 (total >= sum invariant) | `test_suite_result_total_invariant` | PO-001, RRO-001 |
| POST-002 (catalog coverage) | `test_all_catalog_scenarios_have_results` | PO-002, RRO-002 |
| POST-003 (status exhaustive) | `test_status_exhaustive_match` | PO-003, RRO-002 |
| POST-004 (error field) | `test_failed_scenario_carry_error` | PO-004, RRO-003 |
| POST-005 (YAML roundtrip) | `test_evidence_bundle_yaml_roundtrip` | PO-005, RRO-004 |
| POST-006 (Err infrastructure-only) | `test_runner_returns_err_infrastructure_only` | PO-006, RRO-005 |
| INV-001 (scenario ID matching) | `test_scenario_id_matches_catalog` | PO-007, RRO-006 |
| INV-003 (no shared state) | `test_no_shared_state_pollution` | PO-009, RRO-007 |
| INV-004 (schema versioning) | `test_schema_version_enforced` | PO-010, RRO-008 |

## Happy Path Tests

### test_suite_result_total_invariant

**Given**: a BddSuiteResult with known total, passed, failed, skipped
**When**: the invariant `total == passed + failed + skipped` is checked
**Then**: assertion passes for all valid combinations

### test_status_exhaustive_match

**Given**: BddScenarioStatus enum
**When**: all three variants are constructed and pattern-matched
**Then**: all three match arms are reachable and produce correct discriminant values

### test_evidence_bundle_yaml_roundtrip

**Given**: a valid BddSuiteResult
**When**: it is serialized to YAML and deserialized back
**Then**: the deserialized result equals the original exactly

## Error Path Tests

### test_failed_scenario_carry_error

**Given**: a BddScenarioResult with status Failed
**When**: the result is inspected
**Then**: error field is Some with non-empty string

### test_runner_returns_err_infrastructure_only

**Given**: the runner encounters various failure conditions
**When**: run_bdd_suite returns Err
**Then**: the error variant is one of: DiscoveryFailed, ExecutionFailed, ParseFailed, EvidenceWriteFailed, NoTestBinary

## Edge Case Tests

### test_all_catalog_scenarios_have_results

**Given**: the acceptance catalog has 10 scenarios
**When**: run_bdd_suite completes
**Then**: every scenario ID from the catalog appears in results

### test_scenario_id_matches_catalog

**Given**: a result with scenario_id
**When**: the ID is looked up in acceptance_catalog::catalog()
**Then**: it is found exactly (case-sensitive match)

### test_no_shared_state_pollution

**Given**: the same scenario file is run twice sequentially
**When**: both runs complete
**Then**: both produce identical results (no shared state mutation)

### test_schema_version_enforced

**Given**: an EvidenceBundle with a future schema version
**When**: deserialization is attempted
**Then**: it is rejected (schema validation)

## Test Implementation Notes

- All tests use `#![forbid(unsafe_code)]`
- No `unwrap()` or `expect()` on fallible operations — use `?` or `assert!(result.is_ok())`
- Tests for infrastructure errors use `matches!(err, BddRunnerError::Variant { .. })`
- YAML roundtrip uses `serde_yaml::to_string` and `serde_yaml::from_str`
- Catalog lookup uses `acceptance_catalog::catalog()` and searches by `scenario.id`

## Execution

```bash
cargo test -p workspace_tests bdd_runner
```
