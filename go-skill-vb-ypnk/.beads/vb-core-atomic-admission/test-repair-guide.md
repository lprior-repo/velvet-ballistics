# Test Repair Guide: vb-core-atomic-admission

## Routing

- Primary route: return to State 8 test-writer repair.
- State 7 route is not required now because `test-plan-review.md` approves the plan.
- Route back to State 7 only if the repair discovers that the approved test plan names impossible/nonexistent public boundaries and must be rewritten before tests can be made exact.
- After repair, rerun State 9 test-reviewer from Tier 0; do not review only the changed finding.

## Required State 8 repairs

1. Implement every missing contract error scenario from `contract.md:71-78` and `test-plan.md:53-60`:
   - `given_invalid_accepted_artifact_when_strict_admission_runs_then_invalid_accepted_artifact_error`
   - `given_inconsistent_admission_input_when_strict_admission_runs_then_inconsistent_admission_input_error`
   - `given_batch_commit_failure_when_strict_admission_runs_then_batch_commit_failed_error_and_no_ack`
   - `given_partial_visibility_when_readback_runs_then_partial_visibility_detected_error`
   - `given_sequence_binding_failure_when_strict_admission_runs_then_sequence_binding_failed_error`
   - `given_raw_workflow_parts_when_strict_admission_runs_then_strict_raw_workflow_parts_rejected_error`
   - `given_index_derivation_failure_when_strict_admission_runs_then_index_derivation_failed_error`
2. Replace `crates/vb_storage/tests/vb_core_atomic_admission_red.rs:209` with exact evidence. The test must assert the strict admission/readback API returns `AdmissionError::StrictRawWorkflowPartsRejected` with record kind/boundary context, not merely that postcard raw decode fails.
3. Expand changed tests to cover the approved plan's unimplemented behavior groups: B01, B02, B04, B05, B12, B13, B15, B16, B17, B18, U01-U06, I01-I06, P01-P09 where executable now, F01-F04 smoke/seed targets where executable now, and K01-K03 where executable now or validly waived.
4. Preserve exact red-test assertions: exact values, exact durable absence, exact records, exact vectors, exact `AdmissionError::*` variants, and exact context fields. Do not use `is_ok()`, `is_err()`, `Some(_)`, or generic string-only errors as behavior proof.
5. Refresh `.beads/vb-core-atomic-admission/test-writer-report.md` with a complete scenario map, focused compile evidence, expected red-run evidence, and any valid waiver evidence for unavailable verifier/fuzz/Kani surfaces.

## Re-review command expectations

- Compile the repaired changed tests with workspace-local `TMPDIR` and `RUSTC_WRAPPER=` if disk quota/sccache requires it.
- Run the repaired red tests and show failures are due to missing implementation, not invalid test setup.
- Re-run static scans for banned assertions, silent error suppression, ignored tests, sleeps, shared mutable globals, mocks, and private integration-test imports.
