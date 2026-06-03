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

pub mod policy;
pub mod state;

pub use policy::{DelayStrategy, RetryPolicy, RetryPolicyError};
pub use state::{RetryDecision, RetryState};

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

    if state.remaining() == 0 {
        return RetryDecision::Exhausted {
            max_attempts: policy.max_attempts(),
        };
    }

    // Decrement remaining and compute next state.
    let new_remaining = state.remaining().saturating_sub(1);
    let new_attempt = state.current_attempt().saturating_add(1);
    let delay_ms = compute_delay(policy, state.current_attempt());

    let new_state = RetryState::new(new_attempt, new_remaining, delay_ms);

    RetryDecision::Retry {
        state: new_state,
        delay_ms,
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
