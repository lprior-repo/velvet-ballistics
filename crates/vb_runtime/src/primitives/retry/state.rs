#![forbid(unsafe_code)]
//! Retry state management and slot persistence.

use vb_core::frame::RunFrame;
use vb_core::ids::SlotIdx;
use vb_core::value::SlotValue;

use super::policy::{RetryPolicy, RetryPolicyError};

/// Current state of the retry state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryState {
    /// Which attempt we are on (1-indexed, starts at 1).
    pub(crate) current_attempt: u16,
    /// How many attempts remain (including the current one).
    pub(crate) remaining: u16,
    /// The current delay in milliseconds to apply before the next attempt.
    pub(crate) current_delay_ms: u32,
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

    /// Creates a new retry state with explicit values.
    #[inline]
    #[must_use]
    pub(crate) fn new(current_attempt: u16, remaining: u16, current_delay_ms: u32) -> Self {
        Self {
            current_attempt,
            remaining,
            current_delay_ms,
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
