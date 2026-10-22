# Test Review: Run-Attempt Ticket Authority (vb-xvqwl)

**Frozen SHA:** 9954545d3f816ec360b9ecd07ea07dfd77a27767
**Baseline SHA:** 423d8ae8b667d82b4d36e57d29d7ef2aede08bdf
**Reviewer:** Local RTX5090 Qwen3.8-27B via OMP high reasoning

## Summary

Four new regression tests and updates to two existing durable retry tests. All tests use the real Runtime executor, real StorageRuntimeJournal over FjallJournal, and real failure-boundary checkpoint production (no fabricated tickets).

## Decision

APPROVED

The test suite comprehensively covers the new run-attempt ticket authority with genuine counterexamples and proper assertion strength.

## Evidence

### New Tests (4) — All Pass

**Test:** `stale_old_ticket_rejected_after_durable_retry`
**What it proves:** A durable run fails on attempt 1. After recovery and explicit retry (`recover_retry`), the new pending ticket carries run_attempt=2. Replaying the old ticket's completion from attempt 1 is rejected with `StaleAttempt { incoming: 1, current: 2 }`. Counters (runs_completed, runs_failed) remain unchanged after stale rejection. The new ticket's completion succeeds.
**Assertion strength:** Strong. Verifies counter unchanged (no side effect), exact typed error values, new ticket accepted.

**Test:** `run_attempt_advances_but_per_action_attempt_does_not`
**What it proves:** After a durable run retry, run_attempt advances from 1 to 2 but per-action attempt resets to 1. Validates the distinction between the two counters.
**Assertion strength:** Strong. Explicit counter value assertions.

**Test:** `future_run_attempt_ticket_rejected`
**What it proves:** A ticket with run_attempt=3 (future) against current run_attempt=2 is rejected. Not just past tickets, but future tickets too.
**Assertion strength:** Strong. Verifies rejection with correct error type.

**Test:** `zero_run_attempt_ticket_rejected`
**What it proves:** run_attempt=0 (not 1-indexed) is rejected as invalid.
**Assertion strength:** Strong. Verifies rejection with correct error type.

### Existing Tests Updated (2) — All Pass

**Test:** `durable_retry_reopens_and_completes_failed_step` (durable_retry_execution.rs)
**Update:** Added assertion `assert_eq!(retry_ticket.run_attempt, 2, "retry must carry new run attempt number");` to verify the retry ticket has the incremented run_attempt.

**Test:** `checkpoint_roundtrips_full_retry_authority` (durable_retry_red_phase.rs)
**Update:** Updated ticket construction to include run_attempt field.

### Verification of Assertion Strength

The stale rejection test is the key regression. It verifies:
1. Old ticket carries run_attempt=1
2. New ticket after retry carries run_attempt=2
3. Idempotency keys differ between attempts
4. Replaying old completion is rejected (not accepted)
5. Error carries exact incoming/current values
6. Run state unchanged after stale rejection (counters)
7. Old failure ticket also rejected
8. New ticket completion succeeds

This is comprehensive and sufficient to catch the original counterexample (old completion/failure accepted on retry).

### Tests Not Included (by design)

The known independent issues B/C/D (automatic terminal recovery, strict retry sync barrier, raw checkpoint durable chain binding) are being repaired by DeepSeek vb-hz9v8 and are out of scope for this bead's tests.

## Real Runner Verification

All tests run against the real Runtime executor, not mocks. The real journal, checkpoint production, and recovery path are exercised.
