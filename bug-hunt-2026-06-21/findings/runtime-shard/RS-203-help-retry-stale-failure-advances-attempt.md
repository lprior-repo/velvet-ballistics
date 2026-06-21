# RS-203-help: Stale retry failures advance the live attempt counter

- **Severity**: High
- **Category**: correctness / bug
- **Location**: `crates/vb_runtime/src/shard/helpers/retry.rs:85`
- **Confidence**: confirmed

## Description

`record_retry_attempt` validates only that the failed ticket is within the retry policy maximum. It does not reject stale ticket attempts, and the pure kernel uses `max(current, ticket_attempt)`, so a duplicate failure from an older attempt can consume the next retry slot or prematurely exhaust retries.

## Evidence

```rust
// retry.rs:14-21
fn validate_retry_attempt(ticket: ActionTicket, policy: RetryPolicy) -> RuntimeResult<()> {
    if policy.max_attempts == 0 || ticket.attempt == 0 || ticket.attempt > policy.max_attempts {
        return Err(RuntimeError::AttemptBeyondMax { ... });
    }
    Ok(())
}

// retry.rs:85-97
pub fn record_retry_attempt(
    state: &mut RunState,
    ticket: ActionTicket,
    policy: RetryPolicy,
) -> RuntimeResult<bool> {
    validate_retry_attempt(ticket, policy)?;
    let slot = state.action_attempts.get_mut(ticket.step.as_usize()) ...?;
    let (next, can_retry) = retry_attempt_after(Some(*slot), ticket.attempt, policy.max_attempts)?;
    *slot = next;
    Ok(can_retry)
}

// retry.rs:127-134
let base = c.max(ticket_attempt);
if base >= max_attempts {
    Ok((base, false))
} else {
    let next = base.checked_add(1) ...?;
    Ok((next, true))
}
```

Concrete sequence with `max_attempts = 3`: attempt 1 fails and advances the counter to 2. A duplicate stale failure for attempt 1 then passes `validate_retry_attempt`; `base = max(2, 1) = 2`; the counter advances to 3 even though attempt 2 did not fail.

## Adversarial Check

This is not a theoretical mismatch with a private invariant. `helpers/action.rs` has an explicit stale-attempt fence for completions: `classify_ticket_attempt` returns `StaleAttempt` when `ticket_attempt < current` (`action.rs:180-183`). Retry failure handling lacks the same fence even though it mutates the same `action_attempts` counter. Duplicate or delayed async failures are exactly the case generation and attempt fences are meant to reject.

## Suggested Fix

Make `retry_attempt_after` reject `ticket_attempt < current` with `AttemptFenceError::StaleAttempt` and reject `ticket_attempt > current` as an invalid completion/failure. Only advance from `current == ticket_attempt`, after confirming both are within `max_attempts`.
