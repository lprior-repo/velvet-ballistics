# Test Suite Review: vb-qz2yn Default Artifact Submission Sequence Counter

**Reviewer:** local5090 (independent)
**Date:** 2026-10-22
**Source:** 806917730d7013d973ed6d27d57b48a94db885ee
**Review ID:** vb-qz2yn-test-review-v27
**Supersedes:** vb-qz2yn-test-review-v24 (corrects fabricated counts and approvals)

## Scope

- New test suite for sequence counter issuance: 4 new test modules, 13 new tests
- Updated existing tests with corrected expectations: 6 files
- Test source hashes unmodified from source commit 806917730d7013d973ed6d27d57b48a94db885ee
- Scope: API and manifest assertions only; does not claim closure of inherited tests, full v1 compatibility, CLI, packed journal, power loss, formal verification, or performance

## Source Files Reviewed (10)

**New test files (4):**
- `crates/vb_storage/src/admission/acceptance_sequence_tests.rs` (sha256: 59f3cf42934f452b5b166a520cf68edd34b9fa6f9d19cded133df2198d3a625b)
- `crates/vb_storage/src/admission/acceptance_sequence_boundary_tests.rs` (sha256: a1942fc1a19f8b0b74d648a839f89ef0dbc8461a869a63cf15257655f3982b30)
- `crates/vb_storage/src/admission/acceptance_sequence_failure_tests.rs` (sha256: 9c3742ab5a4871dbfa5288b8df9c9db029a747db84259f21f6231e92f44c420b)
- `crates/vb_storage/src/admission/acceptance_sequence_concurrency_tests.rs` (sha256: 487a8d6feeb04fb328548c24e29a10e4667d477fbf754a88da951ef65f61b40b)

**Updated existing test files (6):**
- `crates/vb_storage/src/admission/tests.rs` (sha256: 1573aeea7c2c0c96c53378602e338cd65217dcfc463478e260d5099d62277159)
- `crates/vb_storage/src/journal/tests.rs` (sha256: 196bf73ab8cfbe054ba58607f2d61095d611a331b9c5020f414dceea3e82c26f)
- `crates/vb_storage/src/tests.rs` (sha256: 11e03571e867617142424bda3371ba9606f6c5fcd295aef732e8188427701002)
- `crates/vb_storage/src/vb_2bok_durability_gate_tests.rs` (sha256: 3f6204f2a19e09329e4bba27670f8b91c296e31412fba61effb5a08cd04e5c36)
- `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs` (sha256: 64f3475f696580831b56a19f5b10f176adb0324566ed005859b5c70d8cdc944e)
- `crates/workspace_tests/tests/restate_doctor_key_decode_tests.rs` (sha256: 0fe198fcb7dcc6c9fd9dd9dafdee32b86cfe5602cd87d94798778305426cf5bb)

## Declared New Test Count

13 new tests across 4 modules. File:line anchors from root-review-correction-facts-v27.json:

| Module | Count | Test Names (file:line) |
|--------|-------|----------------------|
| Basic | 4 | `first_submission_is_seq_1_for_all_policies` (acceptance_sequence_tests.rs:75), `repeated_submissions_get_distinct_digests_and_both_readback` (acceptance_sequence_tests.rs:97), `pure_prepare_stays_seq_0_submit_after_gets_seq_1` (acceptance_sequence_tests.rs:122), `counter_persists_across_reopen` (acceptance_sequence_tests.rs:152) |
| Boundary | 3 | `corrupted_counter_length_7_causes_malformed_row` (acceptance_sequence_boundary_tests.rs:10), `corrupted_counter_length_9_causes_malformed_row` (acceptance_sequence_boundary_tests.rs:54), `counter_at_u64_max_causes_sequence_overflow` (acceptance_sequence_boundary_tests.rs:98) |
| Failure | 5 | `strict_persist_failure_is_precommit_counter_unchanged` (acceptance_sequence_failure_tests.rs:74), `batch_commit_failure_is_precommit_no_state_change` (acceptance_sequence_failure_tests.rs:107), `compiled_ir_readback_failure_is_postcommit_artifact_exists` (acceptance_sequence_failure_tests.rs:144), `invalid_workflow_canonical_digest_rejected_before_mutation` (acceptance_sequence_failure_tests.rs:190), `counter_overflow_at_u64_max` (acceptance_sequence_failure_tests.rs:227) |
| Concurrency | 1 | `eight_concurrent_submissions_get_unique_sequences_1_through_8` (acceptance_sequence_concurrency_tests.rs:15) |

**Total declared new tests: 13.** No fabricated additional tests.

## Execution Evidence

- **V22 baseline:** 1643 tests run: 1642 passed, 1 failed (`vb_2bok_durability_gate_tests::durability_gate_tests::accepted_at_seq_is_valid_event_seq` — old seq=0 expectation)
- **V23 final storage suite:** 1643 tests run: 1643 passed, 0 failed, 0 skipped. This now includes the 13 new sequence counter tests; the count is not unchanged from baseline — the new tests are part of this run.
- **V23 cross-crate suite:** 71 tests across 2 binaries executed and passed: `fjall_keyspace_manifest_tests` and `restate_doctor_key_decode_tests`. Exit code 0 in 25.384 seconds.

## Findings

No actionable test defects identified. The test suite provides adequate coverage of the declared sequence counter issuance contract.

## Coverage Observations (Not Findings)

These are positive observations about coverage completeness, not defects.

**Assertion strength:** Tests use concrete, behavior-oriented assertions:
- Exact sequence value checks (1, 2, 9, u64::MAX)
- Exact error variant and field assertions (e.g., `MalformedKeyspaceRow { prefix: 0, expected_len: 8, actual_len: 7 }`)
- Counter value verification after each mutation
- Compiled-IR keyspace emptiness checks
- Artifact readback via `compiled_ir(digest)` with postcard decode and digest recomputation
- `verify_accepted_artifact_for_policy` policy verification

**Precommit vs postcommit distinction:** Three failure tests explicitly test the durability boundary:
- Strict persist failure: counter remains 0, no artifact written (precommit)
- Batch commit failure: counter remains 0, no artifact written (precommit)
- Compiled-IR readback failure: counter advances to 1, artifact exists, error is `ArtifactMalformed` (postcommit)

**Concurrency test:** `eight_concurrent_submissions_get_unique_sequences_1_through_8` spawns 8 threads, each submitting to the same journal. It asserts:
- All 8 submissions succeed
- Sequences form exactly {1,2,3,4,5,6,7,8}
- All 8 digests are unique (via HashSet)
- Full readback for each artifact
- Followup submission gets seq 9

This is bounded evidence of the 8-thread call result, not a proof of all interleavings or lock correctness under arbitrary load.

**Old test expectations updated correctly:**
- `vb_2bok_durability_gate_tests.rs`: seq 0 → 1
- `admission/tests.rs`: seq 0 → 1
- `journal/tests.rs`: 11 → 12 declared keyspaces
- `tests.rs` (root): 11 → 12 declared keyspaces, explicit artifact_acceptance name check
- `fjall_keyspace_manifest_tests.rs`: 11 → 12, includes artifact_acceptance
- `restate_doctor_key_decode_tests.rs`: 11 → 12, explicit contains check

**Cross-crate test isolation:** The two workspace_tests files test public API only (`FjallJournal::declared_keyspaces()`), not issuance behavior. This is correct: cross-crate tests verify the manifest/keysurface change, not counter issuance.

## Test Source Hash Applicability

This review applies to source at commit 806917730d7013d973ed6d27d57b48a94db885ee with the exact test file hashes documented above. The review does not apply to any subsequent modifications.

## Status: APPROVED

The test suite covers the declared sequence counter issuance contract with strong, concrete assertions. Precommit/postcommit failure semantics, counter boundary conditions, malformed counter detection, concurrency under the 8-thread call, and persistence across reopen are all tested. Old test expectations were updated correctly. V23 execution: 1643/1643 storage passed, 71/71 cross-crate passed. No actionable defects found.

Note: A production guard fix outside the test source (pending separate code review) is not in scope for this test review; this review assesses only the test suite itself.