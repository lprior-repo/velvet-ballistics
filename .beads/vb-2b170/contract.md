# vb-2b170: Ticket Authority Cross-Crate Test (Faithful Source)

## Objective
Author canonical cross-crate test at `crates/workspace_tests/tests/vb_79ffw_ticket_authority_probe.rs` from the known passing prior probe at `git show 3292293143bf2808e3803e59fe614fc8ea43dc88:crates/vb_runtime/tests/vb_79ffw_ticket_authority_probe.rs`. Strengthen the stale-completion test with full serialized journal snapshots and exact error type assertions. All six original tests must pass unchanged.

## Acceptance Criteria
1. Test file located at `crates/workspace_tests/tests/vb_79ffw_ticket_authority_probe.rs`
2. All six original tests pass: stale_completion_rejected_after_two_retries, future_run_attempt_ticket_rejected, zero_run_attempt_ticket_rejected, cold_recovery_preserves_run_attempt, idempotency_key_differs_across_run_attempts, ticket_key_valid_after_serialization
3. strengthened stale-completion test captures full serialized JournalEvent vectors before/after stale rejection and after completion
4. Exact RuntimeError::InvalidActionCompletion error type asserted (not generic is_err())
5. Journal byte vector equality asserted after stale rejection (no state mutation)
6. Current ticket completes successfully after stale rejection
7. RunFinished event verified after successful completion
8. No expect/unwrap/panic/unchecked math/unchecked cast/unchecked indexing
9. No lint suppressions, clippy allow, or cfg(test) bypasses

## Test Suite Structure
- Helper functions: shard_count, canonical_workflow_digest, retry_action_workflow, action_contract, contracts, non_retryable_failure, action_output, pending_ticket, latest_retry_checkpoint, next_seq_after, serialize_event_vector, serialize_events_equal
- Test 1: stale_completion_rejected_after_two_retries (strengthened with journal snapshots)
- Test 2: future_run_attempt_ticket_rejected
- Test 3: zero_run_attempt_ticket_rejected
- Test 4: cold_recovery_preserves_run_attempt
- Test 5: idempotency_key_differs_across_run_attempts
- Test 6: ticket_key_valid_after_serialization

## Limitations
- Exact serialized byte equality tests record payload vector equality, not full powercut recovery semantics
- Test uses public API only; no private module access
- Verification scoped to this single test file

## Verification Command
cargo test -p velvet-ballistics-workspace-tests --test vb_79ffw_ticket_authority_probe -- --nocapture
