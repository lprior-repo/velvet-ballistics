#![forbid(unsafe_code)]

//! Retry math gap coverage for `RetryCursor`, `RetryPolicyLimits`, and `RetryPolicy`.
//!
//! Verified gaps covered:
//!
//! - `RetryCursor::initial_cursor` with max_attempts=0 (exhausted)
//! - `RetryCursor::next_cursor` normal progression across attempts
//! - `RetryCursor::next_cursor` idempotent on exhausted cursor
//! - `RetryCursor::fast_forward_cursor` with count=0 (no-op)
//! - `RetryCursor::fast_forward_cursor` count exceeding remaining → exhausted
//! - `RetryPolicyLimits::validate_against` exact boundary match → Ok
//! - `RetryPolicyLimits::validate_against` over-limit → Err
//! - `RetryPolicy::delay_for_attempt` exponential backoff progression
//! - `RetryPolicy::delay_for_attempt` cap enforcement at high attempts

use crate::engine::retry_math::{RetryCursor, RetryPolicyLimits, RetryPolicyMathError};
use crate::engine::types::RetryPolicy;

// =====================================================================
// Gap 1: RetryCursor::initial_cursor with max_attempts=0
// =====================================================================

#[test]
fn initial_cursor_max_attempts_zero_is_exhausted() {
    let policy = RetryPolicy {
        max_attempts: 0,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    let cursor = policy.initial_cursor();

    assert_eq!(cursor.attempt, 1, "attempt should start at 1");
    assert_eq!(
        cursor.remaining, 0,
        "remaining should be 0 when max_attempts=0"
    );
    assert_eq!(cursor.delay_ms, 0, "initial delay should be 0");
    assert!(
        cursor.is_exhausted(),
        "cursor should be exhausted when max_attempts=0"
    );
}

// =====================================================================
// Gap 2: RetryCursor::next_cursor normal progression
// =====================================================================

#[test]
fn next_cursor_progresses_from_attempt_1_through_3() {
    let policy = RetryPolicy {
        max_attempts: 3,
        base_delay_ms: 100,
        exponential_backoff: true,
    };
    let cursor = policy.initial_cursor();

    // Attempt 1 → Attempt 2 (delay = base_delay * 2^0 = 100)
    let c2 = policy
        .next_cursor(10_000, cursor)
        .expect("should advance from attempt 1");
    assert_eq!(c2.attempt, 2);
    assert_eq!(c2.remaining, 2);
    assert_eq!(c2.delay_ms, 100);
    assert!(!c2.exhausted);

    // Attempt 2 → Attempt 3 (delay = base_delay * 2^1 = 200)
    let c3 = policy
        .next_cursor(10_000, c2)
        .expect("should advance from attempt 2");
    assert_eq!(c3.attempt, 3);
    assert_eq!(c3.remaining, 1);
    assert_eq!(c3.delay_ms, 200);
    assert!(!c3.exhausted);

    // Attempt 3 → exhausted (remaining=1, so next_cursor returns unchanged cursor
    // via the `remaining <= 1` guard, keeping attempt=3, delay stays 200)
    let c4 = policy
        .next_cursor(10_000, c3)
        .expect("should advance from attempt 3");
    assert_eq!(
        c4.attempt, 3,
        "exhaustion via remaining<=1 does not increment attempt"
    );
    assert_eq!(c4.remaining, 0);
    assert_eq!(
        c4.delay_ms, 200,
        "delay preserved from cursor when remaining<=1 guard fires"
    );
    assert!(c4.exhausted);
}

// =====================================================================
// Gap 3: RetryCursor::next_cursor idempotent on exhausted cursor
// =====================================================================

#[test]
fn next_cursor_on_exhausted_returns_same_exhausted_state() {
    let policy = RetryPolicy {
        max_attempts: 1,
        base_delay_ms: 50,
        exponential_backoff: false,
    };
    let initial = policy.initial_cursor();
    assert!(!initial.exhausted, "initial cursor should not be exhausted");

    // Advance to exhausted
    let exhausted = policy
        .next_cursor(10_000, initial)
        .expect("first advance should succeed");
    assert!(exhausted.exhausted);
    assert_eq!(exhausted.remaining, 0);

    // Calling next_cursor again on exhausted cursor should be idempotent
    let again = policy
        .next_cursor(10_000, exhausted)
        .expect("exhausted should remain exhausted");

    assert_eq!(again.attempt, exhausted.attempt, "attempt must not change");
    assert_eq!(again.remaining, 0, "remaining stays 0");
    assert!(again.exhausted, "must remain exhausted");
    assert_eq!(again.delay_ms, exhausted.delay_ms, "delay must not change");
}

// =====================================================================
// Gap 4: RetryCursor::fast_forward_cursor with count=0
// =====================================================================

#[test]
fn fast_forward_cursor_zero_count_returns_unchanged() {
    let policy = RetryPolicy {
        max_attempts: 3,
        base_delay_ms: 100,
        exponential_backoff: true,
    };
    let cursor = policy.initial_cursor();
    let original = cursor;

    let result = policy
        .fast_forward_cursor(10_000, cursor, 0)
        .expect("zero count should succeed");

    assert_eq!(result.attempt, original.attempt, "attempt unchanged");
    assert_eq!(result.remaining, original.remaining, "remaining unchanged");
    assert_eq!(result.delay_ms, original.delay_ms, "delay unchanged");
    assert_eq!(
        result.exhausted, original.exhausted,
        "exhausted flag unchanged"
    );
}

// =====================================================================
// Gap 5: RetryCursor::fast_forward_cursor count > remaining → exhausted
// =====================================================================

#[test]
fn fast_forward_cursor_exceeding_remaining_reaches_exhausted() {
    let policy = RetryPolicy {
        max_attempts: 3,
        base_delay_ms: 100,
        exponential_backoff: true,
    };
    let cursor = policy.initial_cursor();

    // Request 5 advances but only 3 attempts are available → should exhaust
    let result = policy
        .fast_forward_cursor(10_000, cursor, 5)
        .expect("should succeed");

    assert!(
        result.exhausted,
        "cursor must be exhausted after exceeding remaining"
    );
    assert_eq!(result.remaining, 0, "remaining must be 0");
    // Last delay computed: attempt 2 → base_delay * 2^1 = 200.
    // When remaining hits 1 the remaining<=1 guard fires and preserves
    // the cursor's delay instead of computing a new one.
    assert_eq!(result.delay_ms, 200, "last computed delay should be 200");
}

// =====================================================================
// Gap 6: RetryPolicyLimits::validate_against exact boundary → Ok
// =====================================================================

#[test]
fn validate_against_exact_boundary_returns_ok() {
    let policy = RetryPolicy {
        max_attempts: 5,
        base_delay_ms: 100,
        exponential_backoff: true,
    };
    let limits = RetryPolicyLimits {
        max_attempts: 5,
        max_interval_ms: 10_000,
    };

    let result = policy.validate_against(limits);
    assert!(
        result.is_ok(),
        "exact boundary max_attempts == limit should be Ok"
    );

    let accepted = result.expect("should have succeeded");
    assert_eq!(accepted.max_attempts, 5);
    assert_eq!(accepted.base_delay_ms, 100);
}

// =====================================================================
// Gap 7: RetryPolicyLimits::validate_against over-limit → Err
// =====================================================================

#[test]
fn validate_against_over_limit_returns_err() {
    let policy = RetryPolicy {
        max_attempts: 6,
        base_delay_ms: 100,
        exponential_backoff: true,
    };
    let limits = RetryPolicyLimits {
        max_attempts: 5,
        max_interval_ms: 10_000,
    };

    let result = policy.validate_against(limits);
    assert!(result.is_err(), "max_attempts > limit should return Err");

    let err = result.expect_err("should have failed");
    assert_eq!(err, RetryPolicyMathError::MaxAttemptsExceeded);
}

// =====================================================================
// Gap 8: RetryPolicy::delay_for_attempt exponential backoff progression
// =====================================================================

#[test]
fn delay_for_attempt_progresses_exponentially_with_backoff() {
    let policy = RetryPolicy {
        max_attempts: 10,
        base_delay_ms: 100,
        exponential_backoff: true,
    };
    let cap = 10_000;

    // attempt 1: base_delay * 2^0 = 100
    let d1 = policy.delay_for_attempt(cap, 1).expect("attempt 1 valid");
    assert_eq!(d1, 100, "attempt 1: base_delay * 2^0 = 100");

    // attempt 2: base_delay * 2^1 = 200
    let d2 = policy.delay_for_attempt(cap, 2).expect("attempt 2 valid");
    assert_eq!(d2, 200, "attempt 2: base_delay * 2^1 = 200");

    // attempt 3: base_delay * 2^2 = 400
    let d3 = policy.delay_for_attempt(cap, 3).expect("attempt 3 valid");
    assert_eq!(d3, 400, "attempt 3: base_delay * 2^2 = 400");

    // attempt 4: base_delay * 2^3 = 800
    let d4 = policy.delay_for_attempt(cap, 4).expect("attempt 4 valid");
    assert_eq!(d4, 800, "attempt 4: base_delay * 2^3 = 800");

    // attempt 10: base_delay * 2^9 = 51200
    let d10 = policy.delay_for_attempt(cap, 10).expect("attempt 10 valid");
    assert_eq!(
        d10, 10_000,
        "attempt 10: 51200 capped to max_interval 10000"
    );
}

// =====================================================================
// Gap 9: RetryPolicy::delay_for_attempt cap at high attempt
// =====================================================================

#[test]
fn delay_for_attempt_does_not_exceed_cap_at_high_attempts() {
    let policy = RetryPolicy {
        max_attempts: 100,
        base_delay_ms: 100,
        exponential_backoff: true,
    };
    let cap = 5_000;

    // Verify that at a very high attempt, the delay stays at cap
    let d90 = policy.delay_for_attempt(cap, 90).expect("attempt 90 valid");
    assert_eq!(
        d90, cap,
        "delay must be capped at max_interval_ms even at high attempt"
    );

    // Also verify at the last valid attempt
    let d100 = policy
        .delay_for_attempt(cap, 100)
        .expect("attempt 100 valid");
    assert_eq!(
        d100, cap,
        "delay must be capped at max_interval_ms at final attempt"
    );

    // Verify non-exponential policy also respects cap
    let flat_policy = RetryPolicy {
        max_attempts: 10,
        base_delay_ms: 9_000,
        exponential_backoff: false,
    };
    let d = flat_policy
        .delay_for_attempt(cap, 5)
        .expect("non-exponential attempt 5 valid");
    assert_eq!(
        d, cap,
        "non-exponential base_delay (9000) should be capped to max_interval (5000)"
    );
}
