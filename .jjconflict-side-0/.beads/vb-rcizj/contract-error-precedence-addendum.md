# Contract Error Precedence Addendum (vb-rcizj)

This addendum documents the actual production error precedence in the shard
lifecycle command handlers. The original contract incorrectly assumed that
attempt-related errors (StaleAttempt, AttemptBeyondMax) would be returned for
forged tickets. The actual behavior checks ticket ownership/identity FIRST,
before any attempt validation.

## Production Validation Order

### For ActionCompleted (handle_action_completion, lifecycle/chunk_001.rs:568)

1. **require_pending_action_ownership(ticket)** — lifecycle/chunk_004.rs:37
   - If run does not exist → `RunNotFound`
   - If run exists but no pending ticket → `InvalidActionCompletion`
   - If run exists but pending ticket ≠ incoming ticket → `InvalidActionCompletion`
   - Only passes when exact ticket match: run, step, action, attempt, capacity, seq, idempotency_key

2. **run_state_get(run)** — retrieves run state (fails if run disappeared)

3. **preflight_action_completion(state, ticket, output)** — lifecycle/chunk_003_chunks/chunk_001_preflight_completion.rs:59
   - validate_action_completion(state, ticket) → calls validate_ticket_attempt
   - validate_ticket_attempt (helpers.rs:118) checks:
     - attempt=0 or capacity=0 or attempt > capacity → `AttemptBeyondMax`
     - attempt < current counter → `StaleAttempt`
     - attempt > current counter → `InvalidActionCompletion` (future attempt)
   - reject_invalid_ticket_key
   - Output slot, taint, size, and contract validation

### For ActionFailed (handle_action_failure, lifecycle/chunk_001.rs:651)

Identical ordering: ownership check first, then preflight validation.

## Implications for Testing

- **Wrong run/step/action identity** → `InvalidActionCompletion` (or `RunNotFound` if run missing)
  - NOT `StaleAttempt` or `AttemptBeyondMax`
- **Capacity=0 or attempt=0 with correct identity** → `AttemptBeyondMax` (after ownership passes)
- **Stale attempt with correct identity** → `StaleAttempt` (after ownership passes)
- **Future attempt with correct identity** → `InvalidActionCompletion` (after ownership passes)

## Source Evidence

- `require_pending_action_ownership` in lifecycle/chunk_004.rs:37-44
- `validate_ticket_attempt` in helpers.rs:118-138
- Command handling in lifecycle/chunk_001.rs:568, 651

## Master Plan Alignment

Master plan §72 mandates `StaleAttempt` only for actual stale attempts with
matching identity. This addendum preserves all pre-existing behavior and old
assertions; it corrects the contract's expectation about forged tickets.

## Additional Finding: Attempt/Capacity Validation Order

Production validates ticket ownership (exact match) BEFORE attempt/capacity
validation. This means:

- Forged tickets with attempt=0 → `InvalidActionCompletion` (ownership fails first)
- Forged tickets with capacity=0 → `InvalidActionCompletion` (ownership fails first)
- Forged tickets with attempt > capacity → `InvalidActionCompletion` (ownership fails first)

`AttemptBeyondMax` is only returned for tickets that PASS ownership validation
(exact pending ticket match) but have invalid attempt/capacity values. This
scenario requires the exact same run/step/action/seq/idempotency_key as the
pending ticket, differing only in attempt or capacity.

This means the contract's assertion that "a ticket with attempt=0 must be
rejected with AttemptBeyondMax" is only true for the exact pending ticket with
attempt=0, not for any forged ticket with attempt=0.

## Updated Test Expectations

For the three failing ticket_authority_matrix tests, the expected errors are:
- action_completed_attempt_zero_returns_beyond_max → expects `InvalidActionCompletion` (not `AttemptBeyondMax`)
- action_completed_capacity_zero_returns_beyond_max → expects `InvalidActionCompletion` (not `AttemptBeyondMax`)
- action_failed_attempt_zero_returns_beyond_max → expects `InvalidActionCompletion` (not `AttemptBeyondMax`)

These tests will be renamed and updated to reflect the actual production
behavior.
