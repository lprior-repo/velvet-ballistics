#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_runtime::engine::{RetryCursor, RetryPolicy, RetryPolicyLimits, RetryPolicyMathError};

const PROOF_MAX_INTERVAL_MS: u64 = u64::MAX;

fn policy_strategy() -> impl Strategy<Value = (RetryPolicy, u64)> {
    (
        1u16..=u16::MAX,
        prop_oneof![
            Just(0u64),
            Just(1),
            Just(99),
            Just(100),
            Just(u64::MAX / 2),
            any::<u64>()
        ],
        prop_oneof![Just(0u64), Just(1), Just(100), Just(u64::MAX), any::<u64>()],
        any::<bool>(),
    )
        .prop_map(|(max_attempts, base, max_interval, exponential)| {
            (
                RetryPolicy {
                    max_attempts,
                    base_delay_ms: base.min(max_interval),
                    exponential_backoff: exponential,
                },
                max_interval,
            )
        })
}

proptest! {
    #[test]
    fn invalid_raw_policy_is_rejected(raw in any::<u16>(), resource in prop::option::of(1u16..=u16::MAX)) {
        let limit = resource.map_or(u16::MAX, |value| value);
        let policy = RetryPolicy {
            max_attempts: raw,
            base_delay_ms: 0,
            exponential_backoff: false,
        };
        let result = policy.validate_against(RetryPolicyLimits { max_attempts: limit, max_interval_ms: 0 });
        if raw == 0 || resource.is_some_and(|limit| raw > limit) {
            prop_assert!(matches!(
                result,
                Err(RetryPolicyMathError::ZeroMaxAttempts | RetryPolicyMathError::MaxAttemptsExceeded)
            ));
        } else {
            prop_assert_eq!(result, Ok(policy));
            if let Some(limit) = resource {
                prop_assert!(raw <= limit);
            }
        }
    }

    #[test]
    fn attempts_are_one_based(attempt in any::<u16>(), max_attempts in 1u16..=u16::MAX) {
        let policy = RetryPolicy { max_attempts, base_delay_ms: 0, exponential_backoff: false };
        let result = policy.delay_for_attempt(PROOF_MAX_INTERVAL_MS, attempt);
        if attempt == 0 || attempt > max_attempts {
            prop_assert!(matches!(
                result,
                Err(RetryPolicyMathError::ZeroAttempt | RetryPolicyMathError::AttemptExceeded)
            ));
        } else {
            prop_assert_eq!(result, Ok(0));
        }
    }

    #[test]
    fn delay_is_bounded_and_deterministic((policy, max_interval_ms) in policy_strategy(), attempt in 1u16..=u16::MAX) {
        let left = policy.delay_for_attempt(max_interval_ms, attempt);
        let right = policy.delay_for_attempt(max_interval_ms, attempt);
        if attempt <= policy.max_attempts {
            prop_assert_eq!(left, right);
            prop_assert!(left.is_ok());
            if let Ok(delay) = left {
                prop_assert!(delay <= max_interval_ms);
            }
        } else {
            prop_assert_eq!(left, Err(RetryPolicyMathError::AttemptExceeded));
        }
    }

    #[test]
    fn fast_forward_matches_repeated_next((policy, max_interval_ms) in policy_strategy(), count in 0u16..=512) {
        let start = policy.initial_cursor();
        let fast = policy.fast_forward_cursor(max_interval_ms, start, count);
        let repeated = (0..count).try_fold(start, |cursor, _| {
            if cursor.exhausted {
                Ok(cursor)
            } else {
                policy.next_cursor(max_interval_ms, cursor)
            }
        });
        prop_assert_eq!(fast, repeated);
        prop_assert_eq!(policy.fast_forward_cursor(max_interval_ms, start, 0), Ok(start));
        if let Ok(cursor) = fast {
            prop_assert!(cursor.attempt <= policy.max_attempts);
        }
    }
}

#[test]
fn public_constant_semantics_are_preserved_in_model() {
    let never = RetryPolicy::NEVER;
    let default = RetryPolicy::DEFAULT;
    assert_eq!(never.max_attempts, 1);
    assert_eq!(never.base_delay_ms, 0);
    assert!(!never.exponential_backoff);
    assert_eq!(default.max_attempts, 3);
    assert_eq!(default.base_delay_ms, 100);
    assert!(!default.exponential_backoff);
    assert_eq!(default.delay_for_attempt(PROOF_MAX_INTERVAL_MS, 1), Ok(100));
}

#[test]
fn exhausted_cursor_is_terminal_in_production_api() {
    let terminal = RetryCursor {
        attempt: 1,
        remaining: 0,
        delay_ms: 0,
        exhausted: true,
    };
    assert_eq!(
        RetryPolicy::NEVER.next_cursor(PROOF_MAX_INTERVAL_MS, terminal),
        Ok(terminal)
    );
}
