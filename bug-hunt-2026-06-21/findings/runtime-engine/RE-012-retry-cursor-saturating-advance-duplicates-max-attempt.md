# RE-012: Retry cursor advancement can duplicate the maximum attempt

- **Severity**: Medium
- **Category**: bug
- **Location**: `crates/vb_runtime/src/engine/retry_math.rs:100-115`
- **Confidence**: confirmed

## Description

`RetryPolicy::next_cursor` uses `saturating_add(1)` to advance the attempt number, while `validate_cursor` does not prove that `attempt + remaining - 1` fits inside `max_attempts`. A caller can construct a public `RetryCursor` that validates, then advances from `u16::MAX` to `u16::MAX` again instead of failing.

## Evidence

`crates/vb_runtime/src/engine/retry_math.rs:100-115`:

```rust
self.validate_cursor(max_interval_ms, cursor)?;
if cursor.exhausted || cursor.remaining <= 1 {
    return Ok(RetryCursor {
        remaining: 0,
        exhausted: true,
        ..cursor
    });
}
let next_attempt = cursor.attempt.saturating_add(1);
self.validate_attempt(next_attempt)?;
Ok(RetryCursor {
    attempt: next_attempt,
    remaining: cursor.remaining.saturating_sub(1),
    delay_ms: self.delay_after_valid_attempt(max_interval_ms, cursor.attempt),
    exhausted: false,
})
```

`crates/vb_runtime/src/engine/retry_math.rs:149-161` only checks delay, `remaining <= max_attempts`, and `attempt <= max_attempts`:

```rust
if cursor.remaining > self.max_attempts {
    return Err(RetryPolicyMathError::RemainingExceeded);
}
if cursor.exhausted {
    return Ok(cursor);
}
match self.validate_attempt(cursor.attempt) {
    Ok(_) => Ok(cursor),
    Err(error) => Err(error),
}
```

With `RetryPolicy { max_attempts: u16::MAX, ... }` and `RetryCursor { attempt: u16::MAX, remaining: 2, delay_ms: 0, exhausted: false }`, validation passes. `saturating_add(1)` returns `u16::MAX`, so the next cursor repeats the same attempt number with `remaining: 1`.

## Adversarial Check

The cursor fields are public, so this is not limited to internally produced cursors. The code already has validation APIs, which means `next_cursor` is meant to reject malformed cursors rather than trust callers. Saturating arithmetic hides the overflow and produces a plausible but wrong state; `validate_attempt` cannot catch it because the saturated value is still within `max_attempts`.

## Suggested Fix

Replace the saturating advance with checked arithmetic and validate the cursor invariant before advancing. For non-exhausted cursors, require `remaining > 0` and `attempt.checked_add(remaining - 1) <= Some(max_attempts)`. Return a new `RetryPolicyMathError` for inconsistent cursors instead of manufacturing a duplicate attempt.
