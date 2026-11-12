# Test Coverage Matrix: vb-shvxy (State 8)

## Overview

Maps each of the 16 refinement obligations to planned tests across test layers. Shows complete traceability from proof obligations through behaviors to test identifiers.

| Mapping ID | Obligation ID | Bead ID | State |
|------------|--------------|---------|-------|
| TCM-001 | RRO-001 through RRO-012L | vb-shvxy | 8 (test-planner) |

---

## Legend

- **Test Layer**: S=Static, I=Integration, E=E2E, P=Proptest, F=Fuzz
- **Status**: PLANNED (State 8) → State 9 writes; DEPLOYED → after State 9
- **Assertion Style**: EC=Exact Exit Code, SM=Substring Match, NV=Non-Vacuous Count, SC=Shape Check

---

## Full Coverage Matrix

### RRO-001: kani-list.sh produces valid JSON with 176 harnesses for vb_core

| Behavior # | Behavior Description | Test ID | Test Layer | Test Name | Assertion Style | Status |
|------------|---------------------|---------|------------|-----------|-----------------|--------|
| B001 | Exits 2 with usage when no args | I01 | Integration | `kani_list_exits_2_with_usage_when_no_args` | EC=2, SM="usage:" | PLANNED |
| B002 | Exits 1 when cargo kani missing | I02 | Integration | `kani_list_exits_1_when_cargo_kani_missing` | EC=1, SM="required on PATH" | PLANNED |
| B003 | Produces valid JSON for vb_core | I03 | Integration | `kani_list_produces_valid_json_for_vb_core_with_nonzero_harnesses` | EC=0, SM="KANI_LIST_OK", NV=176 | PLANNED |
| B004 | Produces valid JSON for vb_runtime | I04 | Integration | `kani_list_produces_valid_json_for_vb_runtime_with_nonzero_harnesses` | EC=0, NV=6 | PLANNED |
| B005 | Exits 1 for nonexistent package | I05 | Integration | `kani_list_exits_1_for_nonexistent_package` | EC=1, SM=error | PLANNED |
| B008 | Outputs to KANI_LIST_DIR override | I08 | Integration | `kani_list_outputs_to_KANI_LIST_DIR_override` | EC=0, SC=file in custom dir | PLANNED |
| B009 | Exits 1 on empty JSON | I09 | Integration | `kani_list_exits_1_on_empty_json_output` | EC=1, SM="did not produce" | PLANNED |
| B010 | Output JSON is valid by json.tool | I10 | Integration | `kani_list_output_json_is_valid_json_by_python_json_tool` | EC=0 | PLANNED |
| — | Any package → valid JSON or error | P01 | Proptest | Property: package_acceptance_property | NV, SC | PLANNED |
| — | JSON always valid on success | P04 | Proptest | Property: valid_json_guarantee | SC | PLANNED |
| — | shellcheck clean | S01 | Static | shellcheck on kani-list.sh | lint=0 | PLANNED |
| — | shebang + execute bit | S02 | Static | script metadata audit | pattern match | PLANNED |
| — | JSON schema validation | S03 | Static | schema check against kani-list.schema.json | schema=pass | PLANNED |
| — | moon ci tasks exist | S05 | Static | .moon/tasks/kani.yml is valid YAML | format=valid | PLANNED |
| — | Full moon pipeline | E01 | E2E | `moon_ci_verifier_tooling_pipeline_all_passes` | EC=0 | PLANNED |
| — | Multi-lane smoke | E02 | E2E | `multi_lane_evidence_smoke_every_lane_produces_output` | NV>0 all lanes | PLANNED |
| — | Evidence dir audit | E03 | E2E | `evidence_directory_audit_all_artifacts_present` | SC=expected files | PLANNED |

**RRO-001 Coverage**: 17 tests (13 I, 2 P, 3 S, 3 E shared)

---

### RRO-002: kani-list.sh produces valid JSON with 6 harnesses for vb_runtime

| Behavior # | Behavior Description | Test ID | Test Layer | Test Name | Assertion Style | Status |
|------------|---------------------|---------|------------|-----------|-----------------|--------|
| B004 | Produces valid JSON for vb_runtime | I04 | Integration | `kani_list_produces_valid_json_for_vb_runtime_with_nonzero_harnesses` | EC=0, NV=6, SM="reentry_proofs" | PLANNED |
| — | shellcheck clean | S01 | Static | shellcheck on kani-list.sh | lint=0 | PLANNED |
| — | shebang + execute bit | S02 | Static | script metadata audit | pattern match | PLANNED |
| — | moon ci tasks exist | S05 | Static | .moon/tasks/kani.yml is valid YAML | format=valid | PLANNED |
| — | Full moon pipeline | E01 | E2E | `moon_ci_verifier_tooling_pipeline_all_passes` | EC=0 | PLANNED |
| — | Multi-lane smoke | E02 | E2E | `multi_lane_evidence_smoke_every_lane_produces_output` | NV>0 all lanes | PLANNED |
| — | Evidence dir audit | E03 | E2E | `evidence_directory_audit_all_artifacts_present` | SC=expected files | PLANNED |

**RRO-002 Coverage**: 7 tests (1 I, 3 S, 3 E shared)

---

### RRO-003: KANI_FEATURES env var undeclared feature causes exit 1

| Behavior # | Behavior Description | Test ID | Test Layer | Test Name | Assertion Style | Status |
|------------|---------------------|---------|------------|-----------|-----------------|--------|
| B006 | Fails closed with undeclared feature | I06 | Integration | `kani_list_fails_closed_when_KANI_FEATURES_requests_undeclared_feature` | EC=1, SM="failed to select a version" | PLANNED |
| B007 | Succeeds with declared feature passthrough | I07 | Integration | `kani_list_succeeds_with_declared_KANI_FEATURES_passthrough` | EC=0, NV>0 | PLANNED |
| — | moon ci tasks exist | S05 | Static | .moon/tasks/kani.yml is valid YAML | format=valid | PLANNED |
| — | Full moon pipeline | E01 | E2E | `moon_ci_verifier_tooling_pipeline_all_passes` | EC=0 | PLANNED |

**RRO-003 Coverage**: 4 tests (2 I, 1 S, 1 E shared)

---

### RRO-004: flux-check-package.sh exits 0 with refinement check on vb_core

| Behavior # | Behavior Description | Test ID | Test Layer | Test Name | Assertion Style | Status |
|------------|---------------------|---------|------------|-----------|-----------------|--------|
| B011 | Exits 2 with usage when no package | I11 | Integration | `flux_check_exits_2_with_usage_when_no_package` | EC=2, SM="usage:" | PLANNED |
| B012 | Executes cargo flux -p for package | I12 | Integration | `flux_check_executes_cargo_flux_for_package_with_exit_0` | EC=0, SM=compilation | PLANNED |
| B018 | Passes through valid flags | I18 | Integration | `flux_check_passes_through_valid_flags_to_cargo_flux` | EC=0 | PLANNED |
| B020 | Propagates cargo flux failure | I20 | Integration | `flux_check_propagates_cargo_flux_failure_exit_code` | EC!=0 | PLANNED |
| — | shellcheck clean | S01 | Static | shellcheck on flux-check-package.sh | lint=0 | PLANNED |
| — | shebang + execute bit | S02 | Static | script metadata audit | pattern match | PLANNED |
| — | Full moon pipeline | E01 | E2E | `moon_ci_verifier_tooling_pipeline_all_passes` | EC=0 | PLANNED |
| — | Multi-lane smoke | E02 | E2E | `multi_lane_evidence_smoke_every_lane_produces_output` | NV>0 all lanes | PLANNED |
| — | Evidence dir audit | E03 | E2E | `evidence_directory_audit_all_artifacts_present` | SC=expected files | PLANNED |

**RRO-004 Coverage**: 9 tests (4 I, 2 S, 3 E shared)

---

### RRO-005: flux-check-package.sh rejects unsupported selectors with exit 2

| Behavior # | Behavior Description | Test ID | Test Layer | Test Name | Assertion Style | Status |
|------------|---------------------|---------|------------|-----------|-----------------|--------|
| B013 | Rejects --lib | I13 | Integration | `flux_check_rejects_lib_selector_with_exit_2_and_message` | EC=2, SM="unsupported ... --lib" | PLANNED |
| B014 | Rejects --test | I14 | Integration | `flux_check_rejects_test_selector_with_exit_2_and_message` | EC=2, SM="unsupported ... --test" | PLANNED |
| B015 | Rejects --tests | I15 | Integration | `flux_check_rejects_tests_selector_with_exit_2_and_message` | EC=2, SM="unsupported ... --tests" | PLANNED |
| B016 | Rejects --benches | I16 | Integration | `flux_check_rejects_benches_selector_with_exit_2_and_message` | EC=2, SM="unsupported ... --benches" | PLANNED |
| B017 | Rejects --all-targets | I17 | Integration | `flux_check_rejects_all_targets_selector_with_exit_2_and_message` | EC=2, SM="unsupported ... --all-targets" | PLANNED |
| B019 | Rejects multiple selectors | I19 | Integration | `flux_check_rejects_multiple_unsupported_selectors` | EC=2, SM=first unsupported | PLANNED |
| — | Any subset of selectors rejected | P02 | Proptest | Property: any_selector_subset_rejected | EC=2 | PLANNED |
| — | Deterministic exit codes | P05 | Proptest | Property: deterministic_exit_codes | EC=stable | PLANNED |
| — | shellcheck clean | S01 | Static | shellcheck on flux-check-package.sh | lint=0 | PLANNED |
| — | shebang + execute bit | S02 | Static | script metadata audit | pattern match | PLANNED |
| — | Full moon pipeline | E01 | E2E | `moon_ci_verifier_tooling_pipeline_all_passes` | EC=0 | PLANNED |

**RRO-005 Coverage**: 11 tests (6 I, 2 P, 2 S, 1 E shared)

---

### RRO-006: guard-zero-tests.sh exits 1 when cargo test reports 0 applicable

| Behavior # | Behavior Description | Test ID | Test Layer | Test Name | Assertion Style | Status |
|------------|---------------------|---------|------------|-----------|-----------------|--------|
| B021 | Exits 2 without args | I21 | Integration | `guard_zero_tests_exits_2_without_args` | EC=2, SM=usage | PLANNED |
| B022 | Exits 1 on zero applicable | I22 | Integration | `guard_zero_tests_exits_1_when_zero_applicable_tests` | EC=1, SM="count=0" | PLANNED |
| B026 | Detects "0 passed, M filtered" | I26 | Integration | `guard_zero_tests_detects_0_passed_M_filtered_as_zero` | EC=1, SM="count=0" | PLANNED |
| B027 | Exits 1 on cargo test nonzero exit | I27 | Integration | `guard_zero_tests_exits_1_on_cargo_test_nonzero_exit` | EC=1 | PLANNED |
| B028 | Exits 1 on unparseable output | I28 | Integration | `guard_zero_tests_exits_1_on_unparseable_output` | EC=1, SM=parse | PLANNED |
| B029 | Detects "running 0 tests" | I29 | Integration | `guard_zero_tests_detects_running_0_tests_as_zero` | EC=1, SM="count=0" | PLANNED |
| — | (N=0, M=any) → exit 1 | P03 | Proptest | Property: zero_applicable_always_fail_closed | EC=1 ∀ N=0 | PLANNED |
| — | shellcheck clean | S01 | Static | shellcheck on guard-zero-tests.sh | lint=0 | PLANNED |
| — | shebang + execute bit | S02 | Static | script metadata audit | pattern match | PLANNED |
| — | Full moon pipeline | E01 | E2E | `moon_ci_verifier_tooling_pipeline_all_passes` | EC=0 | PLANNED |
| — | Evidence dir audit | E03 | E2E | `evidence_directory_audit_all_artifacts_present` | SC=expected files | PLANNED |

**RRO-006 Coverage**: 11 tests (6 I, 1 P, 2 S, 2 E shared)

---

### RRO-007: guard-zero-tests.sh exits 0 for non-zero proptest execution

| Behavior # | Behavior Description | Test ID | Test Layer | Test Name | Assertion Style | Status |
|------------|---------------------|---------|------------|-----------|-----------------|--------|
| B023 | Exits 0 on nonzero applicable | I23 | Integration | `guard_zero_tests_exits_0_when_nonzero_applicable_tests` | EC=0, SM="PASS", NV>0 | PLANNED |
| B024 | Parses "N passed" format | I24 | Integration | `guard_zero_tests_parses_simple_N_passed_format` | EC=0, SM="count=5" | PLANNED |
| B025 | Parses "N passed, M filtered" | I25 | Integration | `guard_zero_tests_parses_N_passed_M_filtered_format` | EC=0, SM="count=5" | PLANNED |
| — | (N>0, M=any) → exit 0 | P03 | Proptest | Property: nonzero_applicable_always_accept | EC=0 ∀ N>0 | PLANNED |
| — | shellcheck clean | S01 | Static | shellcheck on guard-zero-tests.sh | lint=0 | PLANNED |
| — | shebang + execute bit | S02 | Static | script metadata audit | pattern match | PLANNED |
| — | Full moon pipeline | E01 | E2E | `moon_ci_verifier_tooling_pipeline_all_passes` | EC=0 | PLANNED |
| — | Multi-lane smoke | E02 | E2E | `multi_lane_evidence_smoke_every_lane_produces_output` | NV>0 all lanes | PLANNED |
| — | Evidence dir audit | E03 | E2E | `evidence_directory_audit_all_artifacts_present` | SC=expected files | PLANNED |

**RRO-007 Coverage**: 9 tests (3 I, 1 P, 2 S, 3 E shared)

---

### RRO-008: cargo fuzz list enumerates registered fuzz targets

| Behavior # | Behavior Description | Test ID | Test Layer | Test Name | Assertion Style | Status |
|------------|---------------------|---------|------------|-----------|-----------------|--------|
| B033 | Lists registered fuzz target names | I33 | Integration | `cargo_fuzz_list_exits_0_and_lists_target_names` | EC=0, SM=target names | PLANNED |
| B034 | Produces non-empty target list | I34 | Integration | `cargo_fuzz_list_produces_nonempty_target_count` | EC=0, NV>0 | PLANNED |
| — | prefix-closed under fuzz/Cargo.toml | P06 | Proptest | Property: target_list_prefix_closed | NV=match | PLANNED |
| — | Full moon pipeline | E01 | E2E | `moon_ci_verifier_tooling_pipeline_all_passes` | EC=0 | PLANNED |
| — | Multi-lane smoke | E02 | E2E | `multi_lane_evidence_smoke_every_lane_produces_output` | NV>0 all lanes | PLANNED |
| — | Evidence dir audit | E03 | E2E | `evidence_directory_audit_all_artifacts_present` | SC=expected files | PLANNED |

**RRO-008 Coverage**: 6 tests (2 I, 1 P, 3 E shared)

---

### RRO-009: cargo fuzz build compiles all targets with GNU triple

| Behavior # | Behavior Description | Test ID | Test Layer | Test Name | Assertion Style | Status |
|------------|---------------------|---------|------------|-----------|-----------------|--------|
| B035 | Compiles all targets with GNU triple | I35 | Integration | `cargo_fuzz_build_compiles_with_gnu_target` | EC=0, SC=binaries exist | PLANNED |
| B036 | Fails with unsupported target | I36 | Integration | `cargo_fuzz_build_fails_with_unsupported_target` | EC!=0 | PLANNED |
| — | Full moon pipeline | E01 | E2E | `moon_ci_verifier_tooling_pipeline_all_passes` | EC=0 | PLANNED |

**RRO-009 Coverage**: 3 tests (2 I, 1 E shared)

---

### RRO-010: 13 model tests compile and execute under cfg(loom)

| Behavior # | Behavior Description | Test ID | Test Layer | Test Name | Assertion Style | Status |
|------------|---------------------|---------|------------|-----------|-----------------|--------|
| B037 | Loom model tests execute | I37 | Integration | `loom_model_tests_compile_and_execute_under_cfg_loom` | EC=0, SM="passed", NV=13 | PLANNED |
| — | xtask compiles clean | S04 | Static | cargo clippy -p xtask | warning=0 | PLANNED |
| — | Full moon pipeline | E01 | E2E | `moon_ci_verifier_tooling_pipeline_all_passes` | EC=0 | PLANNED |
| — | Multi-lane smoke | E02 | E2E | `multi_lane_evidence_smoke_every_lane_produces_output` | NV>0 all lanes | PLANNED |
| — | Evidence dir audit | E03 | E2E | `evidence_directory_audit_all_artifacts_present` | SC=expected files | PLANNED |

**RRO-010 Coverage**: 5 tests (1 I, 1 S, 3 E shared)

---

### RRO-011: loom-list.sh discovers 5 Loom models

| Behavior # | Behavior Description | Test ID | Test Layer | Test Name | Assertion Style | Status |
|------------|---------------------|---------|------------|-----------|-----------------|--------|
| B030 | Lists 5 models | I30 | Integration | `loom_list_exits_0_and_lists_5_models` | EC=0, SM=model names, NV=5 | PLANNED |
| B031 | Exits 1 when xtask unavailable | I31 | Integration | `loom_list_exits_1_when_xtask_unavailable` | EC=1 | PLANNED |
| B032 | Exits 1 when model list empty | I32 | Integration | `loom_list_exits_1_when_model_list_empty` | EC=1, SM="empty" | PLANNED |
| — | shellcheck clean | S01 | Static | shellcheck on loom-list.sh | lint=0 | PLANNED |
| — | shebang + execute bit | S02 | Static | script metadata audit | pattern match | PLANNED |
| — | Full moon pipeline | E01 | E2E | `moon_ci_verifier_tooling_pipeline_all_passes` | EC=0 | PLANNED |
| — | Multi-lane smoke | E02 | E2E | `multi_lane_evidence_smoke_every_lane_produces_output` | NV>0 all lanes | PLANNED |
| — | Evidence dir audit | E03 | E2E | `evidence_directory_audit_all_artifacts_present` | SC=expected files | PLANNED |

**RRO-011 Coverage**: 8 tests (3 I, 2 S, 3 E shared)

---

### RRO-012K through RRO-012L: Closure Obligations (deferred to State 10)

| Obligation | Status | Reason |
|------------|--------|--------|
| RRO-012K (Kani closure) | DEFERRED | Routed to State 10 (formal-verifier). Requires evidence classification, applicable_count > 0 guard enforcement in verification-ledger.jsonl. |
| RRO-012F (Flux closure) | DEFERRED | Routed to State 10. Requires Flux evidence record classification with applicable_count > 0 or Blocker. |
| RRO-012P (Proptest closure) | DEFERRED | Routed to State 10. Requires proptest lane closure with zero-applicable as Blocker. |
| RRO-012C (Fuzz closure) | DEFERRED | Routed to State 10. Requires cargo-fuzz evidence classification and non-vacuous guard. |
| RRO-012L (Loom closure) | DEFERRED | Routed to State 10. Requires Loom lane closure with cfg dependency failure as Blocker. |

---

## Mutation Kill Matrix

| Mutation # | Target Branch | Expected Survivor Test | Kill Status |
|------------|--------------|----------------------|-------------|
| M01 | kani-list.sh arg count check removed | I01 (no-args test should catch: exit 0 instead of exit 2) | PLANNED |
| M02 | kani-list.sh cargo kani existence check removed | I02 (missing-tool test: should exit 0 instead of exit 1) | PLANNED |
| M03 | kani-list.sh empty JSON check inverted | I09 (empty JSON: should exit 0 instead of exit 1) | PLANNED |
| M04 | kani-list.sh package match quorum changed | I05 (nonexistent package: should exit 0 instead of exit 1) | PLANNED |
| M05 | flux-check-package.sh arg count check removed | I11 (no args: should exit 0 instead of exit 2) | PLANNED |
| M06 | flux-check-package.sh --lib case removed | I13 (--lib: should exit 0 instead of exit 2) | PLANNED |
| M07 | flux-check-package.sh --test case removed | I14 (--test: should exit 0 instead of exit 2) | PLANNED |
| M08 | flux-check-package.sh --tests case removed | I15 (--tests: should exit 0 instead of exit 2) | PLANNED |
| M09 | flux-check-package.sh --benches case removed | I16 (--benches: should exit 0 instead of exit 2) | PLANNED |
| M10 | flux-check-package.sh --all-targets case removed | I17 (--all-targets: should exit 0 instead of exit 2) | PLANNED |
| M11 | guard-zero-tests.sh count comparsion inverted | I22+I23 (zero/nonzero both produce wrong exit) | PLANNED |
| M12 | guard-zero-tests.sh "running 0 tests" removed | I29 (running 0 tests: should exit 0 instead of exit 1) | PLANNED |
| M13 | guard-zero-tests.sh "filtered out" subtraction removed | I26 (0 passed + filtered: wrong count, wrong exit) | PLANNED |
| M14 | guard-zero-tests.sh unparseable handler removed | I28 (unparseable: should exit 0 instead of exit 1) | PLANNED |
| M15 | guard-zero-tests.sh nonzero passthrough removed | I27 (cargo failure: should exit 0 instead of exit 1) | PLANNED |
| M16 | loom-list.sh empty check reversed | I32 (empty models: should exit 0 instead of exit 1) | PLANNED |
| M17 | loom-list.sh xtask failure check removed | I31 (xtask missing: should exit 0 instead of exit 1) | PLANNED |
| M18 | kani-list.sh KANI_FEATURES passthrough removed | I06+I07 (both test feature behavior: exit changes) | PLANNED |
| M19 | kani-list.sh KANI_LIST_DIR override removed | I08 (dir override: wrong output location) | PLANNED |
| M20 | kani-list.sh json.tool validation removed | I10 (invalid JSON would go undetected) | PLANNED |

**Mutation kill threshold target**: 20/20 = 100% (must not drop below 90% = 18/20).

---

## Cross-Lane Coverage Summary

| Lane | Obligations | Unique Tests | Shared Tests | Total Tests |
|------|-------------|--------------|--------------|-------------|
| Kani | RRO-001, 002, 003 | I01-I10, P01, P04, S03, S05 | S01, S02, E01, E02, E03 | 19 unique + 5 shared = 24 |
| Flux | RRO-004, 005 | I11-I20, P02, P05 | S01, S02, E01, E02, E03 | 12 unique + 5 shared = 17 |
| Proptest | RRO-006, 007 | I21-I29, P03 | S01, S02, E01, E02, E03 | 10 unique + 5 shared = 15 |
| Fuzz | RRO-008, 009 | I33-I36, P06 | E01, E02, E03 | 5 unique + 3 shared = 8 |
| Loom | RRO-010, 011 | I30-I32, I37, S04 | S01, S02, E01, E02, E03 | 5 unique + 5 shared = 10 |
| **TOTAL** | **11 obligations** | **37 behaviors** | **35 unique tests + shared** | **74 test assertions** |

---

## Contract Clause Coverage

| Contract Clause | Obligations Covered | Test IDs |
|----------------|--------------------|----------|
| C-002 Availability preflight | RRO-001, 004, 008, 010 | I02, I05, I20, I31, I36 |
| C-003 Non-vacuous success | RRO-001, 002, 006, 007, 008 | I03, I04, I22, I23, I26, I29, I34 |
| C-005 Kani feature parity | RRO-003 | I06, I07 |
| C-006 Flux wrapper shape | RRO-005 | I13-I17, I19 |
| C-008 Proptest zero-test guard | RRO-006, 007 | I21-I29 |
| C-009 Fuzz target/sanitizer guard | RRO-008, 009 | I33-I36 |
| C-010 Loom cfg/dependency guard | RRO-010, 011 | I30-I32, I37 |
| C-012 Fail closed on unknowns | RRO-006 | I28 (unparseable → fail closed) |

---

## Validation Notes

- All planned tests assert exact exit codes, never just `result.success()`/`result.failure()`.
- All non-vacuous obligations require a count assertion (SM="count=N" or NV=N).
- Every public-API behavior on every script has at least one BDD scenario.
- Every parsing/deserialization boundary has a fuzz target.
- Every error variant in the `VerifierBlocker` taxonomy has at least one test triggering it.
- Proptest invariants cover all multi-input pure functions.
- Mutation kill threshold is 90% minimum; 100% targeted.
- The 5 closure obligations (RRO-012K through RRO-012L) are correctly deferred to State 10.
