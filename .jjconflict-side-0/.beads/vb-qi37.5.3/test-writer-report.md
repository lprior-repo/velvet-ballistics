# Test Writer Report - vb-qi37.5.3

## Tests Added

- `admit_artifact_run_rejects_missing_idempotency_gate`.
- `admit_artifact_run_rejects_keyed_action_without_attestation`.
- `admit_artifact_run_carries_idempotency_evidence_to_dispatch`.
- `submit_artifact_carries_idempotency_evidence_from_contracts`.
- `submit_artifact_rejects_failed_idempotency_contract`.

## Red/Green Notes

- Tests target behavior that was previously absent: runtime did not enforce idempotency proof status and `RunAdmission` did not carry idempotency attestation metadata.
