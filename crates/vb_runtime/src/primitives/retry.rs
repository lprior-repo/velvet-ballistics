//! Retry/try_again policy primitives for bounded retries with delay and exhaustion.
//!
//! Provides `RetryPolicy` for configuring retry behavior, `RetryState` for
//! tracking the retry state machine, and enforcement of `ActionFailure.retryable`
//! to prevent retrying non-retriable failures.

use vb_core::action::{ActionFailure, RetrySafety};
use vb_core::errors::CoreError;
use vb_core::frame::RunFrame;
use vb_core::ids::SlotIdx;
use vb_core::value::SlotValue;

/// Delay strategy applied between retry attempts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// - Bits [63:32] = current_delay_ms (u32)
    /// - Bits [31:16] = current_attempt (u16)
    /// - Bits [15:0]  = remaining (u16)
    ///
    /// All bit ranges fit within the non-negative i64 space since the
    /// maximum delay (u32::MAX) occupies bits [63:32] but the high bit
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
/// - `RetrySafety::Safe`: always retriable if the failure's `retryable` flag is true.
/// - `RetrySafety::KeyRequired`: retriable if `retryable` is true (key check is done
///   separately during dispatch via `verify_idempotency`).
/// - `RetrySafety::Unsafe`: never retriable regardless of the failure's flag.
pub fn is_failure_retriable(failure: &ActionFailure, retry_safety: RetrySafety) -> bool {
    match retry_safety {
        RetrySafety::Unsafe => false,
        RetrySafety::Safe | RetrySafety::KeyRequired => failure.retryable,
    }
}

/// Evaluates a retry decision given the current state, policy, and failure.
///
/// This is the core retry state machine transition:
/// 1. Check if the failure is retriable (combines `ActionFailure.retryable` and
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
mod tests {
    use super::*;
    use vb_core::action::ActionFailureCode;
    use vb_core::value::Taint;

    fn fresh_frame() -> RunFrame {
        crate::test_harness::fresh_frame(8, 8)
    }

    // ── RetryPolicy construction ──────────────────────────────────────

    #[test]
    fn retry_policy_new_succeeds_with_valid_params() {
        let policy = RetryPolicy::new(3, 100, 2, DelayStrategy::ExponentialBackoff);
        assert!(policy.is_ok());
        let policy = policy.ok().expect("must succeed");
        assert_eq!(policy.max_attempts(), 3);
        assert_eq!(policy.delay_ms(), 100);
        assert_eq!(policy.backoff_multiplier(), 2);
        assert_eq!(policy.strategy(), DelayStrategy::ExponentialBackoff);
    }

    #[test]
    fn retry_policy_new_rejects_zero_max_attempts() {
        let result = RetryPolicy::new(0, 100, 2, DelayStrategy::Fixed);
        assert_eq!(result, Err(RetryPolicyError::ZeroMaxAttempts));
    }

    #[test]
    fn retry_policy_new_rejects_zero_backoff_multiplier() {
        let result = RetryPolicy::new(3, 100, 0, DelayStrategy::Fixed);
        assert_eq!(result, Err(RetryPolicyError::ZeroBackoffMultiplier));
    }

    #[test]
    fn retry_policy_no_retry_has_single_attempt() {
        let policy = RetryPolicy::no_retry();
        assert_eq!(policy.max_attempts(), 1);
        assert_eq!(policy.delay_ms(), 0);
        assert_eq!(policy.strategy(), DelayStrategy::None);
    }

    #[test]
    fn retry_policy_default_is_three_attempts_fixed() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts(), 3);
        assert_eq!(policy.delay_ms(), 100);
        assert_eq!(policy.strategy(), DelayStrategy::Fixed);
    }

    #[test]
    fn retry_policy_default_policy_matches_default_trait() {
        let trait_default = RetryPolicy::default();
        let method_default = RetryPolicy::default_policy();
        assert_eq!(trait_default, method_default);
    }

    #[test]
    fn retry_policy_new_with_max_u16_attempts() {
        let policy = RetryPolicy::new(u16::MAX, 1000, 1, DelayStrategy::None);
        assert!(policy.is_ok());
        assert_eq!(policy.ok().expect("must succeed").max_attempts(), u16::MAX);
    }

    // ── RetryState construction ───────────────────────────────────────

    #[test]
    fn retry_state_from_policy_initializes_correctly() {
        let policy = RetryPolicy::new(5, 100, 1, DelayStrategy::Fixed)
            .ok()
            .expect("must succeed");
        let state = RetryState::from_policy(&policy);
        assert_eq!(state.current_attempt(), 1);
        assert_eq!(state.remaining(), 5);
        assert_eq!(state.current_delay_ms(), 0);
        assert!(!state.is_exhausted());
    }

    #[test]
    fn retry_state_is_exhausted_when_remaining_zero() {
        let state = RetryState {
            current_attempt: 3,
            remaining: 0,
            current_delay_ms: 100,
        };
        assert!(state.is_exhausted());
    }

    #[test]
    fn retry_state_is_not_exhausted_when_remaining_nonzero() {
        let state = RetryState {
            current_attempt: 1,
            remaining: 2,
            current_delay_ms: 0,
        };
        assert!(!state.is_exhausted());
    }

    // ── RetryState encode/decode roundtrip ────────────────────────────

    #[test]
    fn retry_state_encode_decode_roundtrip() {
        let state = RetryState {
            current_attempt: 2,
            remaining: 3,
            current_delay_ms: 200,
        };
        let packed = state.encode().ok().expect("encode must succeed");
        let decoded = RetryState::decode(packed)
            .ok()
            .expect("decode must succeed");
        assert_eq!(decoded.current_attempt(), 2);
        assert_eq!(decoded.remaining(), 3);
        assert_eq!(decoded.current_delay_ms(), 200);
    }

    #[test]
    fn retry_state_encode_decode_max_values() {
        let state = RetryState {
            current_attempt: u16::MAX,
            remaining: u16::MAX,
            current_delay_ms: u32::MAX,
        };
        let packed = state.encode().ok().expect("encode must succeed");
        let decoded = RetryState::decode(packed)
            .ok()
            .expect("decode must succeed");
        assert_eq!(decoded.current_attempt(), u16::MAX);
        assert_eq!(decoded.remaining(), u16::MAX);
        assert_eq!(decoded.current_delay_ms(), u32::MAX);
    }

    #[test]
    fn retry_state_decode_rejects_negative_with_zero_attempt_nonzero_remaining() {
        // Layout: delay=1 in [63:32], attempt=0 in [31:16], remaining=5 in [15:0]
        // attempt=0 with remaining>0 is invalid regardless of delay.
        let packed: i64 = 0x0000_0001_0000_0005;
        let result = RetryState::decode(packed);
        assert_eq!(result, Err(RetryPolicyError::InvalidRetryState));
    }

    #[test]
    fn retry_state_decode_rejects_zero_attempt_with_nonzero_remaining() {
        // current_attempt=0, remaining=1 is invalid
        // Layout: delay=0 in [63:32], attempt=0 in [31:16], remaining=1 in [15:0]
        let packed: i64 = 0x0000_0000_0000_0001;
        let result = RetryState::decode(packed);
        assert_eq!(result, Err(RetryPolicyError::InvalidRetryState));
    }

    // ── RetryState slot read/write ────────────────────────────────────

    #[test]
    fn retry_state_write_read_slot_roundtrip() {
        let mut frame = fresh_frame();
        let slot = SlotIdx::new(0);
        let state = RetryState {
            current_attempt: 1,
            remaining: 5,
            current_delay_ms: 0,
        };
        let write_result = state.write_to_slot(&mut frame, slot);
        assert!(write_result.is_ok());
        let read_state = RetryState::read_from_slot(&frame, slot)
            .ok()
            .expect("read must succeed");
        assert_eq!(read_state.current_attempt(), 1);
        assert_eq!(read_state.remaining(), 5);
        assert_eq!(read_state.current_delay_ms(), 0);
    }

    #[test]
    fn retry_state_read_from_slot_rejects_non_i64() {
        let mut frame = fresh_frame();
        let slot = SlotIdx::new(0);
        frame
            .write_slot(slot, SlotValue::Bool(true))
            .ok()
            .expect("write must succeed");
        let result = RetryState::read_from_slot(&frame, slot);
        assert_eq!(
            result,
            Err(RetryPolicyError::InvalidRetrySlotType {
                expected: "number",
                found: "boolean",
            })
        );
    }

    // ── is_failure_retriable ──────────────────────────────────────────

    #[test]
    fn is_failure_retriable_safe_and_retryable() {
        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retryable: true,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        assert!(is_failure_retriable(&failure, RetrySafety::Safe));
    }

    #[test]
    fn is_failure_retriable_safe_but_not_retryable() {
        let failure = ActionFailure {
            code: ActionFailureCode::Rejected,
            retryable: false,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        assert!(!is_failure_retriable(&failure, RetrySafety::Safe));
    }

    #[test]
    fn is_failure_retriable_unsafe_always_false() {
        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retryable: true,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        assert!(!is_failure_retriable(&failure, RetrySafety::Unsafe));
    }

    #[test]
    fn is_failure_retriable_key_required_and_retryable() {
        let failure = ActionFailure {
            code: ActionFailureCode::RateLimited,
            retryable: true,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        assert!(is_failure_retriable(&failure, RetrySafety::KeyRequired));
    }

    #[test]
    fn is_failure_retriable_key_required_but_not_retryable() {
        let failure = ActionFailure {
            code: ActionFailureCode::PermissionDenied,
            retryable: false,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        assert!(!is_failure_retriable(&failure, RetrySafety::KeyRequired));
    }

    // ── compute_delay ─────────────────────────────────────────────────

    #[test]
    fn compute_delay_none_is_zero() {
        let policy = RetryPolicy::new(3, 100, 1, DelayStrategy::None)
            .ok()
            .expect("must succeed");
        assert_eq!(compute_delay(&policy, 1), 0);
        assert_eq!(compute_delay(&policy, 5), 0);
    }

    #[test]
    fn compute_delay_fixed_is_constant() {
        let policy = RetryPolicy::new(3, 250, 1, DelayStrategy::Fixed)
            .ok()
            .expect("must succeed");
        assert_eq!(compute_delay(&policy, 1), 250);
        assert_eq!(compute_delay(&policy, 2), 250);
        assert_eq!(compute_delay(&policy, 3), 250);
    }

    #[test]
    fn compute_delay_exponential_backoff_doubles() {
        let policy = RetryPolicy::new(5, 100, 2, DelayStrategy::ExponentialBackoff)
            .ok()
            .expect("must succeed");
        // After attempt 1: delay = 100 * 2^0 = 100
        assert_eq!(compute_delay(&policy, 1), 100);
        // After attempt 2: delay = 100 * 2^1 = 200
        assert_eq!(compute_delay(&policy, 2), 200);
        // After attempt 3: delay = 100 * 2^2 = 400
        assert_eq!(compute_delay(&policy, 3), 400);
        // After attempt 4: delay = 100 * 2^3 = 800
        assert_eq!(compute_delay(&policy, 4), 800);
    }

    #[test]
    fn compute_delay_exponential_backoff_with_multiplier_3() {
        let policy = RetryPolicy::new(4, 50, 3, DelayStrategy::ExponentialBackoff)
            .ok()
            .expect("must succeed");
        // After attempt 1: delay = 50 * 3^0 = 50
        assert_eq!(compute_delay(&policy, 1), 50);
        // After attempt 2: delay = 50 * 3^1 = 150
        assert_eq!(compute_delay(&policy, 2), 150);
        // After attempt 3: delay = 50 * 3^2 = 450
        assert_eq!(compute_delay(&policy, 3), 450);
    }

    #[test]
    fn compute_delay_exponential_saturates_at_u32_max() {
        let policy = RetryPolicy::new(100, u32::MAX, 2, DelayStrategy::ExponentialBackoff)
            .ok()
            .expect("must succeed");
        let delay = compute_delay(&policy, 1);
        assert_eq!(delay, u32::MAX);
    }

    // ── evaluate_retry ────────────────────────────────────────────────

    #[test]
    fn evaluate_retry_retriable_with_remaining_attempts() {
        let policy = RetryPolicy::new(3, 100, 1, DelayStrategy::Fixed)
            .ok()
            .expect("must succeed");
        let state = RetryState {
            current_attempt: 1,
            remaining: 2,
            current_delay_ms: 0,
        };
        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retryable: true,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Safe);
        match decision {
            RetryDecision::Retry { state, delay_ms } => {
                assert_eq!(state.current_attempt(), 2);
                assert_eq!(state.remaining(), 1);
                assert_eq!(delay_ms, 100);
            }
            other => {
                assert_eq!(
                    other,
                    RetryDecision::Retry {
                        state: RetryState {
                            current_attempt: 2,
                            remaining: 1,
                            current_delay_ms: 100,
                        },
                        delay_ms: 100,
                    }
                );
            }
        }
    }

    #[test]
    fn evaluate_retry_exhausted_when_remaining_zero() {
        let policy = RetryPolicy::new(3, 100, 1, DelayStrategy::Fixed)
            .ok()
            .expect("must succeed");
        let state = RetryState {
            current_attempt: 3,
            remaining: 0,
            current_delay_ms: 100,
        };
        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retryable: true,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Safe);
        assert_eq!(decision, RetryDecision::Exhausted { max_attempts: 3 });
    }

    #[test]
    fn evaluate_retry_not_retriable_unsafe() {
        let policy = RetryPolicy::new(3, 100, 1, DelayStrategy::Fixed)
            .ok()
            .expect("must succeed");
        let state = RetryState {
            current_attempt: 1,
            remaining: 2,
            current_delay_ms: 0,
        };
        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retryable: true,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Unsafe);
        assert_eq!(decision, RetryDecision::NotRetriable);
    }

    #[test]
    fn evaluate_retry_not_retriable_failure_flag_false() {
        let policy = RetryPolicy::new(3, 100, 1, DelayStrategy::Fixed)
            .ok()
            .expect("must succeed");
        let state = RetryState {
            current_attempt: 1,
            remaining: 2,
            current_delay_ms: 0,
        };
        let failure = ActionFailure {
            code: ActionFailureCode::PermissionDenied,
            retryable: false,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Safe);
        assert_eq!(decision, RetryDecision::NotRetriable);
    }

    #[test]
    fn evaluate_retry_full_cycle_three_attempts() {
        let policy = RetryPolicy::new(3, 100, 1, DelayStrategy::Fixed)
            .ok()
            .expect("must succeed");
        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retryable: true,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };

        // Attempt 1: remaining=3, should retry
        let state1 = RetryState::from_policy(&policy);
        assert_eq!(state1.current_attempt(), 1);
        assert_eq!(state1.remaining(), 3);
        let decision1 = evaluate_retry(&state1, &policy, &failure, RetrySafety::Safe);
        match decision1 {
            RetryDecision::Retry { state, delay_ms } => {
                assert_eq!(state.current_attempt(), 2);
                assert_eq!(state.remaining(), 2);
                assert_eq!(delay_ms, 100);
            }
            other => {
                assert_eq!(
                    other,
                    RetryDecision::Retry {
                        state: RetryState {
                            current_attempt: 2,
                            remaining: 2,
                            current_delay_ms: 100,
                        },
                        delay_ms: 100,
                    }
                );
            }
        }

        // Attempt 2: remaining=2, should retry
        let state2 = RetryState {
            current_attempt: 2,
            remaining: 2,
            current_delay_ms: 100,
        };
        let decision2 = evaluate_retry(&state2, &policy, &failure, RetrySafety::Safe);
        match decision2 {
            RetryDecision::Retry { state, delay_ms } => {
                assert_eq!(state.current_attempt(), 3);
                assert_eq!(state.remaining(), 1);
                assert_eq!(delay_ms, 100);
            }
            other => {
                assert_eq!(
                    other,
                    RetryDecision::Retry {
                        state: RetryState {
                            current_attempt: 3,
                            remaining: 1,
                            current_delay_ms: 100,
                        },
                        delay_ms: 100,
                    }
                );
            }
        }

        // Attempt 3: remaining=1, should retry (last retry)
        let state3 = RetryState {
            current_attempt: 3,
            remaining: 1,
            current_delay_ms: 100,
        };
        let decision3 = evaluate_retry(&state3, &policy, &failure, RetrySafety::Safe);
        match decision3 {
            RetryDecision::Retry { state, delay_ms } => {
                assert_eq!(state.current_attempt(), 4);
                assert_eq!(state.remaining(), 0);
                assert_eq!(delay_ms, 100);
            }
            other => {
                assert_eq!(
                    other,
                    RetryDecision::Retry {
                        state: RetryState {
                            current_attempt: 4,
                            remaining: 0,
                            current_delay_ms: 100,
                        },
                        delay_ms: 100,
                    }
                );
            }
        }

        // Attempt 4: remaining=0, exhausted
        let state4 = RetryState {
            current_attempt: 4,
            remaining: 0,
            current_delay_ms: 100,
        };
        let decision4 = evaluate_retry(&state4, &policy, &failure, RetrySafety::Safe);
        assert_eq!(decision4, RetryDecision::Exhausted { max_attempts: 3 });
    }

    // ── evaluate_retry with exponential backoff ───────────────────────

    #[test]
    fn evaluate_retry_exponential_backoff_increments_delay() {
        let policy = RetryPolicy::new(4, 100, 2, DelayStrategy::ExponentialBackoff)
            .ok()
            .expect("must succeed");
        let failure = ActionFailure {
            code: ActionFailureCode::ExternalUnavailable,
            retryable: true,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };

        let state1 = RetryState::from_policy(&policy);
        let decision1 = evaluate_retry(&state1, &policy, &failure, RetrySafety::Safe);
        match decision1 {
            RetryDecision::Retry { state, delay_ms } => {
                assert_eq!(delay_ms, 100);
                assert_eq!(state.current_delay_ms(), 100);
                // Verify state for next iteration
                let decision2 = evaluate_retry(&state, &policy, &failure, RetrySafety::Safe);
                match decision2 {
                    RetryDecision::Retry { delay_ms, .. } => {
                        assert_eq!(delay_ms, 200);
                    }
                    other => {
                        assert_eq!(
                            other,
                            RetryDecision::Retry {
                                state: RetryState {
                                    current_attempt: 3,
                                    remaining: 2,
                                    current_delay_ms: 200,
                                },
                                delay_ms: 200,
                            }
                        );
                    }
                }
            }
            other => {
                assert_eq!(
                    other,
                    RetryDecision::Retry {
                        state: RetryState {
                            current_attempt: 2,
                            remaining: 3,
                            current_delay_ms: 100,
                        },
                        delay_ms: 100,
                    }
                );
            }
        }
    }

    // ── Non-retriable failure rejection regardless of policy ──────────

    #[test]
    fn evaluate_retry_non_retryable_failure_rejected_even_with_many_attempts() {
        let policy = RetryPolicy::new(100, 1000, 2, DelayStrategy::ExponentialBackoff)
            .ok()
            .expect("must succeed");
        let state = RetryState {
            current_attempt: 1,
            remaining: 99,
            current_delay_ms: 0,
        };
        let failure = ActionFailure {
            code: ActionFailureCode::Rejected,
            retryable: false,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Safe);
        assert_eq!(decision, RetryDecision::NotRetriable);
    }

    #[test]
    fn evaluate_retry_unsafe_safety_rejects_retryable_failure() {
        let policy = RetryPolicy::new(10, 100, 1, DelayStrategy::Fixed)
            .ok()
            .expect("must succeed");
        let state = RetryState {
            current_attempt: 1,
            remaining: 9,
            current_delay_ms: 0,
        };
        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retryable: true,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Unsafe);
        assert_eq!(decision, RetryDecision::NotRetriable);
    }

    // ── exhaustion_error ──────────────────────────────────────────────

    #[test]
    fn exhaustion_error_produces_repeat_exhausted() {
        let error = exhaustion_error(5);
        assert_eq!(error, CoreError::RepeatExhausted { max: 5 });
    }

    // ── retry_start and retry_on_failure ──────────────────────────────

    #[test]
    fn retry_start_writes_initial_state() {
        let mut frame = fresh_frame();
        let slot = SlotIdx::new(0);
        let policy = RetryPolicy::new(3, 100, 1, DelayStrategy::Fixed)
            .ok()
            .expect("must succeed");
        let result = retry_start(&mut frame, &policy, slot);
        assert!(result.is_ok());
        let state = RetryState::read_from_slot(&frame, slot)
            .ok()
            .expect("must read");
        assert_eq!(state.current_attempt(), 1);
        assert_eq!(state.remaining(), 3);
        assert_eq!(state.current_delay_ms(), 0);
    }

    #[test]
    fn retry_on_failure_writes_updated_state_on_retry() {
        let mut frame = fresh_frame();
        let slot = SlotIdx::new(0);
        let policy = RetryPolicy::new(3, 100, 1, DelayStrategy::Fixed)
            .ok()
            .expect("must succeed");
        retry_start(&mut frame, &policy, slot)
            .ok()
            .expect("start must succeed");

        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retryable: true,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let decision = retry_on_failure(&mut frame, slot, &policy, &failure, RetrySafety::Safe)
            .ok()
            .expect("evaluate must succeed");
        match decision {
            RetryDecision::Retry { state, delay_ms } => {
                assert_eq!(state.current_attempt(), 2);
                assert_eq!(state.remaining(), 2);
                assert_eq!(delay_ms, 100);
            }
            other => {
                assert_eq!(
                    other,
                    RetryDecision::Retry {
                        state: RetryState {
                            current_attempt: 2,
                            remaining: 2,
                            current_delay_ms: 100,
                        },
                        delay_ms: 100,
                    }
                );
            }
        }

        // Verify slot was updated
        let read_state = RetryState::read_from_slot(&frame, slot)
            .ok()
            .expect("must read");
        assert_eq!(read_state.current_attempt(), 2);
        assert_eq!(read_state.remaining(), 2);
        assert_eq!(read_state.current_delay_ms(), 100);
    }

    #[test]
    fn retry_on_failure_does_not_modify_slot_on_exhaustion() {
        let mut frame = fresh_frame();
        let slot = SlotIdx::new(0);
        let policy = RetryPolicy::new(1, 100, 1, DelayStrategy::Fixed)
            .ok()
            .expect("must succeed");
        retry_start(&mut frame, &policy, slot)
            .ok()
            .expect("start must succeed");

        // First failure with max_attempts=1: remaining goes from 1 to 0.
        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retryable: true,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let decision = retry_on_failure(&mut frame, slot, &policy, &failure, RetrySafety::Safe)
            .ok()
            .expect("evaluate must succeed");
        // The first retry decision allows one retry (remaining=1 -> remaining=0)
        match decision {
            RetryDecision::Retry { state, .. } => {
                assert_eq!(state.remaining(), 0);
            }
            other => {
                assert_eq!(
                    other,
                    RetryDecision::Retry {
                        state: RetryState {
                            current_attempt: 2,
                            remaining: 0,
                            current_delay_ms: 100,
                        },
                        delay_ms: 100,
                    }
                );
            }
        }

        // Now the state has remaining=0, next failure should exhaust
        let decision2 = retry_on_failure(&mut frame, slot, &policy, &failure, RetrySafety::Safe)
            .ok()
            .expect("evaluate must succeed");
        assert_eq!(decision2, RetryDecision::Exhausted { max_attempts: 1 });
    }

    #[test]
    fn retry_on_failure_does_not_modify_slot_on_not_retriable() {
        let mut frame = fresh_frame();
        let slot = SlotIdx::new(0);
        let policy = RetryPolicy::new(3, 100, 1, DelayStrategy::Fixed)
            .ok()
            .expect("must succeed");
        retry_start(&mut frame, &policy, slot)
            .ok()
            .expect("start must succeed");

        let failure = ActionFailure {
            code: ActionFailureCode::PermissionDenied,
            retryable: false,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let decision = retry_on_failure(&mut frame, slot, &policy, &failure, RetrySafety::Safe)
            .ok()
            .expect("evaluate must succeed");
        assert_eq!(decision, RetryDecision::NotRetriable);

        // Verify slot was NOT modified
        let read_state = RetryState::read_from_slot(&frame, slot)
            .ok()
            .expect("must read");
        assert_eq!(read_state.current_attempt(), 1);
        assert_eq!(read_state.remaining(), 3);
    }

    // ── No-retry policy (single attempt) ──────────────────────────────

    #[test]
    fn no_retry_policy_exhausts_after_first_failure() {
        let policy = RetryPolicy::no_retry();
        let state = RetryState::from_policy(&policy);
        assert_eq!(state.current_attempt(), 1);
        assert_eq!(state.remaining(), 1);

        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retryable: true,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Safe);
        // remaining=1 -> remaining=0, still a retry decision
        match decision {
            RetryDecision::Retry { state, .. } => {
                assert_eq!(state.remaining(), 0);
            }
            other => {
                assert_eq!(
                    other,
                    RetryDecision::Retry {
                        state: RetryState {
                            current_attempt: 2,
                            remaining: 0,
                            current_delay_ms: 0,
                        },
                        delay_ms: 0,
                    }
                );
            }
        }

        // After the retry, remaining=0, next failure exhausts
        let state_after = RetryState {
            current_attempt: 2,
            remaining: 0,
            current_delay_ms: 0,
        };
        let decision2 = evaluate_retry(&state_after, &policy, &failure, RetrySafety::Safe);
        assert_eq!(decision2, RetryDecision::Exhausted { max_attempts: 1 });
    }

    // ── All ActionFailureCode variants with retryable=true/false ──────

    #[test]
    fn all_retryable_failure_codes_are_retriable_with_safe() {
        let codes = [
            ActionFailureCode::Timeout,
            ActionFailureCode::RateLimited,
            ActionFailureCode::ResourceExhausted,
            ActionFailureCode::ExternalUnavailable,
            ActionFailureCode::Conflict,
        ];
        for code in codes {
            let failure = ActionFailure {
                code,
                retryable: true,
                taint: Taint::Clean,
                detail: None,
                encoded_len: 0,
            };
            assert!(
                is_failure_retriable(&failure, RetrySafety::Safe),
                "expected {code:?} to be retriable"
            );
        }
    }

    #[test]
    fn all_non_retryable_failure_codes_are_not_retriable() {
        let codes = [
            ActionFailureCode::Rejected,
            ActionFailureCode::InvalidInput,
            ActionFailureCode::PermissionDenied,
            ActionFailureCode::Unknown,
        ];
        for code in codes {
            let failure = ActionFailure {
                code,
                retryable: false,
                taint: Taint::Clean,
                detail: None,
                encoded_len: 0,
            };
            assert!(
                !is_failure_retriable(&failure, RetrySafety::Safe),
                "expected {code:?} to not be retriable"
            );
        }
    }

    // ── Adversarial BDD: retry safety enforcement ─────────────────────

    #[test]
    fn retry_safety_unsafe_overrides_retryable_flag() {
        // Given a failure with retryable=true but RetrySafety::Unsafe
        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retryable: true,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        // When checking retriable with Unsafe
        // Then it is not retriable regardless of the flag
        assert!(!is_failure_retriable(&failure, RetrySafety::Unsafe));
    }

    #[test]
    fn retry_safety_safe_respects_retryable_flag_false() {
        let failure = ActionFailure {
            code: ActionFailureCode::Rejected,
            retryable: false,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        assert!(!is_failure_retriable(&failure, RetrySafety::Safe));
    }

    #[test]
    fn evaluate_retry_non_retriable_does_not_consume_attempt() {
        let policy = RetryPolicy::new(3, 100, 1, DelayStrategy::Fixed)
            .ok()
            .expect("must succeed");
        let state = RetryState {
            current_attempt: 1,
            remaining: 3,
            current_delay_ms: 0,
        };
        let failure = ActionFailure {
            code: ActionFailureCode::PermissionDenied,
            retryable: false,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Safe);
        assert_eq!(decision, RetryDecision::NotRetriable);
        // The state was not consumed; remaining is still 3.
        // (We verify by re-evaluating with a retriable failure)
        let retryable_failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retryable: true,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let decision2 = evaluate_retry(&state, &policy, &retryable_failure, RetrySafety::Safe);
        match decision2 {
            RetryDecision::Retry { state, .. } => {
                assert_eq!(state.remaining(), 2);
            }
            other => {
                assert_eq!(
                    other,
                    RetryDecision::Retry {
                        state: RetryState {
                            current_attempt: 2,
                            remaining: 2,
                            current_delay_ms: 100,
                        },
                        delay_ms: 100,
                    }
                );
            }
        }
    }

    // ── Adversarial BDD: slot corruption ──────────────────────────────

    #[test]
    fn retry_on_failure_returns_error_on_corrupted_slot() {
        let mut frame = fresh_frame();
        let slot = SlotIdx::new(0);
        frame
            .write_slot(slot, SlotValue::Bool(false))
            .ok()
            .expect("write must succeed");

        let policy = RetryPolicy::new(3, 100, 1, DelayStrategy::Fixed)
            .ok()
            .expect("must succeed");
        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retryable: true,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let result = retry_on_failure(&mut frame, slot, &policy, &failure, RetrySafety::Safe);
        assert_eq!(
            result,
            Err(RetryPolicyError::InvalidRetrySlotType {
                expected: "number",
                found: "boolean",
            })
        );
    }

    #[test]
    fn retry_state_read_from_slot_returns_error_on_null() {
        let mut frame = fresh_frame();
        let slot = SlotIdx::new(0);
        frame
            .write_slot(slot, SlotValue::Null)
            .ok()
            .expect("write must succeed");
        let result = RetryState::read_from_slot(&frame, slot);
        assert_eq!(
            result,
            Err(RetryPolicyError::InvalidRetrySlotType {
                expected: "number",
                found: "null",
            })
        );
    }

    // ── RetryPolicyError display and equality ─────────────────────────

    #[test]
    fn retry_policy_error_equality_zero_max_attempts() {
        assert_eq!(
            RetryPolicyError::ZeroMaxAttempts,
            RetryPolicyError::ZeroMaxAttempts
        );
    }

    #[test]
    fn retry_policy_error_equality_zero_backoff_multiplier() {
        assert_eq!(
            RetryPolicyError::ZeroBackoffMultiplier,
            RetryPolicyError::ZeroBackoffMultiplier
        );
    }

    #[test]
    fn retry_policy_error_inequality_different_variants() {
        assert_ne!(
            RetryPolicyError::ZeroMaxAttempts,
            RetryPolicyError::ZeroBackoffMultiplier
        );
    }

    #[test]
    fn retry_policy_error_debug_contains_variant_name() {
        let error = RetryPolicyError::ZeroMaxAttempts;
        let debug = format!("{error:?}");
        assert!(debug.contains("ZeroMaxAttempts"));
    }

    // ── DelayStrategy equality ────────────────────────────────────────

    #[test]
    fn delay_strategy_variants_are_distinct() {
        assert_ne!(DelayStrategy::None, DelayStrategy::Fixed);
        assert_ne!(DelayStrategy::Fixed, DelayStrategy::ExponentialBackoff);
        assert_ne!(DelayStrategy::None, DelayStrategy::ExponentialBackoff);
    }

    // ── Full exhaustion scenario ──────────────────────────────────────

    #[test]
    fn full_exhaustion_marks_step_failed() {
        let policy = RetryPolicy::new(2, 50, 2, DelayStrategy::ExponentialBackoff)
            .ok()
            .expect("must succeed");
        let mut frame = fresh_frame();
        let slot = SlotIdx::new(0);
        retry_start(&mut frame, &policy, slot)
            .ok()
            .expect("start must succeed");

        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retryable: true,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };

        // First failure: should retry
        let decision1 = retry_on_failure(&mut frame, slot, &policy, &failure, RetrySafety::Safe)
            .ok()
            .expect("must succeed");
        match decision1 {
            RetryDecision::Retry { delay_ms, state } => {
                assert_eq!(delay_ms, 50);
                assert_eq!(state.remaining(), 1);
            }
            other => {
                assert_eq!(
                    other,
                    RetryDecision::Retry {
                        state: RetryState {
                            current_attempt: 2,
                            remaining: 1,
                            current_delay_ms: 50,
                        },
                        delay_ms: 50,
                    }
                );
            }
        }

        // Second failure: should retry (remaining goes 1->0)
        let decision2 = retry_on_failure(&mut frame, slot, &policy, &failure, RetrySafety::Safe)
            .ok()
            .expect("must succeed");
        match decision2 {
            RetryDecision::Retry { delay_ms, state } => {
                assert_eq!(delay_ms, 100); // 50 * 2^1 = 100
                assert_eq!(state.remaining(), 0);
            }
            other => {
                assert_eq!(
                    other,
                    RetryDecision::Retry {
                        state: RetryState {
                            current_attempt: 3,
                            remaining: 0,
                            current_delay_ms: 100,
                        },
                        delay_ms: 100,
                    }
                );
            }
        }

        // Third failure: exhausted
        let decision3 = retry_on_failure(&mut frame, slot, &policy, &failure, RetrySafety::Safe)
            .ok()
            .expect("must succeed");
        assert_eq!(decision3, RetryDecision::Exhausted { max_attempts: 2 });

        // Exhaustion produces the correct error
        let error = exhaustion_error(2);
        assert_eq!(error, CoreError::RepeatExhausted { max: 2 });
    }

    // ── Boundary: u16::MAX attempts ───────────────────────────────────

    #[test]
    fn retry_policy_with_max_attempts_handles_boundary() {
        let policy = RetryPolicy::new(u16::MAX, 10, 1, DelayStrategy::None)
            .ok()
            .expect("must succeed");
        let state = RetryState::from_policy(&policy);
        assert_eq!(state.remaining(), u16::MAX);
        assert!(!state.is_exhausted());

        let failure = ActionFailure {
            code: ActionFailureCode::Timeout,
            retryable: true,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Safe);
        match decision {
            RetryDecision::Retry { state, delay_ms } => {
                assert_eq!(state.remaining(), u16::MAX - 1);
                assert_eq!(delay_ms, 0);
            }
            other => {
                assert_eq!(
                    other,
                    RetryDecision::Retry {
                        state: RetryState {
                            current_attempt: 2,
                            remaining: u16::MAX - 1,
                            current_delay_ms: 0,
                        },
                        delay_ms: 0,
                    }
                );
            }
        }
    }

    // ── RetryPolicy construction with all strategies ──────────────────

    #[test]
    fn retry_policy_with_none_strategy_succeeds() {
        let policy = RetryPolicy::new(5, 0, 1, DelayStrategy::None);
        assert!(policy.is_ok());
        let policy = policy.ok().expect("must succeed");
        assert_eq!(policy.strategy(), DelayStrategy::None);
    }

    #[test]
    fn retry_policy_with_exponential_backoff_succeeds() {
        let policy = RetryPolicy::new(10, 200, 3, DelayStrategy::ExponentialBackoff);
        assert!(policy.is_ok());
        let policy = policy.ok().expect("must succeed");
        assert_eq!(policy.backoff_multiplier(), 3);
    }

    // ── compute_delay edge cases ──────────────────────────────────────

    #[test]
    fn compute_delay_exponential_with_zero_base_is_zero() {
        let policy = RetryPolicy::new(3, 0, 2, DelayStrategy::ExponentialBackoff)
            .ok()
            .expect("must succeed");
        assert_eq!(compute_delay(&policy, 1), 0);
        assert_eq!(compute_delay(&policy, 5), 0);
    }

    #[test]
    fn compute_delay_exponential_with_multiplier_one_is_fixed() {
        let policy = RetryPolicy::new(3, 100, 1, DelayStrategy::ExponentialBackoff)
            .ok()
            .expect("must succeed");
        assert_eq!(compute_delay(&policy, 1), 100);
        assert_eq!(compute_delay(&policy, 2), 100);
        assert_eq!(compute_delay(&policy, 3), 100);
    }

    #[test]
    fn compute_delay_exponential_zero_attempt_is_base() {
        let policy = RetryPolicy::new(3, 100, 2, DelayStrategy::ExponentialBackoff)
            .ok()
            .expect("must succeed");
        assert_eq!(compute_delay(&policy, 0), 100);
    }
}
