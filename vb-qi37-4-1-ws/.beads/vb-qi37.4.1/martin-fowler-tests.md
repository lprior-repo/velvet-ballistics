# Martin Fowler Test Plan: vb-qi37.4.1 - Accepted Artifact Envelope

## Happy Path Tests

- `test_valid_accepted_artifact_roundtrips_through_storage_envelope`
- `test_runtime_admits_run_from_valid_accepted_artifact`
- `test_submit_artifact_creates_valid_accepted_artifact_with_15_gates`
- `test_strict_admission_syncs_before_returning`
- `test_journaled_admission_queues_before_returning`
- `test_accepted_artifact_persisted_in_compiled_ir_keyspace`

## Error Path Tests

- `test_raw_submit_rejected_when_accepted_artifacts_required`
- `test_bad_magic_returns_error`
- `test_unsupported_schema_version_returns_error`
- `test_migration_required_returns_error`
- `test_bad_record_kind_returns_error`
- `test_header_length_mismatch_returns_error`
- `test_header_checksum_mismatch_returns_error`
- `test_payload_digest_mismatch_returns_error`
- `test_payload_too_large_returns_error`
- `test_unexpected_eof_returns_error`
- `test_postcard_decode_failed_returns_error`
- `test_unsupported_artifact_version_returns_error`
- `test_unsupported_workflow_version_returns_error`
- `test_empty_workflow_name_returns_error`
- `test_invalid_workflow_name_returns_error`
- `test_empty_ir_returns_error`
- `test_ir_digest_mismatch_returns_error`
- `test_storage_key_digest_mismatch_returns_error`
- `test_zero_digest_returns_error`
- `test_invalid_gate_count_returns_error`
- `test_verification_gate_failed_returns_error`
- `test_missing_required_proof_flag_returns_error`
- `test_invalid_warning_gate_returns_error`
- `test_duplicate_capability_returns_error`
- `test_duplicate_secret_returns_error`
- `test_duplicate_action_id_returns_error`
- `test_bound_exceeded_returns_error`
- `test_artifact_not_found_returns_error`
- `test_capability_denied_returns_error`
- `test_resource_capacity_exceeded_returns_error`

## Edge Case Tests

- `test_warning_gate_at_lower_bound_accepted`
- `test_warning_gate_at_upper_bound_accepted`
- `test_empty_capabilities_list_accepted`
- `test_empty_required_secrets_list_accepted`
- `test_empty_idempotency_lists_accepted`
- `test_single_warning_accepted`
- `test_all_gates_in_warning_state_accepted`
- `test_zero_digest_rejected_in_any_field`
- `test_max_ir_bytes_at_boundary_accepted`
- `test_ir_bytes_exceeding_max_rejected`

## Contract Verification Tests

### Precondition Tests

- `test_precondition_magic_validation`
- `test_precondition_schema_version_validation`
- `test_precondition_record_kind_validation`
- `test_precondition_header_checksum_validation`
- `test_precondition_payload_digest_validation`
- `test_precondition_postcard_decode_validation`
- `test_precondition_artifact_version_validation`
- `test_precondition_workflow_version_validation`
- `test_precondition_workflow_name_nonempty`
- `test_precondition_ir_bytes_nonempty`
- `test_precondition_ir_bytes_within_bound`
- `test_precondition_ir_digest_equals_blake3_ir_bytes`
- `test_precondition_digests_nonzero`
- `test_precondition_gate_count_equals_15`
- `test_precondition_all_proof_flags_true`
- `test_precondition_warning_gates_in_range`
- `test_precondition_lists_duplicate_free`

### Postcondition Tests

- `test_postcondition_roundtrip_preserves_artifact`
- `test_postcondition_envelope_header_valid`
- `test_postcondition_storage_key_matches_ir_digest`
- `test_postcondition_run_admission_returned_on_success`

### Invariant Tests

- `test_invariant_no_execution_without_accepted_artifact`
- `test_invariant_digestes_are_distinct`
- `test_invariant_run_accepted_before_execution`
- `test_invariant_run_admission_atomicity`

## Given-When-Then Scenarios

### Scenario 1: Valid accepted artifact roundtrips through the storage envelope

**Given** a valid `AcceptedArtifactV1` with all 15 gates passing and `ir_digest == blake3(ir_bytes)`

**When** it is encoded with `MAGIC_COMPILED_ARTIFACT` and decoded as `RecordKind::CompiledIr`

**Then**:
- decoding succeeds
- the decoded artifact has the same semantic fields
- the storage envelope payload digest and header checksum are valid

### Scenario 2: Runtime admits a run from a valid accepted artifact

**Given** accepted-artifact admission is required

**And** the artifact store returns a validated artifact for digest `D`

**And** input validates against the artifact schema

**And** all required capabilities are granted

**And** all required secrets are present

**When** the runtime admits run `R` for artifact digest `D`

**Then**:
- admission returns `RunAdmissionV1`
- `RunAccepted` is recorded before execution
- `RunAdmission` binds `R`, `D`, workflow digest, input digest, capabilities, secret identifiers, and timestamp

### Scenario 3: Raw submit is rejected when accepted artifacts are required

**Given** runtime policy requires accepted artifacts

**When** a caller submits a raw `CompiledWorkflow` through `submit_direct` or equivalent shard command

**Then**:
- admission fails with `AdmissionRequired`
- no frame is allocated
- no run state is inserted
- no `RunAccepted` event is recorded

### Scenario 4: Artifact payload corruption is rejected before runtime mutation

**Given** a stored accepted artifact record has a corrupted payload byte

**When** runtime loads the artifact by digest

**Then**:
- envelope validation fails with `PayloadDigestMismatch` or `PostcardDecodeFailed`
- admission maps the failure to `ArtifactInvalid`
- no execution-visible state changes occur

### Scenario 5: Internal digest mismatch is rejected

**Given** an accepted artifact is stored under key digest `K`

**And** the decoded artifact contains `ir_digest = D` where `D != K`

**When** load-time validation runs

**Then**:
- validation fails with `StorageKeyDigestMismatch`
- the artifact cannot be used for runtime admission

### Scenario 6: Two-gate legacy proof is not runtime-admissible

**Given** a legacy artifact has `gate_count = 2`

**When** validation requires `AcceptedArtifactV1`

**Then**:
- validation fails with `InvalidGateCount { found: 2 }`
- runtime admission rejects it as `ArtifactInvalid`

### Scenario 7: Missing capability rejects admission

**Given** a valid accepted artifact declares capability `network.github` for action `A`

**And** the granted capability set does not grant it

**When** runtime admission checks capabilities

**Then**:
- admission fails with `CapabilityDenied`
- no `RunAccepted` event is recorded

### Scenario 8: Missing secret rejects admission without exposing values

**Given** a valid accepted artifact declares required secret identifier `github_token`

**And** the runtime secret store reports it absent

**When** runtime admission checks secrets

**Then**:
- admission fails with `SecretUnavailable`
- neither the artifact nor the error includes any secret value

### Scenario 9: Strict admission syncs before returning

**Given** runtime policy is `Strict`

**And** all admission validations pass

**When** `RunAccepted` and `RunAdmission` are appended

**Then**:
- `SyncAll` completes before admission returns success
- if `SyncAll` fails, admission returns `StrictDurabilityFailed` and execution does not begin

### Scenario 10: Journaled admission documents the data-loss window

**Given** runtime policy is `Journaled`

**And** all admission validations pass

**When** `RunAccepted` is queued

**Then**:
- admission may return before disk sync
- execution may begin only after the journal append accepts the event into its queue

### Scenario 11: Oversized artifact payload is rejected

**Given** encoded accepted artifact bytes exceed the configured maximum payload length

**When** decoding validates the storage envelope

**Then**:
- decoding fails with `PayloadTooLarge`
- no postcard decode or runtime admission is attempted

### Scenario 12: Warning gate bounds are enforced

**Given** an accepted artifact contains a warning with gate `16`

**When** semantic validation runs

**Then**:
- validation fails with `InvalidWarningGate { gate: 16 }`

## Scenario Coverage Matrix

| Scenario | Test Function | Layer | Status |
|----------|--------------|-------|--------|
| 1 | `test_accepted_artifact_roundtrips_through_storage_envelope` | unit | required |
| 2 | `test_runtime_admits_run_from_valid_accepted_artifact` | integration | required |
| 3 | `test_raw_submit_rejected_when_accepted_artifacts_required` | integration | required |
| 4 | `test_artifact_payload_corruption_rejected` | unit | required |
| 5 | `test_internal_digest_mismatch_rejected` | unit | required |
| 6 | `test_two_gate_legacy_proof_rejected` | unit | required |
| 7 | `test_missing_capability_rejects_admission` | unit | required |
| 8 | `test_missing_secret_rejects_without_exposing_values` | unit | required |
| 9 | `test_strict_admission_syncs_before_returning` | integration | required |
| 10 | `test_journaled_admission_queues_before_returning` | integration | required |
| 11 | `test_oversized_artifact_payload_rejected` | unit | required |
| 12 | `test_warning_gate_bounds_enforced` | unit | required |

## Exit Criteria

- All 12 scenarios have executable test coverage
- All error variants have at least one error path test
- All preconditions have contract verification tests
- All invariants have tests proving the invariant holds
- Integration tests prove `submit_artifact` followed by `admit_run` executes only after `RunAccepted`
- Regression tests prove `submit_direct` is rejected when accepted-artifact policy is required
