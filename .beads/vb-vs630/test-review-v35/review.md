# Test Suite Review: vb-vs630 (v35)

**Candidate:** `52a69710e78634fb765cdf51a09bf39ef00106ae`
**Scope:** 25 packets (contract, test support, behavioral/failure/boundary tests, source, integration tests)
**Executed evidence:** `ROOT_CLI_ISSUANCE_V33_TEST.json` — 1678 storage lib tests, 1678 passed, 0 skipped, 1.499s
**Workspace cross-crate CLI tests:** Not rerun on this candidate (acknowledged as separate gate)

## Status: REJECTED

### Findings

**F1 — WriteLockPoisoned not tested (MEDIUM)**
- Contract lists `JournalError::WriteLockPoisoned` as an exact actual error.
- No test poisons the write lock and asserts the error variant.
- Location: `crates/vb_storage/src/inputs/submission_tests.rs` — missing.
- Why it matters: Lock poisoning is a distinct failure mode from deadlock; without a test, implementation could silently change to deadlock or ignore the poisoned state.

**F2 — PostcardEncodeFailed not tested (MEDIUM)**
- Contract lists `JournalError::PostcardEncodeFailed` as an exact actual error.
- No test corrupts the artifact structure to cause postcard encoding failure during `prepare_stamped_submission` or `issue_artifact`.
- Location: `crates/vb_storage/src/inputs/submission_tests.rs` or new failure test module — missing.
- Why it matters: Encoding happens after issuance in the CLI path; failure must not leave counter advanced or partial records published.

**F3 — bind_artifact_digest failure propagation not tested (LOW-MEDIUM)**
- Contract states: "Digest binding failures: actual `bind_artifact_digest` errors propagate; `JournalError::PayloadDigestMismatch` only for binding identity failures, not all failures."
- No test constructs an artifact that triggers an actual `bind_artifact_digest` error (distinct from identity mismatch) and asserts the propagated error variant.
- Location: `crates/vb_storage/src/inputs/submission_tests.rs` — missing.
- Why it matters: Confirms the error taxonomy is correctly implemented, not all binding failures collapsed to a single variant.

**F4 — Malformed counter tests only 7 and 9 bytes (LOW)**
- Tests `cli_malformed_counter_seven_bytes` and `cli_malformed_counter_nine_bytes`.
- No tests for length 0, 1, 2, 3, 4, 5, 6, 10, or 100 bytes.
- Location: `crates/vb_storage/src/inputs/issuance_boundary_tests.rs` — incomplete coverage.
- Why it matters: Length 8 is valid; any other length should yield `MalformedKeyspaceRow`. Testing only 7 and 9 is a heuristic choice, not proof that the decode logic handles all invalid lengths.

### Verified Coverage (Not Findings)

The following policies are fully covered by concrete, deterministic, public-API tests with exact assertions:

- All 3 policies (Strict/Journaled/Relaxed) stamp `accepted_at_seq=1` and advance counter
- Stamped digest matches `RunInputsRecord.artifact_digest`, `AcceptedArtifact.digest`, `AcceptedArtifact.verification.digest`, and `RunAdmission.artifact_digest`
- Input digest, typed SlotValues, source, workflow digest, and header `compiled_digest` preserved unchanged
- Per-run event sequences: RunAccepted at 0, RunAdmission at 1
- Relaxed policy writes zero events
- Sequential CLI submissions increment counter monotonically
- Mixed default + CLI submissions share global counter (default→CLI→default tested)
- Pure prepare (no commit) does not advance counter; journal remains empty
- Duplicate run rejected with `RunAlreadyExists` before counter publication; all records and indexes unchanged
- Injected batch commit failure (`fail_next_batch_commit_for_test`) yields exact `Fjall(Io(Other))` on all 3 policies; counter and all keyspaces unchanged; retry succeeds at seq 1
- Policy validation failure before journal open (`journaled_artifact_cannot_be_committed_as_strict_or_relaxed`)
- Malformed 7-byte and 9-byte counters yield exact `MalformedKeyspaceRow` with `prefix=0, expected_len=8, actual_len=<actual>`; raw counter retained unchanged; no run records published
- Counter at `u64::MAX` yields `SequenceOverflow`; counter unchanged at MAX
- Counter at `u64::MAX-1` allows one submission at MAX with full record retention; subsequent submission yields `SequenceOverflow` and prior records preserved
- Same-path close/reopen preserves counter, all typed records, digests, and events; second submission advances counter to 2
- Bounded 4-thread concurrent submissions produce distinct sequences [1,2,3,4] with typed bindings intact
- Integration tests (packets 11, 12) verify CLI submission via actual `velvet-ballistics` binary, retained input execution, input boundary validation, and pre-validation journal isolation

### Determinism and Mutation

- No `#[ignore]` tests, no sleeps, no hidden shared mutable state
- All tests use deterministic temporary directories (`tempfile::tempdir`)
- Mutation resistance verified: deleting counter increment, stamp, digest rebind, event sequencing, or policy handling is caught by named tests
- Close/reopen test uses explicit `FjallJournal::open` on same path — not mocked

### Recommendations for Fix

1. Add `write_lock_poisoned` test: spawn thread holding lock, let it panic, assert `WriteLockPoisoned` on subsequent commit.
2. Add `postcard_encode_failure` test: use `#[cfg(test)]` injection or construct unencodable artifact to trigger `PostcardEncodeFailed`.
3. Add `bind_artifact_digest_failure` test: construct artifact with valid structure but invalid binding data that `bind_artifact_digest` rejects, assert propagated variant.
4. Expand malformed counter tests to include length 0, 1, and 10 (or use parameterized test over lengths [0,1,2,3,4,5,6,7,9,10]).

Byte budget tests acknowledged as separate pending gate — not counted as passed or failed in this review.
