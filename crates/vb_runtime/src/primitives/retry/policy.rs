#![forbid(unsafe_code)]
//! Retry policy configuration types.

/// Delay strategy applied between retry attempts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DelayStrategy {
    /// No delay between attempts.
    None,
    /// Fixed delay in milliseconds between each attempt.
    Fixed,
    /// Exponential backoff: delay doubles each attempt, starting from `delay_ms`.
    ExponentialBackoff,
}

/// Configurable retry policy with bounded attempts, delay, and backoff.
///
/// Sensible defaults: 3 max attempts, 100ms delay, no backoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Maximum number of attempts (including the initial attempt).
    /// Must be at least 1. A value of 1 means "try once, never retry".
    max_attempts: u16,
    /// Base delay in milliseconds between attempts.
    delay_ms: u32,
    /// Backoff multiplier applied to delay for exponential backoff.
    /// A value of 1 means no scaling (fixed delay). A value of 2 means
    /// each subsequent delay is twice the previous.
    backoff_multiplier: u32,
    /// The delay strategy to use.
    strategy: DelayStrategy,
}

impl RetryPolicy {
    /// Creates a new retry policy with full configuration.
    ///
    /// Returns an error if `max_attempts` is zero (at least one attempt is required).
    pub fn new(
        max_attempts: u16,
        delay_ms: u32,
        backoff_multiplier: u32,
        strategy: DelayStrategy,
    ) -> Result<Self, RetryPolicyError> {
        if max_attempts == 0 {
            return Err(RetryPolicyError::ZeroMaxAttempts);
        }
        if backoff_multiplier == 0 {
            return Err(RetryPolicyError::ZeroBackoffMultiplier);
        }
        Ok(Self {
            max_attempts,
            delay_ms,
            backoff_multiplier,
            strategy,
        })
    }

    /// Returns a policy that never retries (single attempt).
    #[must_use]
    pub fn no_retry() -> Self {
        Self {
            max_attempts: 1,
            delay_ms: 0,
            backoff_multiplier: 1,
            strategy: DelayStrategy::None,
        }
    }

    /// Returns the default retry policy: 3 attempts, 100ms fixed delay.
    #[must_use]
    pub fn default_policy() -> Self {
        Self {
            max_attempts: 3,
            delay_ms: 100,
            backoff_multiplier: 1,
            strategy: DelayStrategy::Fixed,
        }
    }

    /// Returns the maximum number of attempts.
    #[must_use]
    pub const fn max_attempts(&self) -> u16 {
        self.max_attempts
    }

    /// Returns the base delay in milliseconds.
    #[must_use]
    pub const fn delay_ms(&self) -> u32 {
        self.delay_ms
    }

    /// Returns the backoff multiplier.
    #[must_use]
    pub const fn backoff_multiplier(&self) -> u32 {
        self.backoff_multiplier
    }

    /// Returns the delay strategy.
    #[must_use]
    pub const fn strategy(&self) -> DelayStrategy {
        self.strategy
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::default_policy()
    }
}

/// Errors from retry policy construction and evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RetryPolicyError {
    /// `max_attempts` was set to zero, which is invalid.
    ZeroMaxAttempts,
    /// `backoff_multiplier` was set to zero, which is invalid.
    ZeroBackoffMultiplier,
    /// The retry state slot did not contain a valid I64.
    InvalidRetrySlotType {
        /// Expected type name.
        expected: &'static str,
        /// Found type name.
        found: &'static str,
    },
    /// The retry state contained an internally inconsistent value.
    InvalidRetryState,
    /// The action failure is not retriable.
    NotRetriable,
}
