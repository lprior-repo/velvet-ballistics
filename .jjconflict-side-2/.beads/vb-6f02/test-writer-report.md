# State 10: Test Writer Report

## Summary

Test suites for vb-6f02 (Contracts-as-Data Suite) comprise 4 test files across 78 tests:
- 31 production binding tests (all passing)
- 17 proptest property tests (all passing)
- 9 Kani proof harnesses (written, pending execution)
- 30 integration tests (10 passing, 20 failing)

## Test Files

### 1. `crates/workspace_tests/tests/contracts_production_binding.rs` (31 tests)

**Status: 31/31 PASSING**

Production binding tests — import directly from `xtask::contracts::*` and `xtask::evidence::*`. No local copies.

| # | Test | Obligation | Description |
|---|------|------------|-------------|
| 1 | `test_prod_parse_schema_version_valid` | OBL-001 | Valid semver strings accepted |
| 2 | `test_prod_parse_schema_version_invalid` | OBL-001 | Malformed versions rejected |
| 3 | `test_prod_parse_schema_version_error_display` | OBL-001 | Error Display format |
| 4 | `test_prod_parse_contract_kind_all_valid` | OBL-002 | All 6 valid kinds parse |
| 5 | `test_prod_parse_contract_kind_invalid` | OBL-002 | Unknown kinds rejected |
| 6 | `test_prod_parse_contract_kind_error_display` | OBL-002 | Error format for unknown kinds |
| 7 | `test_prod_contract_kind_round_trip` | OBL-002 | Display → Parse round trip for all 6 |
| 8 | `test_prod_compare_semver_equal` | OBL-003 | Same versions compare Equal |
| 9 | `test_prod_compare_semver_less` | OBL-003 | Lower versions compare Less |
| 10 | `test_prod_compare_semver_greater` | OBL-003 | Higher versions compare Greater |
| 11 | `test_prod_compare_semver_invalid_format` | OBL-003 | Malformed semver returns error |
| 12 | `test_prod_parse_vet_exit_code_success` | OBL-004 | Exit code 0 returns Ok |
| 13 | `test_prod_parse_vet_exit_code_failure` | OBL-004 | Non-zero exit codes return Err |
| 14 | `test_prod_parse_vet_exit_code_error_message` | OBL-004 | Error message format |
| 15 | `test_prod_gate_evidence_pass` | OBL-005 | Pass case: invalid == 0 |
| 16 | `test_prod_gate_evidence_fail` | OBL-005 | Fail case: invalid > 0 |
| 17 | `test_prod_gate_evidence_empty_report` | OBL-005 | Empty report → Pass |
| 18 | `test_prod_gate_evidence_multiple_errors` | OBL-005 | Multiple errors in report |
| 19 | `test_prod_contract_file_serialization` | OBL-006 | ContractFile serde round-trip |
| 20 | `test_prod_discovery_report_serialization` | OBL-006 | DiscoveryReport JSON structure |
| 21 | `test_prod_report_summary_deterministic_key_order` | OBL-006 | BTreeMap key order in JSON |
| 22 | `test_prod_gate_evidence_serialization` | OBL-006 | GateEvidence Pass serialization |
| 23 | `test_prod_gate_evidence_fail_serialization` | OBL-006 | GateEvidence Fail serialization |
| 24 | `test_prod_summary_total_invariant_pass` | INV-002 | total == valid + invalid |
| 25 | `test_prod_summary_total_invariant_zero` | INV-002 | Zero summary |
| 26 | `test_prod_summary_total_invariant_overflow_safety` | INV-002 | saturating_add overflow safety |
| 27 | `test_prod_contract_kind_display_all` | OBL-002 | All 6 Display outputs |
| 28 | `test_prod_parse_schema_version_uses_valid` | OBL-001 | Returns original string |
| 29 | `test_prod_parse_contract_kind_case_sensitive` | OBL-002 | Case sensitivity |
| 30 | `test_prod_gate_evidence_exit_code_matches_status` | OBL-005 | Exit code ↔ status consistency |
| 31 | `test_prod_contract_error_all_variants_display` | ContractError | All 5 error variants Display |

**Repair 1: Verified.** All 31 tests import `xtask::contracts::{compare_semver, gate_evidence_from_report, parse_schema_version, parse_vet_exit_code, ContractError, ContractFile, ContractKind, DiscoveryReport, ReportSummary, SemverCmp}` and `xtask::evidence::{GateEvidence, GateStatus, WhyFailed}` directly.

### 2. `crates/workspace_tests/tests/contracts_as_data_props.rs` (17 tests)

**Status: 17/17 PASSING**

Supplementary property tests using proptest. Contains local mirror copies of `parse_schema_version`, `parse_contract_kind`, `parse_vet_exit_code`, and `compare_semver` for independent property testing.

| # | Property | Obligation | Description |
|---|----------|------------|-------------|
| 1 | `test_schema_version_accepts_valid_semver` | OBL-001 | Any u32.X.Y.Z format accepted |
| 2 | `test_schema_version_rejects_malformed` | OBL-001 | Non-semver strings rejected |
| 3 | `test_schema_version_matches_spec` | OBL-001 | Parse output matches spec fn |
| 4 | `test_schema_version_idempotent` | OBL-001 | Re-parsing accepted version is identity |
| 5 | `test_kind_rejects_unknown` | OBL-002 | Random strings rejected as kind |
| 6 | `test_btreemap_deterministic_json` | OBL-006 | Same pairs, different order → same JSON |
| 7 | `test_btreemap_sorted_keys` | OBL-006 | JSON keys in sorted order |
| 8 | `test_report_summary_invariant` | INV-002 | total == valid + invalid (property) |
| 9 | `test_compare_semver_reflexive` | OBL-004 | cmp(s, s) == Equal |
| 10 | `test_compare_semver_antisymmetric` | OBL-004 | cmp(a,b) = -cmp(b,a) |
| 11 | `test_compare_semver_transitive` | OBL-004 | a > b > c → a > c |
| 12 | `test_compare_semver_version_constraint` | OBL-009 | new >= old enforced |
| 13 | `test_compare_semver_monotonicity` | OBL-011 | No version downgrades |
| 14 | `test_contract_kind_all_values_parseable` | OBL-008 | all_values() all parse successfully |
| 15 | `test_contract_kind_display_matches_parse` | OBL-002 | Display output parses back |
| 16 | `test_contract_kind_arbitrary_all_covered` | OBL-008 | Arbitrary covers all 6 values |
| 17 | `test_compare_semver_all_edge_cases` | OBL-004 | Zero, max, cross-boundary comparisons |

### 3. `crates/workspace_tests/tests/contracts_as_data_kani.rs` (9 harnesses)

**Status: Written, pending execution**

Bounded model checking with `kani::any()` for structural inputs (per GOD RULE #1).

| # | Harness | Property | Obligation |
|---|---------|----------|------------|
| 1 | `kani_parse_schema_version_valid` | Valid semver passes | OBL-001 |
| 2 | `kani_parse_schema_version_empty` | Empty input → error | OBL-001 |
| 3 | `kani_parse_schema_version_leading_zero` | Leading zero rejected | OBL-001 |
| 4 | `kani_parse_schema_version_non_numeric` | Non-numeric rejected | OBL-001 |
| 5 | `kani_compare_semver_reflexive` | cmp(s,s) == Equal | OBL-004 |
| 6 | `kani_compare_semver_antisymmetric` | cmp(a,b) = -cmp(b,a) | OBL-004 |
| 7 | `kani_compare_semver_transitive` | a > b > c → a > c | OBL-004 |
| 8 | `kani_compare_semver_version_constraint` | Version constraint | OBL-009 |
| 9 | `kani_compare_semver_monotonicity` | No downgrade | OBL-011 |

### 4. `crates/workspace_tests/tests/contracts_integration.rs` (30 tests)

**Status: 10/30 PASSING, 20 FAILING**

Integration tests using `tempfile::TempDir` with `.cue` files containing valid CUE content.

| # | Test | Description | Status |
|---|------|-------------|--------|
| 1 | `test_discover_contracts_finds_all_files` | Finds 3 .cue files | FAILING (left: 0, right: 3) |
| 2 | `test_discover_contracts_valid_files` | All 3 files valid | FAILING |
| 3 | `test_discover_contracts_empty_dir` | Empty dir → 0 files | PASSING |
| 4 | `test_discover_contracts_nonexistent_dir` | Missing dir → error | PASSING |
| 5 | `test_discover_contracts_not_a_dir` | File instead of dir → error | PASSING |
| 6 | `test_discover_contracts_nested_cue_files` | Recurses into subdirs | FAILING (left: 0, right: 2) |
| 7 | `test_discover_contracts_mixed_valid_invalid` | Mix of valid/invalid | FAILING |
| 8 | `test_discover_contracts_sorted_output` | Files sorted by path | PASSING |
| 9 | `test_discover_contracts_deterministic` | Same output on repeated runs | PASSING |
| 10 | `test_discover_contracts_no_cue_files` | Dir with .txt files → 0 | PASSING |
| 11 | `test_validate_single_valid_file` | Valid .cue file passes | FAILING (left: 0, right: 1) |
| 12 | `test_validate_single_invalid_kind` | Invalid kind → error | FAILING (left: 0, right: 1) |
| 13 | `test_validate_single_missing_version` | Missing schema_version → error | FAILING |
| 14 | `test_validate_single_invalid_version` | Invalid version format → error | FAILING |
| 15 | `test_gate_passes_all_valid` | All valid → gate passes | FAILING (left: 0, right: 1) |
| 16 | `test_gate_fails_any_invalid` | Any invalid → gate fails | FAILING |
| 17 | `test_gate_empty_report_passes` | Empty report → gate passes | PASSING |
| 18 | `test_version_violation_detected` | Monotonicity breach detected | FAILING |
| 19 | `test_version_monotonicity_ok` | Monotonic upgrade accepted | FAILING |
| 20 | `test_errors_by_kind_counts` | Correct error counts per kind | FAILING |
| 21 | `test_total_valid_invalid_sum` | total == valid + invalid | FAILING |
| 22 | `test_cue_vet_error_collected` | cue vet errors collected | FAILING |
| 23 | `test_discover_contracts_unicode_paths` | Unicode in file paths | PASSING |
| 24 | `test_discover_contracts_special_chars` | Special characters in names | PASSING |
| 25 | `test_discover_contracts_deep_nesting` | Deep directory nesting | FAILING (left: 0, right: 1) |
| 26 | `test_gate_evidence_command_format` | Command string format | PASSING |
| 27 | `test_gate_evidence_exit_code` | Exit code matches status | PASSING |
| 28 | `test_discover_contracts_symlinks` | Symlinked directories | PASSING |
| 29 | `test_discover_contracts_hidden_files` | Hidden files (.cue) | PASSING |
| 30 | `test_discover_contracts_large_dir` | Large directory (100 files) | PASSING |

**Repair 2: Unresolved.** 20 of 30 integration tests fail with `left: 0, right: N` pattern — `discover_contracts()` returns 0 files in temp directory context. Root cause: `discover_contracts()` uses `collect_cue_files()` which has a path resolution issue with temp directories. This is a Repair requiring code change, not documentation.

## Requirement Coverage Map

| Req/Inv/Obl | Prod Binding | Proptest | Kani | Integration | Total |
|-------------|-------------|----------|------|-------------|-------|
| REQ-001 (CUE schemas) | — | — | — | 2 | 2 |
| REQ-002 (discovery + cue vet) | 4 | 3 | — | 8 | 15 |
| REQ-003 (schema_version + kind) | 5 | 5 | 4 | 4 | 18 |
| REQ-004 (GateEvidence integration) | 5 | — | — | 3 | 8 |
| REQ-005 (version monotonicity) | 3 | 2 | 2 | 2 | 9 |
| REQ-006 (kind completeness) | 4 | 4 | — | — | 8 |
| REQ-007 (cue vet pass) | 3 | — | — | 2 | 5 |
| REQ-008 (deterministic output) | 1 | 2 | — | 2 | 5 |
| REQ-009 (--json flag) | — | — | — | — | 0 |
| INV-001 (gate → all valid) | 1 | 1 | — | 1 | 3 |
| INV-002 (total = valid + invalid) | 3 | 1 | — | 1 | 5 |
| INV-003 (errors_by_kind sum) | 1 | — | — | 1 | 2 |
| INV-004 (no violations when pass) | 1 | — | — | 1 | 2 |
| INV-005 (sorted keys) | 1 | 2 | — | — | 3 |
| INV-006 (non-empty version) | 1 | 1 | — | 1 | 3 |
| INV-007 (ISO8601 timestamp) | — | — | — | — | 0 |
| INV-008 (violation detection) | — | 1 | — | 1 | 2 |
| OBL-001 (semver format) | 3 | 4 | 4 | 1 | 12 |
| OBL-002 (kind parsing total) | 5 | 4 | — | — | 9 |
| OBL-003 (semver comparison) | 4 | 1 | — | — | 5 |
| OBL-004 (parse_vet_exit_code) | 3 | — | — | — | 3 |
| OBL-005 (gate evidence) | 5 | — | — | 3 | 8 |
| OBL-006 (deterministic JSON) | 4 | 3 | — | — | 7 |
| OBL-008 (kind parsing) | 2 | 2 | — | — | 4 |
| OBL-009 (version constraint) | — | 1 | 1 | — | 2 |
| OBL-010 (CUE validation) | — | 1 | — | 2 | 3 |
| OBL-011 (monotonicity) | — | 1 | 1 | — | 2 |

## Coverage Gaps

| Gap | Severity | Description |
|-----|----------|-------------|
| REQ-009 | Low | `--json` flag not explicitly tested (CLI changes in cli.rs have no dedicated tests) |
| INV-007 | Low | ISO8601 timestamp format not tested (documented in TLA+ spec as enforced by Rust runtime) |
| Integration 20 failing | High | `discover_contracts()` returns 0 files in temp directory context |
| No property tests for gate_evidence_from_report | Medium | Gate mapping logic only tested in binding tests, not as properties |
| No property tests for ContractFile/DiscoveryReport serde | Medium | Serialization only tested in binding tests, not as properties |
| No property tests for ContractError variants | Low | Error variants only tested in binding tests |

## Repair History

### Repair 1: Production Binding — VERIFIED ✓
- 31 tests import `xtask::contracts::*` and `xtask::evidence::*` directly
- No local copies of production code
- All 31 tests pass

### Repair 2: Integration Test Discovery — UNRESOLVED ✗
- 20 of 30 integration tests fail with `left: 0, right: N`
- `discover_contracts()` returns empty file list in temp directory context
- Root cause: `collect_cue_files()` path resolution with temp directories
- **This is a code repair, not documentation** — requires fixing `collect_cue_files()` in contracts.rs

### Repair 3: Unwrap Cleanup — VERIFIED ✓
- All `unwrap()` calls in test files replaced with `prop_assert_eq!` on `Result` values
- No `unwrap()` in production binding, proptest, or Kani files
- `run_cue_vet()` line 244 retains `unwrap_or(1)` — intentional fallback for cue binary not found

## State 10 Verdict

**TESTS WRITTEN.** All test suites are on disk and compile. Production binding tests (31/31) and proptest properties (17/17) are passing. Integration tests have 20 failures requiring Repair 2 (code change).

- **58 tests passing**: 31 binding + 17 proptest + 10 integration
- **20 tests failing**: Integration tests (discover_contracts returns 0 files)
- **9 Kani harnesses written**: Pending execution via `cargo kani`
- **1 requirement gap**: REQ-009 (--json flag) — low priority, CLI tests not in scope
- **1 invariant gap**: INV-007 (ISO8601) — documented as enforced by Rust runtime, TLA+ spec models it
- **1 repair unresolved**: Repair 2 (integration test discovery) requires code change to `collect_cue_files()`
