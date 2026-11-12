# Martin Fowler Test Plan

## Happy Path Tests

- `given_valid_accepted_artifact_when_run_created_then_runtime_admits_without_yaml_json_parse`
- `given_valid_accepted_artifact_when_admitted_then_record_contains_digest_certificate_and_profile`

## Error Path Tests

- `given_missing_artifact_when_strict_run_created_then_artifact_not_found_before_allocation`
- `given_raw_workflow_parts_when_strict_run_created_then_invalid_envelope_before_allocation`
- `given_malformed_postcard_when_strict_run_created_then_decode_failed_with_rejected_digest`
- `given_gate_count_zero_or_two_when_strict_run_created_then_gate_mismatch_denies`
- `given_failed_gate_status_when_strict_run_created_then_gate_mismatch_denies`
- `given_stale_or_non_durable_artifact_when_strict_run_created_then_stale_or_invalid_denies`
- `given_digest_mismatch_when_strict_run_created_then_digest_mismatch_denies`
- `given_missing_excess_or_action_mismatched_capability_when_strict_run_created_then_capability_denied`

## Per-Variant Error Scenarios

- ERR-001: `given_missing_artifact_when_strict_run_created_then_artifact_not_found_before_allocation` expects `ArtifactNotFound`/runtime not-found mapping, requested digest preserved, and no allocation.
- ERR-002: `given_raw_or_malformed_bytes_when_strict_run_created_then_decode_failed_with_rejected_digest` expects `ArtifactEnvelopeDecodeFailed`, rejected digest preserved, and raw/YAML/JSON/truncated bytes rejected.
- ERR-003: `given_decoded_envelope_missing_required_acceptance_fields_then_invalid_envelope_denies` expects `ArtifactEnvelopeInvalid` with schema/field/durable/proof-status cause.
- ERR-004: `given_gate_count_zero_two_or_failed_status_when_strict_run_created_then_gate_mismatch_denies` expects `ArtifactGateMismatch` with observed gate evidence and required canonical gate.
- ERR-005: `given_digest_mismatch_when_strict_run_created_then_digest_mismatch_denies` expects `ArtifactDigestMismatch` and does not collapse to invalid envelope.
- ERR-006: `given_stale_artifact_when_strict_run_created_then_stale_certificate_denies` expects `ArtifactStale` with rejected digest and staleness cause.
- ERR-007: `given_missing_excess_prefix_or_action_mismatched_capability_then_capability_denied` expects `CapabilityDenied` with mismatch class.
- ERR-008: `given_cli_ipc_runtime_error_mapping_when_serialized_then_error_category_digest_and_cause_are_preserved` expects RuntimeError/API/CLI/IPC mapping to preserve category, digest when present, and semantic cause.

## Edge Case Tests

- `given_empty_capability_profile_when_artifact_has_extra_public_grant_then_denied`
- `given_unknown_envelope_schema_when_strict_run_created_then_fail_closed_invalid_envelope`
- `given_duplicate_capability_grants_when_strict_run_created_then_denied`
- `given_storage_artifact_gate_count_two_when_runtime_requires_fifteen_then_denied_until_contract_reconciled`

## Contract Verification Tests

- `test_precondition_strict_inputs_must_be_accepted_envelopes`
- `test_precondition_canonical_gate_count_is_enforced`
- `test_postcondition_denial_allocates_no_runtime_state`
- `test_invariant_existence_only_check_never_satisfies_strict_admission`
- `test_invariant_runtime_does_not_parse_yaml_or_json_after_valid_acceptance`

## Given-When-Then Scenarios

### Scenario 1: valid accepted artifact admits
Given: storage contains an accepted-artifact v1 envelope with digest match, canonical gate count, durable non-stale evidence, and exact capability profile.
When: strict/journaled runtime creates a run for that digest.
Then:
- admission succeeds;
- runtime state allocation occurs only after admission;
- no runtime YAML or JSON parse is needed.

### Scenario 2: raw artifact denies before allocation
Given: storage contains raw `WorkflowParts` or YAML/JSON bytes at the requested digest.
When: strict/journaled runtime creates a run for that digest.
Then:
- admission returns typed invalid-envelope diagnostics;
- rejected digest is preserved;
- no frame, run entry, runnable state, `drive_run`, or `RunAccepted` exists.

### Scenario 3: gate mismatch denies
Given: storage contains an accepted artifact with `gate_count` 0 or 2 while strict runtime requires 15.
When: strict/journaled runtime creates a run.
Then:
- admission denies with gate mismatch or invalid envelope diagnostics;
- no runtime state is allocated.

### Scenario 4: dummy store cannot satisfy strict production
Given: a strict/journaled production constructor path.
When: runtime is constructed.
Then:
- it uses storage-backed accepted artifact loading or fails construction;
- `AlwaysPresentArtifactStore` is not reachable for protected strict submissions.
