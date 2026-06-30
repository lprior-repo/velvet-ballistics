#![forbid(unsafe_code)]
//! Retry/try_again policy primitives for bounded retries with delay and exhaustion.
//!
//! Provides `RetryPolicy` for configuring retry behavior, `RetryState` for
//! tracking the retry state machine, and enforcement of `ActionFailure.retry_policy`
//! to prevent retrying non-retriable failures.

use vb_core::action::{ActionFailure, RetrySafety};
use vb_core::errors::CoreError;
use vb_core::frame::RunFrame;
use vb_core::ids::SlotIdx;
use vb_core::value::SlotValue;

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

/// Current state of the retry state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryState {
    /// Which attempt we are on (1-indexed, starts at 1).
    current_attempt: u16,
    /// How many attempts remain (including the current one).
    remaining: u16,
    /// The current delay in milliseconds to apply before the next attempt.
    current_delay_ms: u32,
}

impl RetryState {
    /// Creates a new retry state from a policy, ready for the first attempt.
    ///
    /// The state starts with `current_attempt = 1` and `remaining = max_attempts`.
    #[must_use]
    pub fn from_policy(policy: &RetryPolicy) -> Self {
        Self {
            current_attempt: 1,
            remaining: policy.max_attempts(),
            current_delay_ms: 0,
        }
    }

    /// Returns the current attempt number (1-indexed).
    #[must_use]
    pub const fn current_attempt(&self) -> u16 {
        self.current_attempt
    }

    /// Returns the number of remaining attempts.
    #[must_use]
    pub const fn remaining(&self) -> u16 {
        self.remaining
    }

    /// Returns the delay in milliseconds before the next attempt.
    #[must_use]
    pub const fn current_delay_ms(&self) -> u32 {
        self.current_delay_ms
    }

    /// Returns true if all attempts have been exhausted.
    #[must_use]
    pub const fn is_exhausted(&self) -> bool {
        self.remaining == 0
    }

    /// Encodes the retry state into an I64 slot value.
    ///
    /// Layout:
    /// - Bits \[63:32\] = current_delay_ms (u32)
    /// - Bits \[31:16\] = current_attempt (u16)
    /// - Bits \[15:0\]  = remaining (u16)
    ///
    /// All bit ranges fit within the non-negative i64 space since the
    /// maximum delay (u32::MAX) occupies bits \[63:32\] but the high bit
    /// (sign bit) can only be set if delay >= 2^31, which we accept as
    /// a valid encoding that still round-trips correctly through i64.
    pub fn encode(&self) -> Result<i64, RetryPolicyError> {
        let delay_high = i64::from(self.current_delay_ms)
            .checked_shl(32)
            .ok_or(RetryPolicyError::InvalidRetryState)?;
        let attempt_mid = i64::from(self.current_attempt)
            .checked_shl(16)
            .ok_or(RetryPolicyError::InvalidRetryState)?;
        let remaining_low = i64::from(self.remaining);
        delay_high
            .checked_add(attempt_mid)
            .and_then(|v| v.checked_add(remaining_low))
            .ok_or(RetryPolicyError::InvalidRetryState)
    }

    /// Decodes a retry state from an I64 slot value.
    pub fn decode(packed: i64) -> Result<Self, RetryPolicyError> {
        // Use bitwise operations on the i64 directly to avoid sign issues.
        let current_delay_ms = u32::try_from((packed >> 32) & 0xFFFF_FFFF_i64)
            .map_err(|_| RetryPolicyError::InvalidRetryState)?;
        let current_attempt = u16::try_from((packed >> 16) & 0xFFFF_i64)
            .map_err(|_| RetryPolicyError::InvalidRetryState)?;
        let remaining =
            u16::try_from(packed & 0xFFFF_i64).map_err(|_| RetryPolicyError::InvalidRetryState)?;
        // current_attempt must be >= 1 unless this is a zero-initialized state
        if current_attempt == 0 && remaining > 0 {
            return Err(RetryPolicyError::InvalidRetryState);
        }
        Ok(Self {
            current_attempt,
            remaining,
            current_delay_ms,
        })
    }

    /// Writes the retry state to a frame slot.
    pub fn write_to_slot(
        &self,
        frame: &mut RunFrame,
        slot: SlotIdx,
    ) -> Result<(), RetryPolicyError> {
        let packed = self.encode()?;
        frame
            .write_slot(slot, SlotValue::I64(packed))
            .map_err(|_| RetryPolicyError::InvalidRetryState)
    }

    /// Reads the retry state from a frame slot.
    pub fn read_from_slot(frame: &RunFrame, slot: SlotIdx) -> Result<Self, RetryPolicyError> {
        let value = frame
            .read_slot(slot)
            .map_err(|_| RetryPolicyError::InvalidRetryState)?;
        match *value {
            SlotValue::I64(packed) => Self::decode(packed),
            ref other => Err(RetryPolicyError::InvalidRetrySlotType {
                expected: "number",
                found: other.type_name(),
            }),
        }
    }
}

/// Outcome of evaluating whether a retry should proceed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RetryDecision {
    /// The failure can be retried; this is the updated state and delay.
    Retry {
        /// Updated retry state after decrementing remaining.
        state: RetryState,
        /// Delay in milliseconds to wait before the next attempt.
        delay_ms: u32,
    },
    /// All attempts exhausted. The step should be marked failed.
    Exhausted {
        /// The max_attempts from the original policy.
        max_attempts: u16,
    },
    /// The failure is not retriable. Must not retry regardless of policy.
    NotRetriable,
}

/// Checks whether a failure is retriable given the action's retry safety.
///
/// - `RetrySafety::Safe`: always retriable if the failure's `retry_policy` is `Retryable`.
/// - `RetrySafety::KeyRequired`: retriable if `retry_policy` is `Retryable` (key check is done
///   separately during dispatch via `verify_idempotency`).
/// - `RetrySafety::Unsafe`: never retriable regardless of the failure's policy.
pub fn is_failure_retriable(failure: &ActionFailure, retry_safety: RetrySafety) -> bool {
    match retry_safety {
        RetrySafety::Unsafe => false,
        RetrySafety::Safe | RetrySafety::KeyRequired => {
            failure.retry_policy == vb_core::action::RetryPolicy::Retryable
        }
        // Handle any future RetrySafety variants as not retriable (safest default).
        #[allow(unreachable_code)]
        _ => false,
    }
}

/// Evaluates a retry decision given the current state, policy, and failure.
///
/// This is the core retry state machine transition:
/// 1. Check if the failure is retriable (combines `ActionFailure.retry_policy` and
///    `RetrySafety`). Non-retriable failures produce `RetryDecision::NotRetriable`.
/// 2. If retriable and attempts remain, decrement remaining and compute delay.
/// 3. If retriable but no attempts remain, produce `RetryDecision::Exhausted`.
pub fn evaluate_retry(
    state: &RetryState,
    policy: &RetryPolicy,
    failure: &ActionFailure,
    retry_safety: RetrySafety,
) -> RetryDecision {
    if !is_failure_retriable(failure, retry_safety) {
        return RetryDecision::NotRetriable;
    }

    if state.remaining == 0 {
        return RetryDecision::Exhausted {
            max_attempts: policy.max_attempts(),
        };
    }

    // Decrement remaining and compute next state.
    let new_remaining = state.remaining.saturating_sub(1);
    let new_attempt = state.current_attempt.saturating_add(1);
    let delay_ms = compute_delay(policy, state.current_attempt);

    let new_state = RetryState {
        current_attempt: new_attempt,
        remaining: new_remaining,
        current_delay_ms: delay_ms,
    };

    RetryDecision::Retry {
        state: new_state,
        delay_ms,
    }
}

/// Computes the delay for the next attempt based on policy and current attempt number.
///
/// - `DelayStrategy::None`: always 0.
/// - `DelayStrategy::Fixed`: always `policy.delay_ms`.
/// - `DelayStrategy::ExponentialBackoff`: `delay_ms * backoff_multiplier^(attempt-1)`,
///   saturating at `u32::MAX`.
pub fn compute_delay(policy: &RetryPolicy, current_attempt: u16) -> u32 {
    match policy.strategy() {
        DelayStrategy::None => 0,
        DelayStrategy::Fixed => policy.delay_ms(),
        DelayStrategy::ExponentialBackoff => {
            // For attempt N, the delay before attempt N+1 is:
            // base * multiplier^(N-1) where N is the attempt just completed.
            let exponent = if current_attempt > 0 {
                u32::from(current_attempt.saturating_sub(1))
            } else {
                0
            };
            let mut delay = policy.delay_ms();
            let multiplier = policy.backoff_multiplier();
            let mut i: u32 = 0;
            while i < exponent {
                delay = match delay.checked_mul(multiplier) {
                    Some(d) => d,
                    None => {
                        return u32::MAX;
                    }
                };
                i = i.saturating_add(1);
            }
            delay
        }
    }
}

/// Handles exhaustion by producing a `CoreError::RepeatExhausted` with the
/// retry-exhausted diagnostic information.
pub fn exhaustion_error(max_attempts: u16) -> CoreError {
    CoreError::RepeatExhausted { max: max_attempts }
}

/// High-level retry step handler: initializes retry state and writes it to a slot.
///
/// This is used by the TryAgain start node to set up the retry state machine.
pub fn retry_start(
    run: &mut RunFrame,
    policy: &RetryPolicy,
    slot: SlotIdx,
) -> Result<(), RetryPolicyError> {
    let state = RetryState::from_policy(policy);
    state.write_to_slot(run, slot)
}

/// High-level retry step handler: advances the retry state after a failure.
///
/// Returns the retry decision. If `RetryDecision::Retry`, the updated state is
/// written back to the slot. If `Exhausted` or `NotRetriable`, the slot is not modified.
pub fn retry_on_failure(
    run: &mut RunFrame,
    slot: SlotIdx,
    policy: &RetryPolicy,
    failure: &ActionFailure,
    retry_safety: RetrySafety,
) -> Result<RetryDecision, RetryPolicyError> {
    let state = RetryState::read_from_slot(run, slot)?;
    let decision = evaluate_retry(&state, policy, failure, retry_safety);
    if let RetryDecision::Retry { ref state, .. } = decision {
        state.write_to_slot(run, slot)?;
    }
    Ok(decision)
}

#[cfg(test)]
#[path = "retry/tests.rs"]
mod tests;
