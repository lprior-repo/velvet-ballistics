# Martin Fowler Test Plan: Atomize xtask Section 77 Command-Center Gates

## Domain

- **System under test**: `xtask/src/` — the atomized xtask command wrappers for Section 77 quality gates
- **Scope**: All ai-fast, ai-deep, and ai-release profile commands plus their evidence-bundle infrastructure
- **Key types**: `GateEvidence`, `WhyFailed`, `GateStatus`, `GateProfile`, `Error`

---

## Happy Path Tests

### test_ai_fast_profile_emits_evidence_bundle_with_command_statuses

**Given**: The xtask workspace is built and all ai-fast constituent tools (fmt, check, clippy, nextest, forbidden-scan, hotpath-scan) are available in the toolchain

**When**: `cargo xtask ai-fast --bead vb-test` is executed

**Then**:
- Exit code is 0
- A YAML document is emitted to `.evidence/vb-test/ai-fast.yaml`
- The YAML contains entries for each of the 6 ai-fast gates
- Each entry has `gate_name`, `command`, `exit_code`, and `status` fields
- All 6 entries have `status: pass` (assuming clean workspace)

### test_ai_deep_profile_emits_evidence_bundle

**Given**: The xtask workspace is built and ai-deep tools (miri, mutants, llvm-cov, fuzz-build) are available

**When**: `cargo xtask ai-deep --bead vb-test` is executed

**Then**:
- Exit code is 0 on clean workspace
- Evidence bundle written to `.evidence/vb-test/ai-deep.yaml`
- Bundle contains 4 gate entries with required fields

### test_ai_release_profile_delegates_to_moon_just

**Given**: The xtask workspace is built and moon + just are available

**When**: `cargo xtask ai-release --bead vb-test` is executed

**Then**:
- Exit code reflects aggregate pass/fail of all constituent gates
- Evidence bundle written to `.evidence/vb-test/ai-release.yaml`
- Bundle contains entries for check, test, supply-chain, miri, fuzz-smoke, coverage, mutants-smoke, bench-build, feature-powerset, source-length, maxperf gates

### test_bead_flag_creates_evidence_directory

**Given**: The `.evidence/vb-test/` directory does not exist

**When**: `cargo xtask ai-fast --bead vb-test` is executed

**Then**:
- The directory `.evidence/vb-test/` is created
- Evidence YAML file is written inside it

### test_evidence_round_trip_through_yaml

**Given**: A valid `GateEvidence` struct with all required fields populated

**When**: The evidence is serialized to YAML and deserialized

**Then**:
- All fields are preserved (kind, gate_name, command, exit_code, log, status, why_failed)
- The deserialized struct equals the original

### test_why_failed_hint_populated_for_failed_gate

**Given**: A gate has returned exit_code != 0

**When**: `explain_failure(evidence)` is called

**Then**:
- The returned `WhyFailed` has non-empty `hint` field
- The `repair_command` field contains the exact command to run to reproduce/fix the failure

---

## Error Path Tests

### test_missing_crash-lab_or_differential-test_evidence_fails_ai_release

**Given**: The ai-release profile requires evidence from the crash-lab and diff-test gates

**When**: `cargo xtask ai-release --bead vb-test` is run but crash-lab evidence is absent

**Then**:
- Exit code is 1
- A `MissingEvidence` diagnostic is emitted for the crash-lab gate

### test_why-failed_reports_the_failing_gate_and_repair_hint

**Given**: A gate (e.g., clippy) has failed

**When**: `cargo xtask why-failed logs/ai-check.yaml` is executed

**Then**:
- Output contains the failing gate name
- Output contains a `repair_command` hint pointing to the failing subcommand
- Output contains a `hint` explaining why the gate failed

### test_gate_failed_propagates_with_exit_code

**Given**: A gate subprocess returns non-zero exit code

**When**: `run_gate` is called with that gate's command

**Then**:
- `GateFailed { gate, exit_code, log }` error is returned
- The exit_code field matches the subprocess exit code

### test_unknown_subcommand_returns_error

**Given**: An unknown subcommand name is passed to xtask

**When**: `cargo xtask does-not-exist` is executed

**Then**:
- Exit code is 1
- Error message mentions the unknown subcommand name

### test_yaml_serialization_failure_returns_typed_error

**Given**: A GateEvidence struct contains a field that fails YAML serialization

**When**: Serialization is attempted

**Then**:
- `YamlSerializationFailed` error is returned with the gate name and cause

### test_evidence_write_failure_returns_typed_error

**Given**: The evidence directory is not writable

**When**: `run_gate` attempts to write the evidence bundle

**Then**:
- `EvidenceWriteFailed` error is returned with the path and cause

### test_bead_directory_creation_failure_returns_typed_error

**Given**: The parent of `.evidence/<bead>/` does not exist and cannot be created

**When**: `cargo xtask ai-fast --bead vb-test` is executed

**Then**:
- `BeadDirectoryCreationFailed` error is returned

---

## Edge Case Tests

### test_empty_gate_name_in_evidence

**Given**: A gate name is an empty string

**When**: GateEvidence is constructed and serialized

**Then**:
- Serialization succeeds (empty string is valid YAML)
- Deserialization produces a GateEvidence with empty gate_name

### test_evidence_log_path_with_special_characters

**Given**: A log file path contains spaces, quotes, and newlines

**When**: GateEvidence is serialized and deserialized

**Then**:
- Round-trip preserves the path correctly
- YAML output is valid

### test_multiple_gates_fail_in_single_profile

**Given**: Two gates in ai-fast profile fail

**When**: `cargo xtask ai-fast` is executed

**Then**:
- Exit code is 1
- Evidence bundle contains both failures
- why-failed block is populated for each failed gate

### test_profile_timeout_enforced

**Given**: A gate has a 1-second timeout configured

**When**: The gate command blocks indefinitely

**Then**:
- `GateTimeout { gate, duration_secs: 1 }` error is returned
- Exit code is 1

### test_bead_flag_without_evidence_scope

**Given**: `cargo xtask ai-fast` is run without --bead flag

**When**: The command completes

**Then**:
- Evidence is written to stdout as YAML
- No `.evidence/` directory is created
- Exit code reflects pass/fail

### test_all_gates_pass_produces_pass_status_array

**Given**: All gates in ai-fast pass

**When**: `cargo xtask ai-fast` is executed

**Then**:
- All status fields in the evidence bundle are `Pass`
- No why-failed block is present in any entry

### test_skipped_gate_has_skipped_status

**Given**: A gate is intentionally skipped (e.g., miri on unsupported platform)

**When**: `cargo xtask ai-deep` is executed

**Then**:
- The skipped gate entry has `status: Skipped { reason: "..." }`
- Overall exit code may still be 0 (skipped gates do not fail the profile)

---

## Contract Verification Tests

### test_precondition_workspace_gate_inventory_documented

**Given**: Section 77 MASTER.md section

**When**: The xtask source is inspected

**Then**: All 28 gates from Section 77.1 + 77.2 + 77.3 have corresponding match arms in the Commands enum or an explicit blocker comment

### test_invariant_fail_closed_missing_evidence

**Given**: An evidence file for a required gate does not exist

**When**: `validate_evidence_dir` is called

**Then**:
- `MissingEvidence` error is returned
- No evidence file is silently created or bypassed

### test_invariant_no_panic_in_gate_wrapper

**Given**: The xtask binary is running

**When**: Any gate command is executed (passing, failing, or timing out)

**Then**:
- No `panic!`, `unwrap!`, or `expect!` is reached
- All errors are returned as `Result<_, Error>`

### test_invariant_deterministic_evidence

**Given**: A clean workspace and fixed toolchain version

**When**: `cargo xtask ai-fast --bead vb-test` is executed twice in sequence

**Then**:
- Both evidence bundles have identical YAML content
- Exit codes are identical

### test_invariant_structured_output_only

**Given**: The xtask command is running

**When**: Any output is emitted to stdout

**Then**:
- The output is valid YAML (parseable by any YAML parser)
- No raw tool output (fmt, clippy, etc.) appears on stdout (it goes to log files)

---

## Integration / End-to-End Tests

### test_full_pipeline_ai_fast_with_real_tools

**Given**: A dirty working tree (some formatting errors)

**When**: `cargo xtask ai-fast --bead vb-e2e` is executed

**Then**:
- fmt gate fails with non-zero exit
- The evidence bundle records the failure
- why-failed hint points to `cargo +nightly fmt --all`
- Exit code is 1

### test_full_pipeline_ai_release_from_clean_state

**Given**: A clean working tree with no missing evidence

**When**: `cargo xtask ai-release --bead vb-e2e` is executed

**Then**:
- Exit code is 0
- All 11 ai-release gate entries are present in the evidence bundle
- All have status Pass

### test_contract_alignment

**Given**: Implementation is complete

**When**: All tests from this plan are run

**Then**:
- Every contract clause has at least one failing test if the implementation does not match
- Every test name maps to a specific contract clause

---

## Test Execution Order

1. **Phase 0** (research — no tests): Read all research files
2. **Phase 1** (tests-first): Write all tests listed above; they must compile and fail before implementation
3. **Phase 2** (implementation): Implement just enough to make tests pass
4. **Phase 3** (verification): Run `moon run :ci`; all tests must green
5. **Phase 4** (integration): Run full `cargo xtask ai-release --bead vb-e2e` from clean state
