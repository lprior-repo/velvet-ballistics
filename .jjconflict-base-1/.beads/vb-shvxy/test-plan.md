# Test Plan: vb-shvxy Formal Verifier Tooling Lanes (State 8)

## Summary
- Bead: vb-shvxy
- State: 8 (test-planner)
- Source checkout: /home/lewis/src/velvet-ballistics
- Isolated workspace: /home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-shvxy
- Obligations in scope: RRO-001 through RRO-011 (11 tooling obligations)
- Obligations deferred to State 10: RRO-012K, RRO-012F, RRO-012P, RRO-012C, RRO-012L (5 closure)
- Behaviors identified: 37
- Trophy allocation: 0 unit / 32 integration / 3 e2e / 5 static (tooling-only bead)
- Proptest invariants: 6
- Fuzz targets: 4 (script argument fuzzing)
- Mutation checkpoints: 20

### Trophy Rationale

This is a **tooling infrastructure bead** with no production Rust behavior changes. All artifacts are bash scripts, CI configuration, and verifier wrapper infrastructure. The Testing Trophy distribution is therefore weighted toward integration tests (exercise scripts against real tooling), with static checks (shellcheck, clippy on xtask) and a small number of end-to-end pipeline tests.

| Trophy Layer   | Count | Rationale |
|----------------|-------|-----------|
| Static         | 5     | shellcheck on *.sh, cargo check on xtask loom integration, script shebang verification, execute-bit audit, JSON schema validation |
| Integration    | 32    | Most behaviors: invoke scripts with real cargo/kani/flux/fuzz, assert exit codes, parse output |
| E2E            | 3     | Full moon ci verifier-tooling pipeline, multi-lane smoke, evidence-dir audit |
| Unit / Calc    | 0     | Not applicable: no pure Rust Calc layer in this bead; bash script logic is tested via integration |

---

## 1. Behavior Inventory

### Script: kani-list.sh (RRO-001, RRO-002, RRO-003)

| # | Behavior |
|---|----------|
| B001 | kani-list.sh exits 2 with usage message when invoked with no arguments |
| B002 | kani-list.sh exits 1 when cargo kani is not on PATH |
| B003 | kani-list.sh produces valid JSON inventory for vb_core with non-zero harness count |
| B004 | kani-list.sh produces valid JSON inventory for vb_runtime with non-zero harness count |
| B005 | kani-list.sh exits 1 with error when package does not exist in workspace |
| B006 | kani-list.sh fails closed (exit 1) when KANI_FEATURES requests an undeclared feature |
| B007 | kani-list.sh succeeds with declared KANI_FEATURES feature passthrough |
| B008 | kani-list.sh produces JSON output to KANI_LIST_DIR override when env var is set |
| B009 | kani-list.sh exits 1 when cargo kani list produces empty JSON file |
| B010 | kani-list.sh validates output JSON with python3 -m json.tool |

### Script: flux-check-package.sh (RRO-004, RRO-005)

| # | Behavior |
|---|----------|
| B011 | flux-check-package.sh exits 2 with usage message when no package argument provided |
| B012 | flux-check-package.sh executes cargo flux -p for a valid package with exit 0 |
| B013 | flux-check-package.sh rejects --lib selector with exit 2 and specific error message |
| B014 | flux-check-package.sh rejects --test selector with exit 2 and specific error message |
| B015 | flux-check-package.sh rejects --tests selector with exit 2 and specific error message |
| B016 | flux-check-package.sh rejects --benches selector with exit 2 and specific error message |
| B017 | flux-check-package.sh rejects --all-targets selector with exit 2 and specific error message |
| B018 | flux-check-package.sh passes through valid options to cargo flux (e.g., --message-format) |
| B019 | flux-check-package.sh exits 2 when invoked with multiple unsupported selectors (batch rejection) |
| B020 | flux-check-package.sh surface-level failure: exits with cargo flux exit code when flux fails |

### Script: guard-zero-tests.sh (RRO-006, RRO-007)

| # | Behavior |
|---|----------|
| B021 | guard-zero-tests.sh exits 2 with usage message when invoked with no arguments |
| B022 | guard-zero-tests.sh exits 1 when cargo test selects 0 applicable tests |
| B023 | guard-zero-tests.sh exits 0 when cargo test selects non-zero applicable tests |
| B024 | guard-zero-tests.sh parses "N passed" (simple format) and reports count=N |
| B025 | guard-zero-tests.sh parses "N passed, M filtered out" and reports applicable=N |
| B026 | guard-zero-tests.sh handles "0 passed, M filtered out" as zero applicable (fail-closed) |
| B027 | guard-zero-tests.sh exits 1 when cargo test command itself fails (nonzero exit) |
| B028 | guard-zero-tests.sh exits 1 when cargo test output is unparseable |
| B029 | guard-zero-tests.sh detects "running 0 tests" as zero applicable |

### Script: loom-list.sh (RRO-011)

| # | Behavior |
|---|----------|
| B030 | loom-list.sh exits 0 and lists 5 known Loom models when xtask is available |
| B031 | loom-list.sh exits 1 when xtask loom integration is unavailable or fails |
| B032 | loom-list.sh exits 1 when enumerated model list is empty |

### Command: cargo fuzz (RRO-008, RRO-009)

| # | Behavior |
|---|----------|
| B033 | cargo fuzz list exits 0 and lists registered fuzz target names |
| B034 | cargo fuzz list produces a non-empty target list |
| B035 | cargo fuzz build --target x86_64-unknown-linux-gnu compiles all targets with exit 0 |
| B036 | cargo fuzz build with unsupported target triple fails with appropriate error |

### Command: Loom cfg execution (RRO-010)

| # | Behavior |
|---|----------|
| B037 | RUSTFLAGS="--cfg loom" cargo test compiles and executes loom model tests |

---

## 2. Trophy Allocation

### Static Analysis Tests (5)

| # | Test | Obligation |
|---|------|------------|
| S01 | `shellcheck` passes on scripts/kani-list.sh, flux-check-package.sh, guard-zero-tests.sh, loom-list.sh | All script obligations |
| S02 | All *.sh scripts have shebang `#!/usr/bin/env bash` and execute permission | All script obligations |
| S03 | scripts/kani-list.sh output JSON validates against kani-list.schema.json | RRO-001, RRO-002 |
| S04 | xtask/src/loom.rs compiles without warnings (`cargo clippy -p xtask`) | RRO-011 |
| S05 | .moon/tasks/kani.yml is valid YAML with required tasks | RRO-001, RRO-002, RRO-003 |

### Integration Tests (32)

| # | Test Name (function) | B# | Obligation |
|---|---------------------|----|------------|
| I01 | `kani_list_exits_2_with_usage_when_no_args` | B001 | RRO-001 |
| I02 | `kani_list_exits_1_when_cargo_kani_missing` | B002 | RRO-001 |
| I03 | `kani_list_produces_valid_json_for_vb_core_with_nonzero_harnesses` | B003 | RRO-001 |
| I04 | `kani_list_produces_valid_json_for_vb_runtime_with_nonzero_harnesses` | B004 | RRO-002 |
| I05 | `kani_list_exits_1_for_nonexistent_package` | B005 | RRO-001 |
| I06 | `kani_list_fails_closed_when_KANI_FEATURES_requests_undeclared_feature` | B006 | RRO-003 |
| I07 | `kani_list_succeeds_with_declared_KANI_FEATURES_passthrough` | B007 | RRO-003 |
| I08 | `kani_list_outputs_to_KANI_LIST_DIR_override` | B008 | RRO-001 |
| I09 | `kani_list_exits_1_on_empty_json_output` | B009 | RRO-001 |
| I10 | `kani_list_output_json_is_valid_json_by_python_json_tool` | B010 | RRO-001 |
| I11 | `flux_check_exits_2_with_usage_when_no_package` | B011 | RRO-004 |
| I12 | `flux_check_executes_cargo_flux_for_package_with_exit_0` | B012 | RRO-004 |
| I13 | `flux_check_rejects_lib_selector_with_exit_2_and_message` | B013 | RRO-005 |
| I14 | `flux_check_rejects_test_selector_with_exit_2_and_message` | B014 | RRO-005 |
| I15 | `flux_check_rejects_tests_selector_with_exit_2_and_message` | B015 | RRO-005 |
| I16 | `flux_check_rejects_benches_selector_with_exit_2_and_message` | B016 | RRO-005 |
| I17 | `flux_check_rejects_all_targets_selector_with_exit_2_and_message` | B017 | RRO-005 |
| I18 | `flux_check_passes_through_valid_flags_to_cargo_flux` | B018 | RRO-004 |
| I19 | `flux_check_rejects_multiple_unsupported_selectors` | B019 | RRO-005 |
| I20 | `flux_check_propagates_cargo_flux_failure_exit_code` | B020 | RRO-004 |
| I21 | `guard_zero_tests_exits_2_without_args` | B021 | RRO-006 |
| I22 | `guard_zero_tests_exits_1_when_zero_applicable_tests` | B022 | RRO-006 |
| I23 | `guard_zero_tests_exits_0_when_nonzero_applicable_tests` | B023 | RRO-007 |
| I24 | `guard_zero_tests_parses_simple_N_passed_format` | B024 | RRO-007 |
| I25 | `guard_zero_tests_parses_N_passed_M_filtered_format` | B025 | RRO-007 |
| I26 | `guard_zero_tests_detects_0_passed_M_filtered_as_zero` | B026 | RRO-006 |
| I27 | `guard_zero_tests_exits_1_on_cargo_test_nonzero_exit` | B027 | RRO-006 |
| I28 | `guard_zero_tests_exits_1_on_unparseable_output` | B028 | RRO-006 |
| I29 | `guard_zero_tests_detects_running_0_tests_as_zero` | B029 | RRO-006 |
| I30 | `loom_list_exits_0_and_lists_5_models` | B030 | RRO-011 |
| I31 | `loom_list_exits_1_when_xtask_unavailable` | B031 | RRO-011 |
| I32 | `loom_list_exits_1_when_model_list_empty` | B032 | RRO-011 |

### End-to-End Tests (3)

| # | Test Name | B# | Obligations |
|---|-----------|----|-------------|
| E01 | `moon_ci_verifier_tooling_pipeline_all_passes` | B001-B037 | All RRO-001 to RRO-011 |
| E02 | `multi_lane_evidence_smoke_every_lane_produces_output` | B003, B004, B012, B023, B030, B033, B035, B037 | All |
| E03 | `evidence_directory_audit_all_artifacts_present` | B008, B010, B012, B023, B030, B033 | All |

---

## 3. BDD Scenarios

### B001: kani-list.sh exits 2 with usage message when invoked with no arguments

```
Given: kani-list.sh is executable and on a PATH that includes cargo kani
When: invoked with zero arguments
Then: exit code is 2
And: stderr contains "usage:"
And: stdout is empty
```

Rust function name: `fn kani_list_exits_2_with_usage_when_no_args()`

---

### B002: kani-list.sh exits 1 when cargo kani is not on PATH

```
Given: cargo kani is NOT available on PATH
When: kani-list.sh is invoked with any package argument
Then: exit code is 1
And: stderr contains "cargo kani is required on PATH"
```

Rust function name: `fn kani_list_exits_1_when_cargo_kani_missing()`

---

### B003: kani-list.sh produces valid JSON inventory for vb_core with non-zero harness count

```
Given: cargo kani is available on PATH and vb_core exists in workspace
When: kani-list.sh vb_core is invoked
Then: exit code is 0
And: stdout contains "KANI_LIST_OK"
And: .evidence/kani-list/vb_core.json exists
And: JSON is valid with "standard-harnesses" field present
And: totals.standard-harnesses > 0 (currently 176)
And: stderr contains "[kani-list] package=vb_core"
```

Rust function name: `fn kani_list_produces_valid_json_for_vb_core_with_nonzero_harnesses()`

---

### B004: kani-list.sh produces valid JSON inventory for vb_runtime with non-zero harness count

```
Given: cargo kani is available on PATH and vb_runtime exists in workspace
When: kani-list.sh vb_runtime is invoked
Then: exit code is 0
And: stdout contains "KANI_LIST_OK"
And: .evidence/kani-list/vb_runtime.json exists
And: JSON is valid with "standard-harnesses" field present
And: totals.standard-harnesses > 0 (currently 6)
And: harnesses are in reentry_proofs.rs
```

Rust function name: `fn kani_list_produces_valid_json_for_vb_runtime_with_nonzero_harnesses()`

---

### B005: kani-list.sh exits 1 when package does not exist in workspace

```
Given: cargo kani is available on PATH
When: kani-list.sh is invoked with a package name not in workspace metadata (e.g., "nonexistent_package_xyz")
Then: exit code is 1
And: stderr indicates the package was not found
```

Rust function name: `fn kani_list_exits_1_for_nonexistent_package()`

---

### B006: kani-list.sh fails closed when KANI_FEATURES requests an undeclared feature

```
Given: cargo kani is available and vb_runtime exists but does NOT declare kani-diagnostic-codes
When: KANI_FEATURES=vb_runtime/kani-diagnostic-codes kani-list.sh vb_runtime is invoked
Then: exit code is 1 (fail-closed)
And: stderr contains cargo metadata resolution error
And: no evidence file is produced
```

Rust function name: `fn kani_list_fails_closed_when_KANI_FEATURES_requests_undeclared_feature()`

---

### B007: kani-list.sh succeeds with declared KANI_FEATURES feature passthrough

```
Given: cargo kani is available and vb_core declares kani-diagnostic-codes feature
When: KANI_FEATURES=vb_core/kani-diagnostic-codes kani-list.sh vb_core is invoked
Then: exit code is 0
And: stdout contains "KANI_LIST_OK"
And: valid JSON evidence file is produced
```

Rust function name: `fn kani_list_succeeds_with_declared_KANI_FEATURES_passthrough()`

---

### B008: kani-list.sh outputs to KANI_LIST_DIR override

```
Given: cargo kani is available
When: KANI_LIST_DIR=/tmp/kani-custom kani-list.sh vb_core is invoked
Then: exit code is 0
And: evidence file is written to /tmp/kani-custom/vb_core.json
And: file is valid JSON
```

Rust function name: `fn kani_list_outputs_to_KANI_LIST_DIR_override()`

---

### B009: kani-list.sh exits 1 when cargo kani list produces empty JSON file

```
Given: cargo kani produces an empty or zero-byte kani-list.json (simulated)
When: kani-list.sh vb_core is invoked
Then: exit code is 1
And: stderr contains "did not produce"
```

Rust function name: `fn kani_list_exits_1_on_empty_json_output()`

---

### B010: kani-list.sh validates output JSON with python3 -m json.tool

``` 
Given: cargo kani is available
When: kani-list.sh vb_core is invoked successfully
Then: the output JSON passes `python3 -m json.tool` validation
And: exit code is 0
```

Rust function name: `fn kani_list_output_json_is_valid_json_by_python_json_tool()`

---

### B011: flux-check-package.sh exits 2 with usage message when no package argument provided

```
Given: flux-check-package.sh is executable
When: invoked with zero arguments
Then: exit code is 2
And: stderr contains "usage:"
```

Rust function name: `fn flux_check_exits_2_with_usage_when_no_package()`

---

### B012: flux-check-package.sh executes cargo flux -p for a valid package with exit 0

```
Given: cargo flux is installed and available on PATH, vb_core exists in workspace
When: flux-check-package.sh vb_core is invoked
Then: exit code is 0
And: stdout indicates Flux compilation succeeded
```

Rust function name: `fn flux_check_executes_cargo_flux_for_package_with_exit_0()`

---

### B013: flux-check-package.sh rejects --lib selector

```
Given: flux-check-package.sh is executable
When: invoked as `flux-check-package.sh vb_core --lib`
Then: exit code is 2
And: stderr contains "unsupported cargo-flux target selector for installed cargo-flux: --lib"
And: cargo flux is NOT invoked (no compilation output on stdout)
```

Rust function name: `fn flux_check_rejects_lib_selector_with_exit_2_and_message()`

---

### B014-B017: Similar scenarios for --test, --tests, --benches, --all-targets

Each follows the same pattern as B013 with the respective selector text in the error message.

Rust function names:
- `fn flux_check_rejects_test_selector_with_exit_2_and_message()`
- `fn flux_check_rejects_tests_selector_with_exit_2_and_message()`
- `fn flux_check_rejects_benches_selector_with_exit_2_and_message()`
- `fn flux_check_rejects_all_targets_selector_with_exit_2_and_message()`

---

### B018: flux-check-package.sh passes through valid options to cargo flux

```
Given: cargo flux is available
When: flux-check-package.sh vb_core --message-format json is invoked
Then: exit code is 0
And: --message-format json is passed through to cargo flux
And: no selector rejection error appears
```

Rust function name: `fn flux_check_passes_through_valid_flags_to_cargo_flux()`

---

### B019: flux-check-package.sh exits 2 when invoked with multiple unsupported selectors

```
Given: flux-check-package.sh is executable
When: invoked as `flux-check-package.sh vb_core --lib --test`
Then: exit code is 2
And: the first encountered unsupported selector is reported in stderr
And: cargo flux is NOT invoked
```

Rust function name: `fn flux_check_rejects_multiple_unsupported_selectors()`

---

### B020: flux-check-package.sh propagates cargo flux failure exit code

```
Given: cargo flux is available but the target package has compilation errors
When: flux-check-package.sh <broken-package> is invoked
Then: exit code matches cargo flux nonzero exit code
And: error output from cargo flux is preserved on stderr/stdout
```

Rust function name: `fn flux_check_propagates_cargo_flux_failure_exit_code()`

---

### B021: guard-zero-tests.sh exits 2 with usage when no arguments

```
Given: guard-zero-tests.sh is executable
When: invoked with zero arguments
Then: exit code is 2
And: stderr contains usage information
```

Rust function name: `fn guard_zero_tests_exits_2_without_args()`

---

### B022: guard-zero-tests.sh exits 1 when cargo test selects 0 applicable tests

```
Given: cargo test is available, vb_core has test suite with real tests
When: guard-zero-tests.sh -- cargo test -p vb_core -- nonexistent_filter_xyz is invoked
Then: exit code is 1 (fail-closed)
And: stderr contains "FAIL: zero applicable tests detected (count=0)"
```

Rust function name: `fn guard_zero_tests_exits_1_when_zero_applicable_tests()`

---

### B023: guard-zero-tests.sh exits 0 when cargo test selects non-zero applicable tests

```
Given: cargo test is available, proptest tests exist in aggregate_resource_budget_properties_red
When: guard-zero-tests.sh -- cargo test -p vb_core --test aggregate_resource_budget_properties_red is invoked
Then: exit code is 0
And: stderr contains "PASS: N applicable tests executed" where N > 0
```

Rust function name: `fn guard_zero_tests_exits_0_when_nonzero_applicable_tests()`

---

### B024: guard-zero-tests.sh parses simple "N passed" format

```
Given: a cargo test output in the format "test result: ok. 5 passed; 0 failed; 0 ignored"
When: guard-zero-tests.sh processes this output
Then: reports count=5 applicable tests
And: exit code is 0
```

Rust function name: `fn guard_zero_tests_parses_simple_N_passed_format()`

---

### B025: guard-zero-tests.sh parses "N passed, M filtered out" format

```
Given: a cargo test output in the format "5 passed; 0 failed; 3 filtered out"
When: guard-zero-tests.sh processes this output
Then: reports count=5 applicable tests
And: exit code is 0
```

Rust function name: `fn guard_zero_tests_parses_N_passed_M_filtered_format()`

---

### B026: guard-zero-tests.sh detects "0 passed, M filtered out" as zero applicable

```
Given: a cargo test output in the format "0 passed; 0 failed; 10 filtered out"
When: guard-zero-tests.sh processes this output
Then: reports count=0 applicable tests
And: exit code is 1 (fail-closed)
```

Rust function name: `fn guard_zero_tests_detects_0_passed_M_filtered_as_zero()`

---

### B027: guard-zero-tests.sh exits 1 on cargo test command itself failing

```
Given: a cargo test command that exits with a nonzero code (e.g., compilation error)
When: guard-zero-tests.sh wraps this command
Then: exit code is 1
And: error originates from wrapped command failure, not from the guard
```

Rust function name: `fn guard_zero_tests_exits_1_on_cargo_test_nonzero_exit()`

---

### B028: guard-zero-tests.sh exits 1 on unparseable output

```
Given: cargo test produces output that does not match any known format
When: guard-zero-tests.sh attempts to parse it
Then: exit code is 1 (fail-closed)
And: stderr indicates parse failure
```

Rust function name: `fn guard_zero_tests_exits_1_on_unparseable_output()`

---

### B029: guard-zero-tests.sh detects "running 0 tests" as zero applicable

```
Given: cargo test output begins with "running 0 tests"
When: guard-zero-tests.sh processes this output
Then: reports zero applicable tests
And: exit code is 1
```

Rust function name: `fn guard_zero_tests_detects_running_0_tests_as_zero()`

---

### B030: loom-list.sh exits 0 and lists 5 known Loom models

```
Given: xtask is compiled and the loom subcommand is wired
When: loom-list.sh is invoked
Then: exit code is 0
And: stdout lists exactly 5 model names:
   journal_writer_queue, action_completion_cancel, timer_fired_cancel,
   shutdown_drain, bounded_queue
```

Rust function name: `fn loom_list_exits_0_and_lists_5_models()`

---

### B031: loom-list.sh exits 1 when xtask loom integration is unavailable

```
Given: xtask loom subcommand is not compiled or not wired
When: loom-list.sh is invoked
Then: exit code is 1
And: stderr indicates xtask unavailability
```

Rust function name: `fn loom_list_exits_1_when_xtask_unavailable()`

---

### B032: loom-list.sh exits 1 when enumerated model list is empty

```
Given: xtask loom returns no models (simulated by xtask returning empty)
When: loom-list.sh is invoked
Then: exit code is 1
And: stderr indicates empty model list
```

Rust function name: `fn loom_list_exits_1_when_model_list_empty()`

---

### B033: cargo fuzz list exits 0 and lists registered fuzz target names

```
Given: fuzz/Cargo.toml exists with [[bin]] entries
When: cargo fuzz list is invoked from workspace root
Then: exit code is 0
And: stdout contains a non-empty list of fuzz target names
And: each name corresponds to a [[bin]] entry in fuzz/Cargo.toml
```

Rust function name: `fn cargo_fuzz_list_exits_0_and_lists_target_names()`

---

### B034: cargo fuzz list produces non-empty target list

```
Given: fuzz/Cargo.toml exists with at least one [[bin]] entry
When: cargo fuzz list is invoked
Then: output contains at least one target name
And: total count >= current known count (57 or more)
```

Rust function name: `fn cargo_fuzz_list_produces_nonempty_target_count()`

---

### B035: cargo fuzz build compiles all targets with GNU triple

```
Given: fuzz/Cargo.toml has registered targets
When: cargo fuzz build --target x86_64-unknown-linux-gnu is invoked
Then: exit code is 0
And: compiled binaries exist under fuzz/target/x86_64-unknown-linux-gnu/release/
And: no sanitizer link errors on stderr
```

Rust function name: `fn cargo_fuzz_build_compiles_with_gnu_target()`

---

### B036: cargo fuzz build with unsupported triple fails

```
Given: an unsupported or nonexistent target triple (e.g., "x86_64-unknown-nonexistent")
When: cargo fuzz build --target x86_64-unknown-nonexistent is invoked
Then: exit code is non-zero
And: error indicates target is not available
```

Rust function name: `fn cargo_fuzz_build_fails_with_unsupported_target()`

---

### B037: Loom model tests compile and execute under cfg(loom)

```
Given: loom is a dev-dependency of vb_runtime, cfg(loom) gates exist
When: RUSTFLAGS="--cfg loom" cargo test -p vb_runtime --lib -- models::loom is invoked
Then: exit code is 0
And: test summary shows N passed (currently 13), many filtered out
And: no unresolved crate errors
And: "running" line shows non-zero test count
```

Rust function name: `fn loom_model_tests_compile_and_execute_under_cfg_loom()`

---

## 4. Proptest Invariants

Since this is a tooling bead with no pure Rust Calc layer, proptest invariants apply to script argument handling and output parsing. These are expressed as property tests on the script behavior.

### Proptest P01: kani-list.sh accepts any valid workspace package

```
Invariant: For any declared package name in cargo metadata, kani-list.sh <pkg> exits with 0 or 1
           (exits 0 if Kani harnesses exist, exits 1 with cargo error if no harnesses).
Strategy: Filter cargo metadata --no-deps --format-version 1 for package names.
Anti-invariant: Random strings that are not package names must exit non-zero.
```

### Proptest P02: flux-check-package.sh rejects all unsupported selectors

```
Invariant: For any combination of --lib, --test, --tests, --benches, --all-targets flags,
           flux-check-package.sh exits 2 before invoking cargo flux.
Strategy: Generate any non-empty subset of unsupported selector flags.
Anti-invariant: Valid flags like --message-format must be passed through.
```

### Proptest P03: guard-zero-tests.sh roundtrips with known test output formats

```
Invariant: For any output matching the known cargo test output patterns ("N passed" or
           "N passed, M filtered out"), the guard correctly classifies zero vs non-zero.
Strategy: Generate (N, M) pairs where N, M in [0, u32::MAX] and check:
           N=0 → exit 1, N>0 → exit 0.
Anti-invariant: Malformed output that does not match any pattern must exit 1.
```

### Proptest P04: kani-list.sh output JSON is always valid

```
Invariant: When kani-list.sh exits 0, the output file is always valid JSON.
Strategy: Run kani-list.sh against all declared packages and validate each JSON output.
Anti-invariant: A file written by the script that is not valid JSON must correspond to exit != 0.
```

### Proptest P05: flux-check-package.sh exit codes are deterministic

```
Invariant: For any fixed input, flux-check-package.sh produces the same exit code on repeated runs.
Strategy: Run same command twice, compare exit codes. Exit code 0 is stable; exit code 2 is stable.
Anti-invariant: Exit code that changes between runs in a clean environment indicates nondeterminism.
```

### Proptest P06: fuzz target list is prefix-closed under valid targets

```
Invariant: If cargo fuzz list succeeds, every listed target exists as a [[bin]] entry in
           fuzz/Cargo.toml and can be individually referenced.
Strategy: Parse cargo fuzz list output, cross-check each target name against fuzz/Cargo.toml.
Anti-invariant: A target name from cargo fuzz list that is absent from fuzz/Cargo.toml.
```

---

## 5. Fuzz Targets

Every script boundary that accepts arguments and parses strings is a fuzz target candidate.

### Fuzz Target F01: kani-list.sh argument parsing

```
Function: kani-list.sh main argument parser
Input type: str (space-separated CLI arguments)
Risk: Argument injection, special character handling, path traversal via KANI_LIST_DIR
Corpus seeds:
  - "" (no args)
  - "vb_core"
  - "vb_core vb_runtime"
  - KANI_LIST_DIR="/tmp/../../evil" bash scripts/kani-list.sh vb_core
  - Package names with special chars: "vb_core; rm -rf /" 
  - Very long package names (exceeding MAX_ARG_STRLEN)
  - Commas in KANI_FEATURES: "vb_core/feature1,vb_runtime/feature2"
```

### Fuzz Target F02: flux-check-package.sh selector rejection

```
Function: flux-check-package.sh selector loop (lines 12-19)
Input type: str (CLI arguments after package name)
Risk: Argument injection bypassing the selector guard, unicode in error messages
Corpus seeds:
  - "--LiB" (different case)
  - "--lib " (trailing space)
  - "-l" (abbreviated)
  - "" (empty string after package)
  - "--lib\x00test" (null byte injection)
  - Very long selector strings
  - Mixed valid/invalid: "--message-format human --lib"
```

### Fuzz Target F03: guard-zero-tests.sh output parser

```
Function: guard-zero-tests.sh output parser (count extraction)
Input type: str (cargo test stdout/stderr)
Risk: Regex panic on pathological input, integer overflow on count extraction, incorrect classification
Corpus seeds:
  - "test result: ok. 0 passed; 0 failed; 0 ignored"
  - "test result: ok. 4294967296 passed" (u32 overflow attempt)
  - Truncated output: "test result: ok." (no pass count)
  - UTF-8 BOM prefix with valid output
  - "running 18446744073709551615 tests" (u64 overflow)
  - Binary garbage after valid test output
```

### Fuzz Target F04: loom-list.sh xtask integration

```
Function: loom-list.sh xtask output parser
Input type: str (xtask stdout output)
Risk: Missing expected model names, partial output, encoding issues
Corpus seeds:
  - Empty output
  - "Available models:" with no model names
  - Models with special chars: "model/with/slash"
  - Unicode model names
  - Truncated output mid-model-name
```

---

## 6. Mutation Checkpoints

### Threshold: ≥90% mutation kill rate on all script-level behaviors.

Each critical branch must be caught by at least one test. If a branch is mutated (removed, inverted, substituted) and no test fails, the test suite is inadequate.

| # | Mutation Target | Must Be Caught By |
|---|----------------|-------------------|
| M01 | kani-list.sh `if [ "$#" -eq 0 ]` removed → usage never shown | I01 |
| M02 | kani-list.sh `if ! cargo kani --version` removed → missing tool not detected | I02 |
| M03 | kani-list.sh `if [ ! -s "$package_dir/kani-list.json" ]` inverted → empty JSON accepted | I09 |
| M04 | kani-list.sh `cargo metadata` matching logic: wrong package match quorum | I05 |
| M05 | flux-check-package.sh `if [ "$#" -lt 1 ]` removed → missing arg accepted | I11 |
| M06 | flux-check-package.sh selector case `--lib)` removed → --lib no longer rejected | I13 |
| M07 | flux-check-package.sh selector case `--test)` removed → --test no longer rejected | I14 |
| M08 | flux-check-package.sh selector case `--tests)` removed → --tests no longer rejected | I15 |
| M09 | flux-check-package.sh selector case `--benches)` removed → --benches no longer rejected | I16 |
| M10 | flux-check-package.sh selector case `--all-targets)` removed → --all-targets no longer rejected | I17 |
| M11 | guard-zero-tests.sh count extraction: `0` vs `>0` comparison inverted | I22, I23 |
| M12 | guard-zero-tests.sh "running 0 tests" pattern removed | I29 |
| M13 | guard-zero-tests.sh "filtered out" subtraction logic removed or wrong | I26 |
| M14 | guard-zero-tests.sh unknown output format handling removed → parse failure accepted as pass | I28 |
| M15 | guard-zero-tests.sh cargo test nonzero exit passthrough removed | I27 |
| M16 | loom-list.sh empty model list check reversed or removed | I32 |
| M17 | loom-list.sh xtask failure detection removed | I31 |
| M18 | kani-list.sh `KANI_FEATURES` passthrough logic removed → features never activated | I06, I07 |
| M19 | kani-list.sh `KANI_LIST_DIR` override removed → always writes to default | I08 |
| M20 | kani-list.sh `python3 -m json.tool` validation removed → invalid JSON accepted | I10 |

---

## 7. Proof/Refinement Coverage Matrix

Each test in this plan covers one or more refinement obligations (RRO-001 through RRO-011). The matrix below maps obligations to their test coverage.

| Obligation | Verifier | Behaviors | Unit Tests | Integration Tests | Proptest | E2E Tests | Static Tests | Fuzz Targets |
|------------|----------|-----------|------------|-------------------|----------|-----------|--------------|-------------|
| RRO-001 | kani | B001-B010 | 0 | I01-I10 | P01, P04 | E01, E02, E03 | S01, S02, S03, S05 | F01 |
| RRO-002 | kani | B004 | 0 | I04 | — | E01, E02, E03 | S01, S02, S05 | — |
| RRO-003 | kani | B006-B007 | 0 | I06, I07 | — | E01 | S05 | — |
| RRO-004 | flux-rs | B011-B012, B018, B020 | 0 | I11, I12, I18, I20 | P05 | E01, E02, E03 | S01, S02 | F02 |
| RRO-005 | flux-rs | B013-B019 | 0 | I13-I17, I19 | P02, P05 | E01 | S01, S02 | F02 |
| RRO-006 | proptest | B021-B022, B026-B029 | 0 | I21, I22, I26-I29 | P03 | E01, E03 | S01, S02 | F03 |
| RRO-007 | proptest | B023-B025 | 0 | I23-I25 | P03 | E01, E02, E03 | S01, S02 | — |
| RRO-008 | cargo-fuzz | B033-B034 | 0 | I33, I34 | P06 | E01, E02, E03 | — | — |
| RRO-009 | cargo-fuzz | B035-B036 | 0 | I35, I36 | — | E01 | — | — |
| RRO-010 | loom | B037 | 0 | I37 | — | E01, E02, E03 | S04 | — |
| RRO-011 | loom | B030-B032 | 0 | I30-I32 | — | E01, E02, E03 | S01, S02 | F04 |
| **TOTAL** | — | **37** | **0** | **37** | **6** | **3** | **5** | **4** |

### Combinatorial Coverage Matrix

### Script: kani-list.sh

| Scenario | Input Class | Expected Output | Test Layer | Test ID |
|----------|-------------|-----------------|------------|---------|
| no args | zero arguments | exit 2, stderr "usage:" | integration | I01 |
| missing tool | cargo kani not on PATH | exit 1, stderr "required" | integration | I02 |
| happy: vb_core | "vb_core" (known package with harnesses) | exit 0, valid JSON, 176 harnesses | integration | I03, I10 |
| happy: vb_runtime | "vb_runtime" (known package with harnesses) | exit 0, valid JSON, 6 harnesses | integration | I04 |
| nonexistent package | "nonexistent_pkg" (unknown) | exit 1, error message | integration | I05 |
| undeclared KANI_FEATURES | KANI_FEATURES=vb_runtime/kani-diagnostic-codes | exit 1, cargo metadata error | integration | I06 |
| declared KANI_FEATURES | KANI_FEATURES=vb_core/kani-diagnostic-codes | exit 0, valid JSON | integration | I07 |
| KANI_LIST_DIR override | KANI_LIST_DIR set | output in custom dir | integration | I08 |
| empty JSON output | cargo kani list produces empty file | exit 1 | integration | I09 |
| any package → valid JSON | proptest across all workspace packages | valid JSON or exit != 0 | proptest | P01, P04 |
| shellcheck | static | no errors, no warnings | static | S01 |
| shebang + execute | static | #!/usr/bin/env bash, +x | static | S02 |
| JSON schema | static | validates against schema | static | S03 |

### Script: flux-check-package.sh

| Scenario | Input Class | Expected Output | Test Layer | Test ID |
|----------|-------------|-----------------|------------|---------|
| no args | zero arguments | exit 2, stderr "usage:" | integration | I11 |
| happy: vb_core | "vb_core" | exit 0, compilation output | integration | I12 |
| reject --lib | "vb_core --lib" | exit 2, stderr "unsupported ... --lib" | integration | I13 |
| reject --test | "vb_core --test" | exit 2, stderr "unsupported ... --test" | integration | I14 |
| reject --tests | "vb_core --tests" | exit 2, stderr "unsupported ... --tests" | integration | I15 |
| reject --benches | "vb_core --benches" | exit 2, stderr "unsupported ... --benches" | integration | I16 |
| reject --all-targets | "vb_core --all-targets" | exit 2, stderr "unsupported ... --all-targets" | integration | I17 |
| valid passthrough | "vb_core --message-format json" | exit 0, flag passed through | integration | I18 |
| multiple selectors | "vb_core --lib --test" | exit 2, rejects first | integration | I19 |
| flux failure | package with compile error | exit != 0, error preserved | integration | I20 |
| any selector subset | proptest across unsupported flag subsets | exit 2 always | proptest | P02 |
| deterministic | same input twice | same exit code both times | proptest | P05 |
| shellcheck | static | no errors | static | S01 |
| shebang + execute | static | #!/usr/bin/env bash, +x | static | S02 |

### Script: guard-zero-tests.sh

| Scenario | Input Class | Expected Output | Test Layer | Test ID |
|----------|-------------|-----------------|------------|---------|
| no args | zero arguments | exit 2, usage | integration | I21 |
| zero applicable | cargo test with nonexistent filter | exit 1, "zero applicable" | integration | I22 |
| nonzero applicable | cargo test with real proptest suite | exit 0, "PASS: N applicable" | integration | I23 |
| "N passed" format | output "5 passed; 0 failed" | count=5, exit 0 | integration | I24 |
| "N passed, M filtered" | output "5 passed; 3 filtered" | count=5, exit 0 | integration | I25 |
| "0 passed, M filtered" | output "0 passed; 10 filtered" | count=0, exit 1 | integration | I26 |
| cargo test nonzero exit | compilation failure | exit 1, passthrough | integration | I27 |
| unparseable output | random string output | exit 1, parse failure | integration | I28 |
| "running 0 tests" | "running 0 tests" prefix | zero applicable, exit 1 | integration | I29 |
| any (N, M) pair | proptest across all N, M >= 0 | N=0→exit1, N>0→exit0 | proptest | P03 |
| shellcheck | static | no errors | static | S01 |
| shebang + execute | static | #!/usr/bin/env bash, +x | static | S02 |

### Script: loom-list.sh

| Scenario | Input Class | Expected Output | Test Layer | Test ID |
|----------|-------------|-----------------|------------|---------|
| happy path | xtask available | exit 0, 5 model names listed | integration | I30 |
| xtask unavailable | xtask not compiled | exit 1, error message | integration | I31 |
| empty model list | xtask returns no models | exit 1, empty list error | integration | I32 |
| shellcheck | static | no errors | static | S01 |
| shebang + execute | static | #!/usr/bin/env bash, +x | static | S02 |

### Command: cargo fuzz

| Scenario | Input Class | Expected Output | Test Layer | Test ID |
|----------|-------------|-----------------|------------|---------|
| list targets | cargo fuzz list | exit 0, non-empty list | integration | I33, I34 |
| build GNU | cargo fuzz build --target x86_64-unknown-linux-gnu | exit 0, binaries exist | integration | I35 |
| unsupported target | cargo fuzz build --target bad-triple | exit != 0 | integration | I36 |
| prefix-closed | proptest: each listed target exists in fuzz/Cargo.toml | all targets match | proptest | P06 |

### Command: Loom tests

| Scenario | Input Class | Expected Output | Test Layer | Test ID |
|----------|-------------|-----------------|------------|---------|
| compile+execute | RUSTFLAGS="--cfg loom" cargo test -p vb_runtime --lib -- models::loom | exit 0, 13 passed | integration | I37 |
| xtask compiles | cargo clippy -p xtask | no warnings | static | S04 |

### End-to-End: Multi-lane pipeline

| Scenario | Input Class | Expected Output | Test Layer | Test ID |
|----------|-------------|-----------------|------------|---------|
| all lanes pass | moon ci verifier-tooling | exit 0, all tasks green | e2e | E01 |
| every lane produces output | run each lane individually | each produces non-empty evidence | e2e | E02 |
| evidence dir audit | ls .evidence/* | expected artifact files present | e2e | E03 |

---

## 8. Coverage Targets by Obligation

| Obligation | Test IDs | Behavior Count |
|------------|----------|----------------|
| RRO-001 (kani vb_core) | I01, I02, I03, I05, I08, I09, I10, S01, S02, S03, S05, E01, E02, E03 | 10 behaviors (B001-B010) |
| RRO-002 (kani vb_runtime) | I04, S01, S02, S05 | 1 behavior (B004) |
| RRO-003 (kani feature gate) | I06, I07, S05 | 2 behaviors (B006-B007) |
| RRO-004 (flux package smoke) | I11, I12, I18, I20, S01, S02 | 4 behaviors (B011-B012, B018, B020) |
| RRO-005 (flux selector rejection) | I13, I14, I15, I16, I17, I19, S01, S02 | 7 behaviors (B013-B019) |
| RRO-006 (proptest zero-test) | I21, I22, I26, I27, I28, I29, S01, S02 | 5 behaviors (B021-B022, B026-B029) |
| RRO-007 (proptest non-zero) | I23, I24, I25, S01, S02 | 3 behaviors (B023-B025) |
| RRO-008 (fuzz target reg) | I33, I34, P06 | 2 behaviors (B033-B034) |
| RRO-009 (fuzz GNU build) | I35, I36 | 2 behaviors (B035-B036) |
| RRO-010 (loom compile) | I37, S04 | 1 behavior (B037) |
| RRO-011 (loom enumeration) | I30, I31, I32, S01, S02 | 3 behaviors (B030-B032) |

---

## 9. Implementation Notes for Test-Writer (State 9)

### Test framework

- Use bash-based tests invoked via `bash -c` or a minimal bash test runner.
- Tests must be runnable from the workspace root.
- Tests must be independent (no shared state, no ordering dependency).
- Each test file should correspond to one script/command group.

### Required assertions

- Every test must assert EXACT exit code, not just `< 0` vs `== 0`.
- Output assertions must check for substring presence, not exact match (tooling output varies by environment).
- Count assertions must check the actual numeric value extracted from output.
- No test may assert only `result.success()` or `result.failure()` without the specific code.

### Test isolation

- Kani tests may require `cargo kani` to be installed. Tests should check for availability and skip with a clear message if unavailable, rather than silently pass.
- Flux tests may require `cargo flux` to be installed. Same skip pattern.
- Fuzz tests may require `cargo fuzz` to be installed. Same skip pattern.
- Tests in CI must skip gracefully when tools are not installed but must not produce false passes.

### Evidence directory

- Tests that write evidence files must use a temporary directory (`mktemp -d`) and clean up.
- Do NOT pollute `.evidence/` from test runs.

---

## 10. Open Questions

1. **OTL-001**: Should the tests include a placeholder for the `verify-kani-inventory` moon task that is planned for State 11, or defer that test entirely to the implementation state?
   - **Answer for now**: Defer to State 11. Test stubs may reference the task name but should not assert its existence at State 8.

2. **OTL-002**: `guard-zero-tests.sh` and `loom-list.sh` were created in the proof-writer workspace (State 5) but do not yet exist in the source checkout. Tests targeting these scripts should use the source checkout copy once State 11 places them there. Until then, tests may reference these scripts by expected path only.
   - **Answer for now**: Write tests against the expected paths. The test-writer at State 9 should gate these tests on file existence.

3. **OTL-003**: The pipefragility issue in `guard-zero-tests.sh` (`set -euo pipefail` interaction with grep) is documented as FIND-SHVXY-001. Should tests include a negative test for the fragile pipe behavior?
   - **Answer for now**: Yes, include a defensive test that the script does not exit incorrectly on grep's natural "no match" behavior (I28 already covers unparseable output, which includes grep returning exit 1).

4. **OTL-004**: `cargo fuzz list` currently returns 57 targets. The tests assert a non-empty list but should not hardcode the exact count (it may grow). Is asserting "count >= 57" acceptable?
   - **Answer for now**: Yes. Assert `count > 0` as the primary invariant. Optionally assert `count >= 57` as a regression check that can be bumped when new targets are added.

5. **OTL-005**: The proptest invariants P03 and P04 require a test framework that can generate and inject controlled cargo test output. Should these be deferred to a future state where the implementation supports mock injection?
   - **Answer for now**: Write proptest invariants as documentation/strategy, but the actual test implementation may substitute controlled input files or use `printf` pipelines to simulate cargo test output without mocking cargo itself.
