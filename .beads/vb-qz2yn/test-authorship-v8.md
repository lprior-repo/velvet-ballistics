# Test Authorship Stage — vb-qz2yn

## Files Written

1. `crates/vb_storage/src/admission/acceptance_sequence_tests.rs` — 6 actual test bodies
2. `crates/vb_storage/src/admission/tests.rs` — added module path decl; updated accepted_artifact_fields_are_populated expectation from 0 to 1
3. `crates/vb_storage/src/journal/tests.rs` — updated declared keyspaces count from 11 to 12

## Tests Implemented

1. `first_submission_is_seq_1_for_all_policies` — Fresh journal for each of 3 policies (Relaxed, Journaled, Strict) gets sequence 1; both digests non-zero; full readback verified with digest comparison and serialized artifact roundtrip equality.

2. `repeated_submissions_get_distinct_digests_and_both_readback` — Same workflow submitted twice gets sequences 1 and 2; artifact digests differ; both readable and verified.

3. `pure_prepare_stays_seq_0_submit_after_gets_seq_1` — Pure `prepare_compiled_artifact` leaves counter at 0 (no key written); subsequent submit gets seq 1.

4. `counter_persists_across_reopen` — Strict policy submission gets seq 1, old artifact readable after close/reopen, next submission gets seq 2.

5. `corrupted_counter_length_7_causes_malformed_row` — 7-byte counter value causes `MalformedKeyspaceRow` with keyspace="artifact_acceptance", key=b"last_issued", actual_len=7, expected_len=8; counter bytes unchanged; no compiled_ir written.

6. `corrupted_counter_length_9_causes_malformed_row` — 9-byte counter value causes `MalformedKeyspaceRow` with actual_len=9, expected_len=8; counter bytes unchanged.

7. `counter_at_u64_max_causes_sequence_overflow` — Counter at u64::MAX causes `SequenceOverflow` on next submit; counter unchanged; no compiled_ir written.

## Design Decisions

- Tests access journal through public crate API (`crate::FjallJournal`, `crate::admission::*`, `crate::error::JournalError`).
- Counter field accessed via crate-private `artifact_acceptance` keyspace with literal key `b"last_issued"` (production constant is crate-private).
- Shared `assert_readback_verified` helper performs digest-based readback, policy verification, and serialized artifact roundtrip equality check.
- Tests use `Result<(), String>` with propagated setup errors via `?`.
- No `unwrap`, `expect`, `panic`, `unsafe`, unchecked arithmetic, or unchecked indexing used.
- No external fixture files or helper modules; tests use parent module's `minimal_workflow` and `temp_journal` via `use super::{...}`.

## Not Yet Covered (Next Stage)

- Concurrent submission tests (8 threads, strictly increasing sequences)
- u64::MAX - 1 boundary test (issues u64::MAX then fails)
- Invalid workflow digest rejection
- Failed validation does not advance counter
- Persist fault and batch commit fault tests
- Post-commit readback fault tests

These are contract-required behaviors deferred to the next bounded local stage.