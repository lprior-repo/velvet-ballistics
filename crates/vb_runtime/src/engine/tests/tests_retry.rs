// =====================================================================
// Basic retry policy tests
// =====================================================================

#[test]
fn retry_policy_never_has_max_attempts_one() {
    assert_eq!(RetryPolicy::NEVER.max_attempts, 1);
}

#[test]
fn retry_policy_default_has_max_attempts_three() {
    assert_eq!(RetryPolicy::DEFAULT.max_attempts, 3);
}

#[test]
fn retry_policy_never_has_base_delay_zero() {
    assert_eq!(RetryPolicy::NEVER.base_delay_ms, 0);
    assert_eq!(RetryPolicy::NEVER.exponential_backoff, false);
}

#[test]
fn retry_policy_default_has_base_delay_100() {
    assert_eq!(RetryPolicy::DEFAULT.base_delay_ms, 100);
    assert_eq!(RetryPolicy::DEFAULT.exponential_backoff, false);
}

#[test]
fn retry_policy_clone_preserves_values() {
    let policy = RetryPolicy {
        max_attempts: 5,
        base_delay_ms: 200,
        exponential_backoff: true,
    };
    let cloned = policy.clone();
    assert_eq!(cloned.max_attempts, 5);
    assert_eq!(cloned.base_delay_ms, 200);
    assert_eq!(cloned.exponential_backoff, true);
}

// =====================================================================
// Retry check tests
// =====================================================================

#[test]
fn retry_check_routes_to_body_when_attempts_remain() {
    let policy = RetryPolicy::DEFAULT;
    let target = execute_retry_check(1, policy, StepIdx::new(5), StepIdx::new(10));
    assert_eq!(target, StepIdx::new(5));
}

#[test]
fn retry_check_routes_to_exhausted_when_attempts_spent() {
    let policy = RetryPolicy {
        max_attempts: 2,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    let target = execute_retry_check(2, policy, StepIdx::new(5), StepIdx::new(10));
    assert_eq!(target, StepIdx::new(10));
}

#[test]
fn never_retry_always_exhausts_after_one() {
    let policy = RetryPolicy::NEVER;
    let target = execute_retry_check(1, policy, StepIdx::new(3), StepIdx::new(7));
    assert_eq!(target, StepIdx::new(7));
}

#[test]
fn retry_check_routes_to_body_below_max() {
    let policy = RetryPolicy {
        max_attempts: 3,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    let target = execute_retry_check(0, policy, StepIdx::new(5), StepIdx::new(10));
    assert_eq!(target, StepIdx::new(5));
}

#[test]
fn retry_check_routes_to_body_at_max_minus_one() {
    let policy = RetryPolicy {
        max_attempts: 3,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    let target = execute_retry_check(2, policy, StepIdx::new(5), StepIdx::new(10));
    assert_eq!(target, StepIdx::new(5));
}

#[test]
fn retry_check_returns_done_when_attempts_equal_max() {
    let policy = RetryPolicy::DEFAULT;
    let target = execute_retry_check(3, policy, StepIdx::new(1), StepIdx::new(10));
    assert_eq!(target, StepIdx::new(10));
}

// =====================================================================
// Error handler tests
// =====================================================================

#[test]
fn error_handler_routes_to_handler_on_retryable_failure() {
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let target = execute_error_handler(&failure, StepIdx::new(5), StepIdx::new(3));
    assert_eq!(target, StepIdx::new(5));
}

#[test]
fn error_handler_routes_to_body_on_non_retryable_unknown() {
    let failure = ActionFailure {
        code: ActionFailureCode::Unknown,
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let target = execute_error_handler(&failure, StepIdx::new(5), StepIdx::new(3));
    assert_eq!(target, StepIdx::new(3));
}

#[test]
fn error_handler_routes_to_body_when_failure_is_unknown_and_non_retryable() {
    let failure = ActionFailure {
        code: ActionFailureCode::Unknown,
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let target = execute_error_handler(&failure, StepIdx::new(8), StepIdx::new(3));
    assert_eq!(target, StepIdx::new(3));
}

#[test]
fn error_handler_routes_to_handler_on_unknown_retryable() {
    let failure = ActionFailure {
        code: ActionFailureCode::Unknown,
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let target = execute_error_handler(&failure, StepIdx::new(8), StepIdx::new(3));
    assert_eq!(target, StepIdx::new(8));
}

#[test]
fn error_handler_routes_to_handler_on_non_unknown_non_retryable() {
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let target = execute_error_handler(&failure, StepIdx::new(8), StepIdx::new(3));
    assert_eq!(target, StepIdx::new(8));
}

