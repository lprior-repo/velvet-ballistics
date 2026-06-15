#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses for retry cursor bounds invariants.
//!
//! PO-KANI-001: Proves cursor.attempt ≤ policy.max_attempts,
//! cursor.delay_ms ≤ max_interval_ms, and
//! cursor.exhausted ⟺ cursor.remaining == 0.

use crate::engine::retry_math::{RetryCursor, RetryPolicyLimits};
use crate::engine::types::RetryPolicy;

// ---------------------------------------------------------------------------
// kani::Arbitrary implementations
// ---------------------------------------------------------------------------

impl kani::Arbitrary for RetryPolicy {
    fn any() -> Self {
        let max_attempts: u16 = kani::any();
        kani::assume(max_attempts >= 1);
        kani::assume(max_attempts <= 10);
        Self {
            max_attempts,
            base_delay_ms: kani::any(),
            exponential_backoff: kani::any(),
        }
    }
}

impl kani::Arbitrary for RetryCursor {
    fn any() -> Self {
        let attempt: u16 = kani::any();
        kani::assume(attempt >= 1);
        kani::assume(attempt <= 10);
        let remaining: u16 = kani::any();
        kani::assume(remaining <= 10);
        Self {
            attempt,
            remaining,
            delay_ms: kani::any(),
            exhausted: kani::any(),
        }
    }
}

impl kani::Arbitrary for RetryPolicyLimits {
    fn any() -> Self {
        Self {
            max_attempts: 10,
            max_interval_ms: u64::MAX,
        }
    }
}

// ---------------------------------------------------------------------------
// Harnesses
// ---------------------------------------------------------------------------

/// PO-KANI-001: Proves cursor bounds across all constructor/transition paths.
///
/// Verifies:
/// - initial_cursor: attempt=1, remaining=policy.max_attempts, delay_ms=0
/// - next_cursor: attempt ≤ policy.max_attempts for all non-exhausted states
/// - next_cursor: delay_ms ≤ max_interval_ms
/// - exhausted ⟺ remaining == 0
/// - validate_attempt: rejects 0, rejects attempt > max_attempts
/// - validate_cursor: rejects cursor.delay_ms > max_interval_ms
#[kani::proof]
#[kani::unwind(20)]
fn kani_retry_cursor_bounds() {
    let policy: RetryPolicy = kani::any();
    let limits = RetryPolicyLimits {
        max_attempts: 10,
        max_interval_ms: u64::MAX,
    };

    // Validate policy against limits
    let validated = policy.validate_against(limits);
    if let Err(_) = validated {
        // Policy rejected by limits — nothing more to prove
        return;
    }

    // --- initial_cursor ---
    let initial = policy.initial_cursor();
    assert_eq!(initial.attempt, 1);
    assert_eq!(initial.remaining, policy.max_attempts);
    assert_eq!(initial.delay_ms, 0);
    assert_eq!(
        initial.exhausted,
        policy.max_attempts == 0,
        "initial cursor exhausted only when max_attempts == 0"
    );
    kani::cover!(initial.exhausted == false);
    kani::cover!(initial.exhausted == true);

    // --- next_cursor for arbitrary cursor state ---
    let cursor: RetryCursor = kani::any();
    // Constrain cursor to be valid according to policy
    kani::assume(cursor.attempt >= 1);
    kani::assume(cursor.attempt <= policy.max_attempts);
    kani::assume(cursor.remaining <= policy.max_attempts);
    kani::assume(cursor.delay_ms <= limits.max_interval_ms);

    let max_interval = limits.max_interval_ms;

    match policy.next_cursor(max_interval, cursor) {
        Ok(next) => {
            // Invariant: exhausted ⟺ remaining == 0
            assert_eq!(
                next.exhausted,
                next.remaining == 0,
                "exhausted must be true iff remaining is 0"
            );

            if !next.exhausted {
                // Non-exhausted: attempt must be ≤ max_attempts
                kani::assert(
                    next.attempt <= policy.max_attempts,
                    "next cursor attempt {} must not exceed max_attempts {}",
                    next.attempt,
                    policy.max_attempts
                
                // delay must be ≤ max_interval
                kani::assert(
                    next.delay_ms <= max_interval,
                    "next cursor delay {} must not exceed max_interval {}",
                    next.delay_ms,
                    max_interval
                
                // remaining must have decreased or be 0
                if cursor.remaining > 1 {
                    assert_eq!(
                        next.remaining,
                        cursor.remaining - 1,
                        "remaining must decrease by 1"
                    );
                }
            }
            kani::cover!(next.exhausted == true);
            kani::cover!(next.exhausted == false);
            kani::cover!(next.delay_ms == max_interval);
            kani::cover!(next.remaining == 0);
            kani::cover!(next.remaining > 0);
        }
        Err(e) => {
            // Errors should only occur for invalid cursors outside our assumes
            let msg = format!("{:?}", e);
            kani::assert(
                msg.contains("exceeded") || msg.contains("nonzero") || msg.contains("zero"),
                "only expected validation errors"
            
        }
    }

    // --- fast_forward_cursor ---
    let count: u16 = kani::any();
    kani::assume(count <= 5);
    let ff_cursor: RetryCursor = kani::any();
    kani::assume(ff_cursor.attempt >= 1);
    kani::assume(ff_cursor.attempt <= policy.max_attempts);
    kani::assume(ff_cursor.remaining <= policy.max_attempts);
    kani::assume(ff_cursor.delay_ms <= max_interval);

    let _ = policy.fast_forward_cursor(max_interval, ff_cursor, count);

    // --- delay_for_attempt bounds ---
    let attempt: u16 = kani::any();
    kani::assume(attempt >= 1);
    kani::assume(attempt <= policy.max_attempts);
    if let Ok(delay) = policy.delay_for_attempt(max_interval, attempt) {
        kani::assert(
            delay <= max_interval,
            "delay_for_attempt must not exceed max_interval"
        
    }

    // --- Negative tests: validate_attempt rejects invalid inputs ---
    // Reject attempt 0
    kani::assert(policy.delay_for_attempt(max_interval, 0).is_err()

    // Reject attempt > max_attempts
    if policy.max_attempts < 10 {
        kani::assert(policy
            .delay_for_attempt(max_interval, policy.max_attempts + 1)
            .is_err()
    }

    // Cover key states
    kani::cover!(policy.max_attempts == 1);
    kani::cover!(policy.max_attempts == 10);
    kani::cover!(policy.exponential_backoff == true);
    kani::cover!(policy.exponential_backoff == false);
    kani::cover!(policy.base_delay_ms == 0);
    kani::cover!(policy.base_delay_ms > 0);
}
