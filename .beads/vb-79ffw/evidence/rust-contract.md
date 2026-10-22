# Rust Contract Review: vb-79ffw

**Reviewer:** Independent local3090 review
**Baseline:** af142d34 (frozen)
**Old production baseline:** 423d8ae8
**Status:** APPROVED

## Contract Summary

The run-attempt ticket authority mechanism provides:

1. **Run-level attempt tracking:** `ActionTicket.run_attempt` (u16, 1-indexed) advances on each durable run retry
2. **Per-action attempt tracking:** `ActionTicket.attempt` (u16, 1-indexed) tracks retries within a run attempt
3. **Idempotency key binding:** `compute_action_idempotency_key(run, run_attempt, seq, action)` binds the key to the run attempt
4. **Stale rejection:** Old tickets from previous run attempts are rejected as stale

## Contract Verification

### 1. Stale completion rejection (verified)
- Old ticket with `run_attempt=1` vs current `run_attempt=3` is rejected
- Rejection includes exact expected/found values in `StaleAttempt` error
- Verified in `stale_completion_rejected_after_two_retries` test

### 2. Future run-attempt rejection (verified)
- Ticket with `run_attempt` ahead of current is rejected
- Verified in `future_run_attempt_ticket_rejected` test

### 3. Zero run-attempt rejection (verified)
- Ticket with `run_attempt=0` is rejected as invalid
- Verified in `zero_run_attempt_ticket_rejected` test

### 4. Cold recovery preserves run attempt (verified)
- After durable checkpoint, cold recovery restores correct `run_attempt`
- Verified in `cold_recovery_preserves_run_attempt` test

### 5. Idempotency key differs across run attempts (verified)
- Keys computed from different `run_attempt` values differ
- Verified in `idempotency_key_differs_across_run_attempts` test

### 6. Ticket key valid after serialization (verified)
- Ticket roundtrips through postcard serialization with valid key
- Verified in `ticket_key_valid_after_serialization` test

## Contract Compliance

The implementation satisfies all named contract requirements:
- Run-attempt authority is enforced on completion paths
- Idempotency keys are bound to run attempts
- Stale/future/zero attempts are rejected
- Cold recovery preserves attempt state

**Conclusion:** Rust contract is correctly specified and implemented.
