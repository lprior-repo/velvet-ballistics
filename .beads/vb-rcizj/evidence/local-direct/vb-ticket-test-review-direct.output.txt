# Test Suite Review: vb-rcizj (retry and attempt integration)

## Findings

### BLOCKER: Stale-branch sequence guard not mutation-tested

**Location:** `crates/vb_runtime/src/shard/lifecycle/chunk_004.rs:44` (`require_pending_action_ownership`)

The stale-attempt branch requires `ticket.seq <= pending.seq` in addition to the attempt and identity checks. If a ticket has a lower attempt but a *higher* sequence number, it must be rejected as `InvalidActionCompletion`, not `StaleAttempt`. No test exercises this case.

Mutation: removing `&& ticket.seq <= pending.seq` from the branch guard. Current tests still pass because:
- `ticket_authority_matrix.rs` `Sequence` tamper increments seq but keeps attempt equal, so `ticket.attempt < pending.attempt` is false and the stale branch is never entered.
- `vb_jggy_lifecycle_tests.rs` stale tests use legitimately issued tickets where seq matches.

Add a test that submits a ticket with `attempt = current - 1` and `seq = pending.seq + 1` (recomputed key) and asserts `InvalidActionCompletion`. Without this, a future regression that drops the seq guard would silently misclassify out-of-order replay tickets as stale.

**Disposition required:** `fixed_with_evidence` (add targeted test) or `owner_approved_no_action` with rationale.

## Observations

- `encode_failed_completion_returns_error_and_leaves_state_unchanged` in `vb_jggy_lifecycle_tests.rs` executes the happy path and does not force or verify an encode failure. The test name overstates coverage; the body and comment are honest about documenting expected behavior. Rename or adjust expectation.
- `durable_retry_red_phase.rs` tests are labeled red-phase and several explicitly document integration gaps (e.g., `record_retry_attempt_integration_gap`, `action_failure_preserves_action_completed_slots_integration_gap`). These are acceptable as evidence of known gaps, not as coverage claims.
- The ticket authority matrix correctly expects `InvalidActionCompletion` for all forged-identity variants, consistent with the contract addendum (ownership check precedes attempt/capacity validation).
- Stale-attempt tests (`stale_attempt_completion_rejected_before_journal_write`, `stale_attempt_failure_rejected_before_journal_write`) correctly assert the exact `StaleAttempt` variant with `incoming`/`current` fields, journal immutability, and counter immutability.
- Future-attempt rejection is tested via `future_attempt_within_capacity_is_rejected`. Equal-attempt acceptance is tested via `equal_attempt_is_not_stale`.
- Journal mutation ordering (validation before append) is verified in the stale-attempt tests via snapshot comparison.
- The targeted command ran 26 integration tests across three files, matching the described scope.

## Status

**STATUS: REJECTED**

One blocker finding remains open. Add a targeted test for the stale-branch sequence guard, then re-run the targeted test command. Do not expand to a full CI sweep.