# Contract: Resolve nine retry and attempt integration failures using issued action-ticket authority

## Authority
Master plan §18 (Action Execution Model), §44 (Runtime Verification), and the accepted runtime lifecycle acceptance contract.

## Scope
Own `crates/vb_runtime/tests/durable_retry_red_phase.rs` and `crates/vb_runtime/tests/vb_jggy_lifecycle_tests.rs` entirely. Repair production preflight functions only if a genuinely valid issued-ticket path fails; preserve authority, identity, capacity, sequence, idempotency, and all existing attempt validation semantics.

## Requirements

1. **Issued-ticket acceptance**: A ticket with valid run/step/action identity, correct attempt (matches current counter), and valid idempotency key must be accepted for completion or failure. Success requires the EXACT pending ticket to match the incoming ticket (full equality).
2. **Stale-attempt rejection**: A ticket with attempt lower than the current action_attempts value must be rejected with `StaleAttempt { incoming, current }` before any journal mutation. This applies only when all other identity fields (run, step, action, capacity) match the pending ticket and the canonical idempotency key is valid.
3. **Future-attempt rejection**: A ticket with attempt higher than the current action_attempts value must be rejected with `InvalidActionCompletion` (G005).
4. **Forged/invalid identity rejection**: A ticket with wrong run, step, or action identity must be rejected with `InvalidActionCompletion` before any state or journal mutation.
5. **Zero attempt rejection**: A ticket with attempt=0 must be rejected with `AttemptBeyondMax`.
6. **Capacity validation**: A ticket with attempt > capacity or capacity=0 must be rejected with `AttemptBeyondMax`.
7. **Journal mutation ordering**: All ticket validation must occur before any durable journal append.
8. **Attempt tracking**: action_attempts[step] must be updated monotonically and never decrease.
9. **Retry PC reset**: On retryable failure, the PC must reset to the failed step.
10. **Idempotency key validation**: Any ticket classification (including StaleAttempt) must use the production canonical idempotency key predicate; tickets with invalid keys remain rejected regardless of other fields.

## Test acceptance criteria

- All 9 retained failures (3 from durable_retry_red_phase, 6 from vb_jggy_lifecycle_tests) must pass.
- Existing expected errors must not be weakened; tests must still exercise StaleAttempt, AttemptBeyondMax, InvalidActionCompletion, etc.
- Positive tests must assert exact event/value/attempt behavior.
- Negative tests must verify journal state is unchanged.
- No new unwrap/expect/panic/unchecked operations.
