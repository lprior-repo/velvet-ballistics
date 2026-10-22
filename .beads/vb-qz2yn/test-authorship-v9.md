# vb-qz2yn Test Authorship v9 — Local5090 testfix

## Changes from v8

Fixed one compiler error that prevented compilation: the test helper `digest_artifact` was called in `assert_readback_verified` but does not exist in the API. Replaced with the actual function `accepted_artifact_digest(artifact: &AcceptedArtifact) -> Result<WorkflowDigest, JournalError>` defined at admission.rs:160, with proper `.map_err()` propagation. The helper now calls the real API and maps its `JournalError` to `String` for the test's `Result<(), String>` return type.

Also corrected the `counter_persists_across_reopen` test. Previously it only verified that the second submission received seq 2, which proved counter increment but not artifact durability. Now after the journal is reopened it first reads back and verifies the first artifact is still intact and the counter bytes are exactly `1` (u64 big-endian), before submitting the second artifact. This proves both the counter persisted and the first artifact survived the reopen.

## File Structure

Single test file: `crates/vb_storage/src/admission/acceptance_sequence_tests.rs` (under 300 lines).

Seven test functions:
1. `first_submission_is_seq_1_for_all_policies` — seq 1 on first submit for Relaxed, Journaled, and Strict policies.
2. `repeated_submissions_get_distinct_digests_and_both_readback` — two submits get seq 1/2, distinct digests, both read back.
3. `pure_prepare_stays_seq_0_submit_after_gets_seq_1` — prepare is seq 0 with no counter write; subsequent submit is seq 1.
4. `counter_persists_across_reopen` — first submit seq 1, reopen, counter bytes = 1 (u64 BE), second submit seq 2.
5. `corrupted_counter_length_7_causes_malformed_row` — 7-byte counter → MalformedKeyspaceRow (expected=8, actual=7), counter unchanged, no IR written.
6. `corrupted_counter_length_9_causes_malformed_row` — 9-byte counter → MalformedKeyspaceRow (expected=8, actual=9), counter unchanged.
7. `counter_at_u64_max_causes_sequence_overflow` — counter at u64::MAX → SequenceOverflow, counter unchanged, no IR written.

## Real API Used

- `temp_journal()` — test fixture returning tempdir + open journal
- `minimal_workflow()` — test fixture returning minimal CompiledWorkflow
- `submit_artifact(journal, workflow, policy)` — production admission path
- `prepare_compiled_artifact(workflow, policy, action_contracts)` — prepare-only path
- `verify_accepted_artifact_for_policy(artifact, policy)` — full policy verification
- `accepted_artifact_digest(artifact)` — real digest function from admission.rs:160
- `journal.artifact_acceptance.get/insert` — direct counter access for corruption tests
- `postcard::from_bytes` — decode for stamped-artifact roundtrip check

The AcceptedArtifact type does not have an `executor_profile` field; it has `accepted_at_seq`, `digest`, `source_digest`, `policy_digest`, `verification`, `compiled_ir_bytes`, `policy`, and `capability`.

## Assertions (strong, no weak subset)

- Every submit: sequence number exactly correct
- Every artifact: digest non-zero, source_digest non-zero
- Readback verified: journal record decodes, matches artifact, digest recomputes via `accepted_artifact_digest`, verification proof digest matches
- Reopen test: counter bytes exactly `1` u64 BE; first artifact survives reopen
- Corruption tests: specific `MalformedKeyspaceRow` error with exact prefix/expected/actual fields
- Overflow test: `SequenceOverflow` on u64::MAX; no wrap-around

## Verification Status

Compiler check only. All seven test bodies written with correct real API. One compilation error (`digest_artifact` not found) corrected to use `accepted_artifact_digest`. Tests not yet executed.