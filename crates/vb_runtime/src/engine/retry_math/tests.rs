//! Regression tests for RE-012: retry cursor advancement must never
//! duplicate the maximum attempt, and the cursor invariant
//! `attempt + remaining - 1 <= max_attempts` must be upheld for every
//! non-exhausted cursor the production path accepts.

use super::{RetryCursor, RetryPolicy, RetryPolicyMathError};

const MAX_INTERVAL_MS: u64 = u64::MAX;

/// Regression: RE-012 — a public `RetryCursor` whose attempt/remaining
/// window already exceeds `max_attempts` must be rejected with a typed
/// error instead of being advanced by `saturating_add(1)` into a
/// duplicate-attempt cursor.
#[test]
fn next_cursor_rejects_cursor_whose_window_exceeds_max_attempts() {
    let policy = RetryPolicy {
        max_attempts: 3,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    // attempt + remaining - 1 = 3 + 2 - 1 = 4 > max_attempts (3).
    let inconsistent_cursor = RetryCursor {
        attempt: 3,
        remaining: 2,
        delay_ms: 0,
        exhausted: false,
    };
    match policy.next_cursor(MAX_INTERVAL_MS, inconsistent_cursor) {
        Err(RetryPolicyMathError::InconsistentCursor) => {}
        other => panic!("expected Err(InconsistentCursor), got {:?}", other),
    }
}

/// Regression: RE-012 — at the `u16::MAX` boundary where `saturating_add(1)`
/// would saturate, `next_cursor` must surface a typed error rather than
/// silently returning a cursor with the same attempt number.
#[test]
fn next_cursor_at_u16_max_does_not_duplicate_attempt() {
    let policy = RetryPolicy {
        max_attempts: u16::MAX,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    let boundary_cursor = RetryCursor {
        attempt: u16::MAX,
        remaining: 2,
        delay_ms: 0,
        exhausted: false,
    };
    let result = policy.next_cursor(MAX_INTERVAL_MS, boundary_cursor);
    assert!(
        matches!(result, Err(RetryPolicyMathError::InconsistentCursor)),
        "expected Err(InconsistentCursor), got {:?}",
        result
    );
}

/// Regression: RE-012 — a non-exhausted cursor with `remaining == 0`
/// violates the invariant that an exhausted cursor must report zero
/// remaining. The production path must reject such cursors with a typed
/// error rather than silently transitioning them.
#[test]
fn next_cursor_rejects_non_exhausted_with_zero_remaining() {
    let policy = RetryPolicy::DEFAULT;
    let inconsistent_cursor = RetryCursor {
        attempt: 1,
        remaining: 0,
        delay_ms: 0,
        exhausted: false,
    };
    let result = policy.next_cursor(MAX_INTERVAL_MS, inconsistent_cursor);
    assert!(
        matches!(result, Err(RetryPolicyMathError::InconsistentCursor)),
        "expected Err(InconsistentCursor), got {:?}",
        result
    );
}

/// Regression: RE-012 — the happy path for a consistent cursor must
/// continue to advance by one attempt without regression.
#[test]
fn next_cursor_advances_consistent_cursor_by_one() -> Result<(), RetryPolicyMathError> {
    let policy = RetryPolicy::DEFAULT;
    let cursor = policy.initial_cursor()?;
    let advanced = policy.next_cursor(MAX_INTERVAL_MS, cursor)?;
    assert_eq!(advanced.attempt, 2);
    assert_eq!(advanced.remaining, 2);
    assert!(!advanced.exhausted);
    Ok(())
}

// =====================================================================
// RE-013 regression tests: initial_cursor must reject zero-attempt
// policies instead of silently returning a cursor with exhausted=true.
// =====================================================================

/// Regression: RE-013 — a policy with `max_attempts == 0` must be
/// rejected by `initial_cursor` with the same typed error as
/// `validate_against`. The previous behavior silently produced a cursor
/// with `exhausted: true`, which the engine then treated as legitimate
/// (silent corruption).
#[test]
fn initial_cursor_rejects_zero_max_attempts() {
    let policy = RetryPolicy {
        max_attempts: 0,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    assert_eq!(
        policy.initial_cursor(),
        Err(RetryPolicyMathError::ZeroMaxAttempts),
    );
}

/// Regression: RE-013 — `RetryPolicy::DEFAULT` must produce a valid
/// non-exhausted initial cursor with the documented `attempt=1` and
/// `remaining=max_attempts` invariant.
#[test]
fn initial_cursor_default_policy_is_not_exhausted() -> Result<(), RetryPolicyMathError> {
    let cursor = RetryPolicy::DEFAULT.initial_cursor()?;
    assert_eq!(cursor.attempt, 1);
    assert_eq!(cursor.remaining, 3);
    assert_eq!(cursor.delay_ms, 0);
    assert!(!cursor.exhausted);
    Ok(())
}

/// Regression: RE-013 — `RetryPolicy::NEVER` has `max_attempts=1` and
/// must produce a non-exhausted cursor (one attempt is allowed).
#[test]
fn initial_cursor_never_policy_is_not_exhausted() -> Result<(), RetryPolicyMathError> {
    let cursor = RetryPolicy::NEVER.initial_cursor()?;
    assert_eq!(cursor.attempt, 1);
    assert_eq!(cursor.remaining, 1);
    assert_eq!(cursor.delay_ms, 0);
    assert!(!cursor.exhausted);
    Ok(())
}

/// Regression: RE-012 — `fast_forward_cursor` must surface the typed
/// error when the cursor it is asked to advance is internally
/// inconsistent.
#[test]
fn fast_forward_cursor_rejects_inconsistent_cursor() {
    let policy = RetryPolicy {
        max_attempts: 2,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    let inconsistent_cursor = RetryCursor {
        attempt: 2,
        remaining: 2,
        delay_ms: 0,
        exhausted: false,
    };
    let result = policy.fast_forward_cursor(MAX_INTERVAL_MS, inconsistent_cursor, 1);
    assert!(
        matches!(result, Err(RetryPolicyMathError::InconsistentCursor)),
        "expected Err(InconsistentCursor), got {:?}",
        result
    );
}
