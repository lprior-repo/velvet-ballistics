# RP-001: `evaluate_retry` off-by-one allows `max_attempts + 1` total attempts

- **Severity**: High
- **Category**: bug
- **Location**: `crates/vb_runtime/src/primitives/retry.rs:89`
- **Confidence**: confirmed

## Description

`evaluate_retry` tests `state.remaining() == 0` to decide exhaustion, but `RetryState::remaining` is documented as "attempts remaining **including the current one**". When `remaining == 1` the engine is already on its last attempt; after a failure it must not retry. The current check retries on `remaining == 1`, producing `max_attempts + 1` total action invocations for every policy.

## Evidence

`crates/vb_runtime/src/primitives/retry/state.rs:14-19` documents `remaining` as "How many attempts remain (including the current one)." `from_policy` (state.rs:26-32) seeds `remaining = policy.max_attempts()` and `current_attempt = 1`.

`evaluate_retry` (retry.rs:84-106):

```rust
if state.remaining() == 0 {
    return RetryDecision::Exhausted { max_attempts: policy.max_attempts() };
}
let new_remaining = state.remaining().saturating_sub(1);
let new_attempt   = state.current_attempt().saturating_add(1);
let delay_ms      = compute_delay(policy, state.current_attempt());
let new_state     = RetryState::new(new_attempt, new_remaining, delay_ms);
RetryDecision::Retry { state: new_state, delay_ms }
```

Trace for `max_attempts = 3`:

| call | state in        | new state out   | action runs? |
|------|-----------------|-----------------|--------------|
| 1    | (attempt=1, rem=3) | (2, 2)       | yes (attempt 2) |
| 2    | (attempt=2, rem=2) | (3, 1)       | yes (attempt 3) |
| 3    | (attempt=3, rem=1) | (4, 0)       | yes (attempt 4) — BUG |
| 4    | (attempt=4, rem=0) | Exhausted    | no           |

Total action executions = 4 for `max_attempts = 3`. For `max_attempts = 1` the engine performs 2 attempts even though `RetryPolicy::new` documents "A value of 1 means 'try once, never retry'" (`policy.rs:21-22`).

The authoritative sibling implementation `engine/retry_math.rs:101` gets this right:

```rust
if cursor.exhausted || cursor.remaining <= 1 {
    return Ok(RetryCursor { remaining: 0, exhausted: true, ..cursor });
}
```

The two retry implementations disagree, proving one is wrong.

## Adversarial Check

Three alternative readings were considered and rejected:

1. *"`remaining` excludes the current attempt."* — Refuted by the field doc ("including the current one") and by `from_policy` seeding `remaining = max_attempts` while `current_attempt = 1`; if `remaining` excluded current, `from_policy` would seed `max_attempts - 1`.
2. *"Callers compensate by pre-decrementing."* — `retry_on_failure` (retry.rs:130-143) calls `evaluate_retry` directly on the slot state with no pre-decrement.
3. *"This is dead code; the bug is unreachable."* — `retry_on_failure` is `pub`, exported via `primitives::retry`, and is invoked from production paths (`workspace_tests/tests/timer_deadline_primitive_tests.rs:163` imports it for the timer-deadline primitive). The function is the canonical retry stepper for any host that drives retries through `RetryState`.

The severity is High rather than Critical because retry exhaustion produces an `Exhausted` decision (not a panic), but the contract violation is real: every retried action performs one extra attempt, which for side-effecting non-idempotent actions (the exact case `RetrySafety::RequiresIdempotencyKey` exists for) can double-write.

## Suggested Fix

Match the `retry_math.rs` semantics. Either:

```rust
if state.remaining() <= 1 {
    return RetryDecision::Exhausted { max_attempts: policy.max_attempts() };
}
```

or seed `from_policy` with `remaining = policy.max_attempts().saturating_sub(1)` and keep the `== 0` check. The former is the smaller diff and mirrors `retry_math::RetryPolicy::next_cursor` exactly. Add a Kani harness proving `evaluate_retry` returns `Retry` at most `max_attempts - 1` times for arbitrary `max_attempts ∈ 1..=u16::MAX`.
