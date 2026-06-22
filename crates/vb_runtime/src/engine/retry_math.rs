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
    /// Cursor's claimed remaining attempts cannot fit within the policy
    /// maximum, given the current attempt. Returned when a public cursor
    /// declares an attempt window that would advance past `max_attempts`.
    #[error("retry cursor inconsistent: attempt window exceeds max_attempts")]
    CursorInconsistent,
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
        // Checked advance: a saturating_add(1) at u16::MAX would silently
        // duplicate the previous attempt. Use checked arithmetic so an
        // overflow is reported as a typed error rather than producing a
        // plausible-but-wrong cursor.
        let next_attempt = cursor
            .attempt
            .checked_add(1)
            .ok_or(RetryPolicyMathError::CursorInconsistent)?;
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
        // Reject cursors whose claimed attempt window cannot fit within
        // the policy maximum. A non-exhausted cursor with `remaining`
        // attempts starting at `attempt` must satisfy
        // `attempt + remaining - 1 <= max_attempts`. Catching this here
        // prevents a subsequent `next_cursor` from being asked to
        // advance past the policy limit via saturating arithmetic.
        if cursor.remaining > 0 {
            let last_attempt = match cursor
                .attempt
                .checked_add(cursor.remaining.saturating_sub(1))
            {
                Some(value) => value,
                None => return Err(RetryPolicyMathError::CursorInconsistent),
            };
            if last_attempt > self.max_attempts {
                return Err(RetryPolicyMathError::CursorInconsistent);
            }
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
