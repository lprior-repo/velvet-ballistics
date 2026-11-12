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

// =====================================================================
// Retry cursor iterator: one cursor per admitted attempt.
// =====================================================================

/// The iterator must yield exactly `max_attempts` cursors, starting at
/// attempt 1 with no preceding delay.
#[test]
fn cursors_yield_one_cursor_per_attempt() -> Result<(), RetryPolicyMathError> {
    let policy = RetryPolicy {
        max_attempts: 3,
        base_delay_ms: 100,
        exponential_backoff: false,
    };
    let cursors: Vec<_> = policy.cursors(MAX_INTERVAL_MS)?.collect();
    assert_eq!(cursors.len(), 3);
    assert_eq!(
        cursors
            .iter()
            .map(|cursor| cursor.attempt)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(
        cursors
            .iter()
            .map(|cursor| cursor.delay_ms)
            .collect::<Vec<_>>(),
        vec![0, 100, 100]
    );
    Ok(())
}

/// Exponential policies must expose the doubling delays through the
/// iterator, including the final exhausted cursor shape.
#[test]
fn cursors_apply_exponential_backoff() -> Result<(), RetryPolicyMathError> {
    let policy = RetryPolicy {
        max_attempts: 4,
        base_delay_ms: 100,
        exponential_backoff: true,
    };
    let delays: Vec<_> = policy
        .cursors(MAX_INTERVAL_MS)?
        .map(|cursor| cursor.delay_ms)
        .collect();
    assert_eq!(delays, vec![0, 100, 200, 400]);
    Ok(())
}

/// The iterator must respect the caller's timer budget cap.
#[test]
fn cursors_clamp_delays_to_max_interval() -> Result<(), RetryPolicyMathError> {
    let policy = RetryPolicy {
        max_attempts: 5,
        base_delay_ms: 100,
        exponential_backoff: true,
    };
    let cursors: Vec<_> = policy.cursors(250)?.collect();
    assert_eq!(
        cursors.len(),
        usize::from(policy.max_attempts),
        "clamping must not change the attempt count"
    );
    assert!(
        cursors.iter().all(|cursor| cursor.delay_ms <= 250),
        "cursor delay exceeded the interval cap: {cursors:?}"
    );
    assert_eq!(
        cursors
            .iter()
            .map(|cursor| cursor.attempt)
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5],
        "clamping must not skip or duplicate attempts"
    );
    Ok(())
}

/// A zero-attempt policy is invalid and must be rejected up front rather
/// than yielding an empty iterator that hides the bad config.
#[test]
fn cursors_reject_zero_attempts() {
    let policy = RetryPolicy {
        max_attempts: 0,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    let result = policy.cursors(MAX_INTERVAL_MS);
    assert!(
        matches!(result, Err(RetryPolicyMathError::ZeroMaxAttempts)),
        "expected Err(ZeroMaxAttempts), got {:?}",
        result
    );
}

/// `size_hint` must stay exact so `ExactSizeIterator` consumers are safe.
#[test]
fn cursors_size_hint_is_exact() -> Result<(), RetryPolicyMathError> {
    let policy = RetryPolicy {
        max_attempts: 3,
        base_delay_ms: 10,
        exponential_backoff: false,
    };
    let mut iter = policy.cursors(MAX_INTERVAL_MS)?;
    assert_eq!(iter.len(), iter.clone().count());
    assert_eq!(iter.size_hint(), (iter.len(), Some(iter.len())));
    assert_eq!(iter.len(), 3);
    let _first = iter.next();
    assert_eq!(iter.len(), 2);
    let _second = iter.next();
    let _third = iter.next();
    assert_eq!(iter.len(), 0);
    assert!(iter.next().is_none());
    Ok(())
}

// =====================================================================
// Jitter: delays stay inside the un-jittered bound.
// =====================================================================

/// Jittered delays must fall in `[0, delay_for_attempt)`.
#[test]
fn jittered_delay_stays_within_base_delay() -> Result<(), RetryPolicyMathError> {
    let policy = RetryPolicy {
        max_attempts: 4,
        base_delay_ms: 1_000,
        exponential_backoff: true,
    };
    let mut rng = rand::rng();
    let mut saw_nonzero = false;
    for attempt in 1..=4 {
        let base = policy.delay_for_attempt(MAX_INTERVAL_MS, attempt)?;
        for _ in 0..64 {
            let jittered = policy.jittered_delay_for_attempt(MAX_INTERVAL_MS, attempt, &mut rng)?;
            assert!(
                jittered < base || base == 0,
                "jittered delay {jittered} not below base {base}"
            );
            saw_nonzero = saw_nonzero || jittered > 0;
        }
    }
    // A jitter that always returned the lower bound would satisfy every bound
    // above; at least one draw must land above zero.
    assert!(saw_nonzero, "jitter returned zero for every draw");
    Ok(())
}

/// A zero base delay has no span to jitter, so it must stay zero.
#[test]
fn jittered_delay_with_zero_base_stays_zero() -> Result<(), RetryPolicyMathError> {
    let policy = RetryPolicy {
        max_attempts: 2,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    let mut rng = rand::rng();
    assert_eq!(
        policy.jittered_delay_for_attempt(MAX_INTERVAL_MS, 1, &mut rng)?,
        0
    );
    Ok(())
}

/// `jittered_next_cursor` must preserve the cursor's attempt bookkeeping
/// and only replace the delay.
#[test]
fn jittered_next_cursor_preserves_attempt_identity() -> Result<(), RetryPolicyMathError> {
    let policy = RetryPolicy {
        max_attempts: 3,
        base_delay_ms: 500,
        exponential_backoff: false,
    };
    let mut rng = rand::rng();
    let initial = policy.initial_cursor()?;
    let next = policy.jittered_next_cursor(MAX_INTERVAL_MS, initial, &mut rng)?;
    assert_eq!(next.attempt, 2);
    assert_eq!(next.remaining, 2);
    assert!(next.delay_ms < 500);
    Ok(())
}
