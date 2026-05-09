# Martin Fowler Test Plan: verify Hero Command and VerificationReport

## Test Naming Convention

Test names follow the pattern: `test_<scenario>_<when>_<then>`

## Happy Path Tests

### test_verify_succeeds_for_a_fixture_with_valid_workflow_in_text_mode
**Given**: A valid workflow YAML file at `tests/fixtures/valid/minimal.yaml`
**When**: The operator runs `velvet-ballastics verify tests/fixtures/valid/minimal.yaml` with default (standard) profile
**Then**: Exit code is 0; output contains "verified"; `VerificationReport` fields are all present and non-empty

### test_verify_succeeds_for_a_fixture_with_valid_source_ir_generated_code_digest_and_rep
**Given**: A valid workflow YAML fixture
**When**: `verify` is invoked with `--format json`
**Then**: A parseable `VerificationReport` JSON object is emitted with `profile`, `artifact.digest`, `artifact.source_digest_hex`, `artifact.ir_digest_hex`, `artifact.node_count`, `replay.gates_passed`, `durability`, `repair_hints`, `exit_code`

### test_verify_emits_verification_report_in_jsonl_mode
**Given**: A valid workflow YAML file
**When**: The operator runs `velvet-ballastics verify workflow.yaml --jsonl`
**Then**: One valid JSON object is printed per line; each object is parseable and contains all `VerificationReport` fields

### test_verify_report_contains_profile
**Given**: A valid workflow YAML file
**When**: `verify --profile quick` is invoked
**Then**: The report's `profile` field equals `"quick"`

### test_verify_report_contains_artifact_evidence
**Given**: A valid workflow YAML file
**When**: `verify` is invoked
**Then**: The report's `artifact` field contains `source_digest_hex` (64 hex chars), `ir_digest_hex` (64 hex chars), `node_count > 0`, and `passed_checks` (non-empty)

### test_verify_report_contains_replay_evidence
**Given**: A valid workflow YAML file
**When**: `verify` is invoked
**Then**: The report's `replay` field contains `gates_passed` (non-empty), `gate_sequence`, and `replay_safe: true`

### test_verify_report_contains_durability_evidence
**Given**: A valid workflow YAML file
**When**: `verify` is invoked with `--profile full`
**Then**: The report's `durability` field contains `profile: "full"` and `durable: false` (verify does not write journal)

### test_verify_report_contains_repair_hints
**Given**: A valid workflow YAML file with all gates passing
**When**: `verify` is invoked
**Then**: The report's `repair_hints` field is an empty array

### test_verify_report_contains_exit_code
**Given**: A valid workflow YAML file
**When**: `verify` is invoked
**Then**: The report's `exit_code` field equals `0` (u8)

## Error Path Tests

### test_verify_fails_closed_when_durability_evidence_is_missing
**Given**: A workflow artifact that would require durability evidence for strict mode
**When**: `verify --profile full` is run but no journal exists for the artifact
**Then**: Exit code is 2 (`VerificationFailed`); `repair_hints` contains a non-empty hint citing the durability gate

### test_verify_returns_the_documented_nonzero_exit_code_when_replay_evidence_fails
**Given**: An artifact with incomplete replay evidence
**When**: `verify` detects replay divergence
**Then**: Exit code is 8 (`ReplayDivergence`); the error message appears in both text and JSON output

### test_verify_returns_validation_failed_on_yaml_parse_error
**Given**: An invalid YAML file (e.g., `tests/fixtures/invalid/invalid_missing_when.yaml`)
**When**: `velvet-ballastics verify invalid.yaml` is invoked
**Then**: Exit code is 1; error message is present in both text and JSON output

### test_verify_returns_verification_failed_on_ir_validation_error
**Given**: A YAML file that parses but produces an IR validation error
**When**: `verify` is invoked
**Then**: Exit code is 2; repair hint cites "IrValidation" gate

### test_verify_returns_verification_failed_on_budget_policy_error
**Given**: A workflow that exceeds budget policy bounds
**When**: `verify --profile full` is invoked
**Then**: Exit code is 2; repair hint cites "BudgetPolicy" gate

### test_verify_returns_storage_error_on_storage_failure
**Given**: A workflow file path that causes a storage error
**When**: `verify` is invoked and storage is unavailable
**Then**: Exit code is 5 (`StorageError`); error message appears in both text and JSON

### test_verify_returns_replay_divergence_on_replay_error
**Given**: An artifact with mismatched action ABI digest
**When**: `verify` is invoked
**Then**: Exit code is 8 (`ReplayDivergence`); repair hint cites "Replay" gate

## Edge Case Tests

### test_verify_handles_empty_file_gracefully
**Given**: An empty workflow file
**When**: `verify empty.yaml` is invoked
**Then**: Exit code is 1; error message is non-empty; no panic

### test_verify_handles_1mb_workflow_file
**Given**: A workflow YAML file at the 1 MiB limit
**When**: `verify large.yaml` is invoked
**Then**: Exit code is 0 (if valid) or 1/2 (if invalid); no memory allocation failure

### test_verify_handles_deeply_nested_workflow
**Given**: A workflow with nesting depth of 8 (language limit)
**When**: `verify nested.yaml` is invoked
**Then**: `verify` completes without stack overflow; correct exit code

### test_verify_repair_hint_cites_gate_name
**Given**: A YAML file producing a `BudgetPolicy` error
**When**: `verify --profile full` is invoked
**Then**: The repair hint's `gate` field equals `"BudgetPolicy"`

### test_verify_repair_hint_cites_bead_reference_when_available
**Given**: A YAML file producing a known error with an associated bead
**When**: `verify` is invoked
**Then**: The repair hint's `bead_reference` field is non-empty when a bead exists for the failing gate

### test_verify_repair_hints_empty_when_all_gates_pass
**Given**: A fully valid workflow file
**When**: `verify workflow.yaml` is invoked
**Then**: `repair_hints` is an empty array; no repair hints emitted

## Contract Verification Tests

### test_precondition_no_auth_required
**Given**: Any workflow file
**When**: `verify` is invoked without any authentication credentials
**Then**: `verify` proceeds without error; no auth-related error is returned

### test_postcondition_report_contains_all_certificate_fields
**Given**: A valid workflow file
**When**: `verify --format json` is invoked
**Then**: The JSON contains all required certificate fields: `profile`, `artifact`, `replay`, `durability`, `repair_hints`, `exit_code`

### test_invariant_exit_code_stable_across_text_and_json_format
**Given**: An invalid workflow file producing `YamlParse` error
**When**: `verify invalid.yaml` and `verify invalid.yaml --json` are both invoked
**Then**: Both invocations return the same exit code (1)

### test_invariant_failing_gates_identical_in_text_and_json
**Given**: An invalid workflow file producing `Compile` error
**When**: `verify invalid.yaml` and `verify invalid.yaml --json` are both invoked
**Then**: The set of failing gates reported in text output equals the set in JSON `errors` array

### test_invariant_no_panic_propagates_to_operator
**Given**: A workflow file that causes an internal panic in a downstream crate
**When**: `verify bad.yaml` is invoked
**Then**: Operator sees a clean error message with exit code 2; no Rust panic string or backtrace in output

### test_invariant_json_output_is_valid_utf8
**Given**: A valid workflow file
**When**: `verify workflow.yaml --json` is invoked
**Then**: The output is valid UTF-8; `serde_json::from_str::<Value>(&output)` succeeds

### test_exit_code_matches_documented_value_for_each_error_variant
**Given**: Each `VerifyError` variant
**When**: `exit_code_for_error` is called with that variant
**Then**: The returned `CliExitCode` matches the documented discriminant value from `exit_code.rs`

### test_verify_format_json_emits_a_parseable_verificationreport_with_certificate_ident
**Given**: A valid workflow YAML file
**When**: `verify workflow.yaml --format json` is invoked
**Then**: The output is parseable as a JSON object; it contains `profile`, `artifact.digest`, `exit_code`, and `repair_hints`

## Given-When-Then Scenarios

### Scenario 1: Operator verifies a valid workflow in text mode
**Given**: A valid workflow YAML file at `tests/fixtures/valid/minimal.yaml`
**When**: The operator runs `velvet-ballastics verify tests/fixtures/valid/minimal.yaml`
**Then**:
- Exit code is 0
- Output contains "verification certificate"
- Output contains "verified"
- Output lists checks: yaml_parse, compilation, ir_validation, budget_computation, boundedness_policy (for standard/full)

### Scenario 2: Operator verifies a valid workflow in JSON mode
**Given**: A valid workflow YAML file
**When**: The operator runs `velvet-ballastics verify workflow.yaml --json`
**Then**:
- Exit code is 0
- Output is valid JSON
- JSON object contains `"success": true`
- JSON object contains `profile`, `digest`, `checks`, `warnings`, `repair_hints`, `exit_code`
- All hex digests are 64-character lowercase hex strings

### Scenario 3: Operator verifies an invalid workflow — YAML parse failure
**Given**: A workflow YAML file with a YAML syntax error
**When**: The operator runs `velvet-ballastics verify invalid.yaml`
**Then**:
- Exit code is 1 (`ValidationFailed`)
- Output contains "YAML parse error: ..."
- In JSON mode, `success` is `false` and `error` field is non-empty
- Repair hint is non-empty citing "YamlParse" gate

### Scenario 4: Operator verifies with full profile — budget policy violation
**Given**: A workflow that violates budget policy (e.g., exceeds step count limit)
**When**: The operator runs `velvet-ballastics verify workflow.yaml --profile full`
**Then**:
- Exit code is 2 (`VerificationFailed`)
- Output contains "budget policy violation: ..."
- Repair hint cites "BudgetPolicy" gate
- In JSON mode, `success` is `false`

### Scenario 5: Operator verifies the same file in text and JSON — exit codes match
**Given**: Any workflow file (valid or invalid)
**When**: The operator runs both `velvet-ballastics verify file.yaml` and `velvet-ballastics verify file.yaml --json`
**Then**:
- Both invocations return the same exit code
- The set of failing gates is identical in both outputs
