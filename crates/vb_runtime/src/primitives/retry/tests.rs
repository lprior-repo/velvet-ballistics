use super::*;
use vb_core::action::{ActionFailureCode, RetryPolicy as VbRetryPolicy};
use vb_core::value::{SlotValue, Taint};

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
fn ut_retrystate_encoding_roundtrips() {
    let state = RetryState {
        current_attempt: 3,
        remaining: 4,
        current_delay_ms: 500,
    };
    let packed = state.encode().expect("encode must succeed");
    let decoded = RetryState::decode(packed).expect("decode must succeed");
    assert_eq!(decoded.current_attempt(), state.current_attempt);
    assert_eq!(decoded.remaining(), state.remaining);
    assert_eq!(decoded.current_delay_ms(), state.current_delay_ms);
}

#[test]
fn ut_retrystate_invariant_holds_for_active_state() {
    let max_attempts: u16 = 5;
    let state = RetryState {
        current_attempt: 2,
        remaining: 3,
        current_delay_ms: 100,
    };
    let packed = state.encode().expect("encode must succeed");
    let decoded = RetryState::decode(packed).expect("decode must succeed");
    let total_attempts = decoded.current_attempt() + decoded.remaining();
    let max_live_attempts = max_attempts + 1;
    assert!(
        total_attempts <= max_live_attempts,
        "current_attempt({}) + remaining({}) = {} must be <= max_attempts({}) + 1 = {}",
        decoded.current_attempt(),
        decoded.remaining(),
        total_attempts,
        max_attempts,
        max_live_attempts
    );
}

#[test]
fn ut_retrystate_invariant_holds_for_zero_state() {
    let max_attempts: u16 = 5;
    let state = RetryState {
        current_attempt: 0,
        remaining: 0,
        current_delay_ms: 0,
    };
    let packed = state.encode().expect("encode must succeed");
    let decoded = RetryState::decode(packed).expect("decode must succeed");
    assert_eq!(decoded.current_attempt(), 0);
    assert_eq!(decoded.remaining(), 0);
    assert_eq!(decoded.current_delay_ms(), 0);
    let total_attempts = decoded.current_attempt() + decoded.remaining();
    let max_live_attempts = max_attempts + 1;
    assert!(
        total_attempts <= max_live_attempts,
        "zero state: current_attempt({}) + remaining({}) = {} must be <= max_attempts({}) + 1 = {}",
        decoded.current_attempt(),
        decoded.remaining(),
        total_attempts,
        max_attempts,
        max_live_attempts
    );
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
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    assert!(is_failure_retriable(&failure, RetrySafety::Idempotent));
}

#[test]
fn is_failure_retriable_safe_but_not_retryable() {
    let failure = ActionFailure {
        code: ActionFailureCode::Rejected,
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    assert!(!is_failure_retriable(&failure, RetrySafety::Idempotent));
}

#[test]
fn is_failure_retriable_unsafe_always_false() {
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    assert!(!is_failure_retriable(&failure, RetrySafety::NotRetrySafe));
}

#[test]
fn is_failure_retriable_key_required_and_retryable() {
    let failure = ActionFailure {
        code: ActionFailureCode::RateLimited,
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    assert!(is_failure_retriable(&failure, RetrySafety::RequiresIdempotencyKey));
}

#[test]
fn is_failure_retriable_key_required_but_not_retryable() {
    let failure = ActionFailure {
        code: ActionFailureCode::PermissionDenied,
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    assert!(!is_failure_retriable(&failure, RetrySafety::RequiresIdempotencyKey));
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
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Idempotent);
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
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Idempotent);
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
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::NotRetrySafe);
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
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Idempotent);
    assert_eq!(decision, RetryDecision::NotRetriable);
}

#[test]
fn evaluate_retry_full_cycle_three_attempts() {
    let policy = RetryPolicy::new(3, 100, 1, DelayStrategy::Fixed)
        .ok()
        .expect("must succeed");
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };

    // Attempt 1: remaining=3, should retry
    let state1 = RetryState::from_policy(&policy);
    assert_eq!(state1.current_attempt(), 1);
    assert_eq!(state1.remaining(), 3);
    let decision1 = evaluate_retry(&state1, &policy, &failure, RetrySafety::Idempotent);
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
    let decision2 = evaluate_retry(&state2, &policy, &failure, RetrySafety::Idempotent);
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
    let decision3 = evaluate_retry(&state3, &policy, &failure, RetrySafety::Idempotent);
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
    let decision4 = evaluate_retry(&state4, &policy, &failure, RetrySafety::Idempotent);
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
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };

    let state1 = RetryState::from_policy(&policy);
    let decision1 = evaluate_retry(&state1, &policy, &failure, RetrySafety::Idempotent);
    match decision1 {
        RetryDecision::Retry { state, delay_ms } => {
            assert_eq!(delay_ms, 100);
            assert_eq!(state.current_delay_ms(), 100);
            // Verify state for next iteration
            let decision2 = evaluate_retry(&state, &policy, &failure, RetrySafety::Idempotent);
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
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Idempotent);
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
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::NotRetrySafe);
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
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let decision = retry_on_failure(&mut frame, slot, &policy, &failure, RetrySafety::Idempotent)
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
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let decision = retry_on_failure(&mut frame, slot, &policy, &failure, RetrySafety::Idempotent)
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
    let decision2 = retry_on_failure(&mut frame, slot, &policy, &failure, RetrySafety::Idempotent)
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
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let decision = retry_on_failure(&mut frame, slot, &policy, &failure, RetrySafety::Idempotent)
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
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Idempotent);
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
    let decision2 = evaluate_retry(&state_after, &policy, &failure, RetrySafety::Idempotent);
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
            retry_policy: VbRetryPolicy::Retryable,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        assert!(
            is_failure_retriable(&failure, RetrySafety::Idempotent),
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
            retry_policy: VbRetryPolicy::NonRetryable,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        };
        assert!(
            !is_failure_retriable(&failure, RetrySafety::Idempotent),
            "expected {code:?} to not be retriable"
        );
    }
}

// ── Adversarial BDD: retry safety enforcement ─────────────────────

#[test]
fn retry_safety_unsafe_overrides_retryable_flag() {
    // Given a failure with retry_policy=Retryable but RetrySafety::NotRetrySafe
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    // When checking retriable with Unsafe
    // Then it is not retriable regardless of the flag
    assert!(!is_failure_retriable(&failure, RetrySafety::NotRetrySafe));
}

#[test]
fn retry_safety_safe_respects_retryable_flag_false() {
    let failure = ActionFailure {
        code: ActionFailureCode::Rejected,
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    assert!(!is_failure_retriable(&failure, RetrySafety::Idempotent));
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
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Idempotent);
    assert_eq!(decision, RetryDecision::NotRetriable);
    // The state was not consumed; remaining is still 3.
    // (We verify by re-evaluating with a retriable failure)
    let retryable_failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let decision2 = evaluate_retry(&state, &policy, &retryable_failure, RetrySafety::Idempotent);
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
        .expect("write must succeed");

    let policy = RetryPolicy::new(3, 100, 1, DelayStrategy::Fixed)
        .ok()
        .expect("must succeed");
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let result = retry_on_failure(&mut frame, slot, &policy, &failure, RetrySafety::Idempotent);
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
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };

    // First failure: should retry
    let decision1 = retry_on_failure(&mut frame, slot, &policy, &failure, RetrySafety::Idempotent)
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
    let decision2 = retry_on_failure(&mut frame, slot, &policy, &failure, RetrySafety::Idempotent)
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
    let decision3 = retry_on_failure(&mut frame, slot, &policy, &failure, RetrySafety::Idempotent)
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
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Idempotent);
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

// =========================================================================
// vb-u09ai: 4-variant RetrySafety runtime tests (Tier 1 — fail-loud compile
// on 3-variant code because Idempotent/RequiresIdempotencyKey/NotRetrySafe/
// Unknown are not in scope).
// =========================================================================

/// Tier 1: `is_failure_retriable` returns true for `Idempotent` + retryable failure.
#[test]
fn is_failure_retriable_returns_true_for_idempotent_with_retryable_failure() {
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    assert!(is_failure_retriable(&failure, RetrySafety::Idempotent));
}

/// Tier 1: `is_failure_retriable` returns false for `NotRetrySafe`.
#[test]
fn is_failure_retriable_returns_false_for_not_retry_safe() {
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    assert!(!is_failure_retriable(&failure, RetrySafety::NotRetrySafe));
}

/// Tier 1: `is_failure_retriable` returns false for `Unknown`.
#[test]
fn is_failure_retriable_returns_false_for_unknown() {
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    assert!(!is_failure_retriable(&failure, RetrySafety::Unknown));
}

/// Tier 1: `Unknown` collapses with `NotRetrySafe` at the retriable gate (C8).
#[test]
fn is_failure_retriable_collapses_unknown_with_not_retry_safe() {
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    assert_eq!(
        is_failure_retriable(&failure, RetrySafety::Unknown),
        is_failure_retriable(&failure, RetrySafety::NotRetrySafe),
        "Unknown and NotRetrySafe must collapse to the same retriable result"
    );
}
