#![forbid(unsafe_code)]

//! Pure retry policy calculations.

use crate::engine::types::RetryPolicy;

/// Bounded resource limits used when accepting a retry policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicyLimits {
    /// Maximum attempts admitted by the caller's resource budget.
    pub max_attempts: u16,
    /// Maximum delay interval admitted by the caller's timer budget.
    pub max_interval_ms: u64,
}

/// One-based retry cursor used by pure scheduling calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryCursor {
    /// Current one-based attempt.
    pub attempt: u16,
    /// Attempts remaining including the current attempt.
    pub remaining: u16,
    /// Delay that precedes the current retry attempt.
    pub delay_ms: u64,
    /// True once no further attempts can be scheduled.
    pub exhausted: bool,
}

/// Rejected retry policy or cursor inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RetryPolicyMathError {
    /// Retry policies must admit at least one attempt.
    #[error("retry max_attempts must be nonzero")]
    ZeroMaxAttempts,
    /// Retry policy exceeded its caller-supplied attempt budget.
    #[error("retry max_attempts exceeded resource limit")]
    MaxAttemptsExceeded,
    /// Base delay exceeded the caller-supplied timer interval.
    #[error("retry base_delay_ms exceeded max interval")]
    BaseDelayExceeded,
    /// Retry attempt indices are one-based.
    #[error("retry attempt must be one-based")]
    ZeroAttempt,
    /// Retry attempt exceeded the policy maximum.
    #[error("retry attempt exceeded max_attempts")]
    AttemptExceeded,
    /// Cursor delay exceeded the caller-supplied timer interval.
    #[error("retry cursor delay exceeded max interval")]
    CursorDelayExceeded,
    /// Cursor remaining attempts exceeded the policy maximum.
    #[error("retry cursor remaining exceeded max_attempts")]
    RemainingExceeded,
    /// Cursor attempt/remaining window violated the invariant
    /// `attempt + remaining - 1 <= max_attempts` for a non-exhausted cursor.
    #[error("retry cursor attempt/remaining window exceeds max_attempts")]
    InconsistentCursor,
}

impl RetryPolicy {
    /// Accepts a retry policy only when it fits caller-supplied resource limits.
    pub const fn validate_against(
        self,
        limits: RetryPolicyLimits,
    ) -> Result<Self, RetryPolicyMathError> {
        if self.max_attempts == 0 {
            return Err(RetryPolicyMathError::ZeroMaxAttempts);
        }
        if self.max_attempts > limits.max_attempts {
            return Err(RetryPolicyMathError::MaxAttemptsExceeded);
        }
        if self.base_delay_ms > limits.max_interval_ms {
            return Err(RetryPolicyMathError::BaseDelayExceeded);
        }
        Ok(self)
    }

    /// Returns the bounded delay for a one-based attempt.
    pub fn delay_for_attempt(
        self,
        max_interval_ms: u64,
        attempt: u16,
    ) -> Result<u64, RetryPolicyMathError> {
        self.validate_attempt(attempt)?;
        Ok(self.delay_after_valid_attempt(max_interval_ms, attempt))
    }

    /// Builds the initial retry cursor for this policy.
    #[must_use]
    pub const fn initial_cursor(self) -> RetryCursor {
        RetryCursor {
            attempt: 1,
            remaining: self.max_attempts,
            delay_ms: 0,
            exhausted: self.max_attempts == 0,
        }
    }

    /// Advances a retry cursor by one attempt.
    pub fn next_cursor(
        self,
        max_interval_ms: u64,
        cursor: RetryCursor,
    ) -> Result<RetryCursor, RetryPolicyMathError> {
        self.validate_cursor(max_interval_ms, cursor)?;
        if cursor.exhausted || cursor.remaining <= 1 {
            return Ok(RetryCursor {
                remaining: 0,
                exhausted: true,
                ..cursor
            });
        }
        // The cursor invariant (enforced by `validate_cursor`) implies
        // `attempt + remaining - 1 <= max_attempts`. Combined with
        // `remaining > 1` (guarded above) we have
        // `attempt + 1 <= max_attempts`, so the checked add is defense in
        // depth rather than an overflow path.
        let next_attempt = cursor
            .attempt
            .checked_add(1)
            .ok_or(RetryPolicyMathError::InconsistentCursor)?;
        self.validate_attempt(next_attempt)?;
        Ok(RetryCursor {
            attempt: next_attempt,
            remaining: cursor.remaining.saturating_sub(1),
            delay_ms: self.delay_after_valid_attempt(max_interval_ms, cursor.attempt),
            exhausted: false,
        })
    }

    /// Advances a retry cursor by `count` attempts or until exhaustion.
    pub fn fast_forward_cursor(
        self,
        max_interval_ms: u64,
        cursor: RetryCursor,
        count: u16,
    ) -> Result<RetryCursor, RetryPolicyMathError> {
        (0..count).try_fold(cursor, |current, _| {
            if current.exhausted {
                Ok(current)
            } else {
                self.next_cursor(max_interval_ms, current)
            }
        })
    }

    const fn validate_attempt(self, attempt: u16) -> Result<u16, RetryPolicyMathError> {
        if attempt == 0 {
            return Err(RetryPolicyMathError::ZeroAttempt);
        }
        if attempt > self.max_attempts {
            return Err(RetryPolicyMathError::AttemptExceeded);
        }
        Ok(attempt)
    }

    const fn validate_cursor(
        self,
        max_interval_ms: u64,
        cursor: RetryCursor,
    ) -> Result<RetryCursor, RetryPolicyMathError> {
        if cursor.delay_ms > max_interval_ms {
            return Err(RetryPolicyMathError::CursorDelayExceeded);
        }
        if cursor.remaining > self.max_attempts {
            return Err(RetryPolicyMathError::RemainingExceeded);
        }
        if cursor.exhausted {
            return Ok(cursor);
        }
        // For non-exhausted cursors enforce the cursor invariants:
        //   * `remaining > 0` — a non-exhausted cursor must still owe at
        //     least one attempt; zero remaining is the exhausted state.
        //   * `attempt + remaining - 1 <= max_attempts` — the highest
        //     attempt the cursor could still reach must not exceed the
        //     policy ceiling. Without this check a caller-constructed
        //     cursor could be silently advanced past `max_attempts` via
        //     saturating arithmetic (RE-012).
        if cursor.remaining == 0 {
            return Err(RetryPolicyMathError::InconsistentCursor);
        }
        match cursor.attempt.checked_add(cursor.remaining - 1) {
            Some(last_attempt) if last_attempt <= self.max_attempts => {}
            _ => return Err(RetryPolicyMathError::InconsistentCursor),
        }
        match self.validate_attempt(cursor.attempt) {
            Ok(_) => Ok(cursor),
            Err(error) => Err(error),
        }
    }

    /// Computes delay using exponential backoff with base 2.
    ///
    /// WHY base 2: Standard exponential backoff uses base 2 for good convergence
    /// while avoiding thundering herd. Each attempt doubles the wait time.
    ///
    /// WHY saturating_mul: Prevents overflow when delay exceeds u64::MAX.
    /// When max_interval_ms is hit, we clamp rather than wrap.
    fn delay_after_valid_attempt(self, max_interval_ms: u64, attempt: u16) -> u64 {
        if !self.exponential_backoff {
            return self.base_delay_ms.min(max_interval_ms);
        }
        (1..attempt).fold(self.base_delay_ms, |delay, _| {
            delay
                .checked_mul(2)
                .map_or(max_interval_ms, |next| next.min(max_interval_ms))
        })
    }
}

#[cfg(test)]
mod tests {
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
    fn next_cursor_advances_consistent_cursor_by_one() {
        let policy = RetryPolicy::DEFAULT;
        let cursor = policy.initial_cursor();
        let advanced = policy
            .next_cursor(MAX_INTERVAL_MS, cursor)
            .expect("consistent cursor must advance");
        assert_eq!(advanced.attempt, 2);
        assert_eq!(advanced.remaining, 2);
        assert!(!advanced.exhausted);
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
}
