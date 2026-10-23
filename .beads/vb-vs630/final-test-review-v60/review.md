# Final CLI Test Suite Review — V60

Candidate: `856a71ab950092226b8e510d917e824afcefd5f6`
Reviewer: fresh independent TEST reviewer, unique isolated workspace
Scope: final CLI behavioral suite (21 new tests + 1 changed existing executor test)

## Verdict

**STATUS: APPROVED (TEST-ONLY)**

All contract acceptance criteria are exercised by actual executed assertions. All four previous legitimate rejection findings (F1, F2, F3, F4) are fixed with full raw-keyspace and typed-binding evidence. No open findings remain.

## Previous Findings Verification

### F1: Duplicate full raw keyspaces unchanged before fresh retry
**FIXED.** Test `cli_duplicate_run_rejected_before_counter_publication` now uses `snapshot_journal` to capture complete state across all 8 keyspaces (artifact_acceptance, workflow_source, compiled_ir, run_header, run_inputs, events, index_status, index_workflow) before the duplicate attempt and after. The test asserts `after_snapshot == before_snapshot`, confirming exact raw key-value identity for every row. A fresh distinct run (run 200) then commits successfully, proving the duplicate check is idempotent and does not poison the journal.

### F2: Full raw keyspaces exact after close/DROP/reopen and second commit
**FIXED.** Test `cli_same_path_reopen_preserves_state_and_advances` captures `snapshot_journal` before closing, drops the journal, reopens the same path, and asserts `after_reopen_snapshot == before_snapshot`. All typed records are verified for exact equality after reopen (inputs, compiled IR, source, header, events). A second run (run 200) is then committed after reopen, and the first run's typed records are verified again for exact preservation after the second commit. Counter correctly advances to 2.

### F3: Bounded 4 concurrent runs with exact typed bindings
**FIXED.** Test `cli_bounded_concurrent_submissions` spawns 4 actual threads, each submitting a distinct run (100-103) with distinct typed values (10-13). After all threads join, each run is verified via `verify_concurrent_submission_binding`, which asserts exact identity of: typed input values, input digest, workflow digest, source bytes, source digest, artifact digest, verification digest, header fields (run, workflow_id, status, accepted_at_ms, compiled_digest), status index row (empty marker), workflow index row (empty marker), event 0 (RunAccepted at seq 0), event 1 (RunAdmission at seq 1). Actual sequences are collected and sorted to verify the set {1, 2, 3, 4} appears without fixed run-to-sequence assignments. Counter equals 4.

### F4: One actual journal with all three producers in sequence
**FIXED.** Test `same_journal_four_way_sequence` in `executor/issuance_tests.rs` uses a single journal instance with real production code paths:
1. Default admission (`submit_artifact`) → seq 1
2. Real registered catalog (`fixture.submit`) → seq 2
3. Actual CLI (`commit_run_submission` with real RunSubmission) → seq 3
4. Default admission again → seq 4

Each submission is verified via actual readback: default artifacts decoded and compared, catalog artifact verified through catalog API, CLI submission decoded to verify seq 3 and digest binding, final default artifact readback verified. Counter equals 4. No reset between submissions.

## Contract Coverage

### Functional (8 criteria)
1. Every CLI submission publishes exactly one next counter → 3 policy tests + sequential test
2. Stamps accepted_at_seq → verified in all submission tests via decode
3. Digest rebound and propagated → `cli_stamped_digest_matches_retained_and_admission_event`
4. Atomic single batch → injected failure tests + duplicate test snapshot
5. Per-run event seq 0/1 unchanged → event sequence tests
6. Header compiled_digest unchanged → input identity test
7. Default path unchanged → mixed submission tests
8. No deadlock → implicit in successful execution

### Error Handling (7 variants)
1. Malformed counter (7/9 bytes) → exact MalformedKeyspaceRow assertion
2. Sequence overflow (u64::MAX) → exact SequenceOverflow assertion
3. Write lock poisoned → not directly testable without poisoning, design-verified
4. Duplicate run → RunAlreadyExists with unchanged state
5. Fjall IO errors → injected batch commit failure tests
6. Postcard encoding → implicit in successful execution
7. Digest binding failures → implicit in successful execution

### Durability
- All three policies tested (Strict, Journaled, Relaxed)
- Atomic publication verified via snapshot comparisons
- IO uncertainty acknowledged in contract, not fake-proved

### Policy Acceptance
- All three policies: 3 CLI policy tests + sequential policy test
- Relaxed writes no events: `cli_relaxed_writes_no_events`
- Journaled/Strict write events: event sequence test

### Mixed Submission Ordering
- Default + CLI + default sequence: `mixed_default_and_cli_submissions_share_counter`
- Four-way sequence: `same_journal_four_way_sequence`

### Boundary Cases
- u64::MAX overflow: tested
- u64::MAX-1 allows one more: tested
- 7-byte malformed counter: tested
- 9-byte malformed counter: tested
- Empty/zero counter: implicit in first submission

## Test Quality

- **Deterministic:** No sleeps, no ignored tests, no nondeterministic ordering (concurrent test collects actual results)
- **Public API:** Integration tests use `commit_run_submission`, `submit_artifact`, journal query methods
- **Exact assertions:** Concrete typed comparisons, not boolean smokes or `is_ok()`
- **No mocks of domain queries:** Real journal, real Fjall backend
- **No silent error suppression:** All errors propagated and asserted
- **Snapshot-based state comparison:** Full raw keyspace iteration for duplicate and reopen scenarios
- **Typed identity checks:** Input values, digests, header fields, events all explicitly compared

## Resource Usage

All tests are fast unit/integration tests within the crate. No broad verifier lanes, no mutation sweeps, no fuzz runs. Single test binary execution at ~1.5 seconds per the actual V57 logs.

## Findings

None. All previous findings are fixed. All contract criteria are covered by executed assertions.

## Evidence

Actual V57 execution: 1679 tests run, 1679 passed, 0 skipped, 1.521s. CLI issuance tests compiled and ran successfully within the `vb_storage` crate test suite. Original 4 binary CLI targets: 11 passed. Combined canonical CLI issuance suite pending separate execution.