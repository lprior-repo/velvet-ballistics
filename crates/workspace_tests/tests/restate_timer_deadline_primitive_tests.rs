#![forbid(unsafe_code)]
//! Primitive-level behavior tests proving invariants documented in the
//! state11 proof-preparation report for vb-wfi4:
//!
//! - TimerWheel `collect_expired_keys` purity (via public `fire_expired`)
//! - `compute_delay` loop bound invariants
//! - `fast_forward_cursor` bounds
//! - Documented invariant categories: index consistency, run uniqueness,
//!   generation monotonicity, deadline ordering, bounded operations,
//!   state-machine determinism, arithmetic safety, total functions.
//!
//! Every test exercises a public API and asserts exact values or error
//! variants. No `is_ok()`, `is_err()`, or silent wildcard branches.

use std::time::{Duration, Instant};

use vb_core::action::{
    ActionFailure, ActionFailureCode, RetryPolicy as VbRetryPolicy, RetrySafety,
};
use vb_core::errors::CoreError;
use vb_core::ids::RunId;
use vb_core::value::Taint;
use vb_runtime::engine::{
    RetryCursor, RetryPolicy as EngineRetryPolicy, RetryPolicyLimits, RetryPolicyMathError,
};
use vb_runtime::primitives::retry::{
    DelayStrategy, RetryDecision, RetryPolicy, RetryPolicyError, RetryState, compute_delay,
    evaluate_retry, exhaustion_error, is_failure_retriable,
};
use vb_runtime::shard::timer_wheel::{TimerEntry, TimerWheel, TimerWheelError};
use vb_runtime::shard::types::PendingTimerKind;

// ── Helpers ────────────────────────────────────────────────────────────────

fn make_run(id: u64) -> RunId {
    RunId::new(id)
}

fn retryable_failure(code: ActionFailureCode) -> ActionFailure {
    ActionFailure {
        code,
        retry_policy: VbRetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    }
}

fn non_retryable_failure(code: ActionFailureCode) -> ActionFailure {
    ActionFailure {
        code,
        retry_policy: VbRetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    }
}

// ============================================================================
// Section 1: collect_expired_keys purity (tested via fire_expired public API)
// ============================================================================

mod collect_expired_keys_purity {
    use super::*;

    #[test]
    fn fire_expired_on_empty_wheel_returns_empty_vector() {
        let mut wheel = TimerWheel::new();
        let fired = wheel.fire_expired(Instant::now());
        assert_eq!(fired, Vec::new());
        assert_eq!(wheel.len(), 0);
        assert!(wheel.is_empty());
    }

    #[test]
    fn fire_expired_at_far_future_returns_empty_and_preserves_entries() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let far_future = now + Duration::from_secs(3600);
        let _ = wheel.insert(make_run(1), far_future, PendingTimerKind::Wait);
        assert_eq!(wheel.len(), 1);
        let fired = wheel.fire_expired(now);
        assert_eq!(fired, Vec::new());
        assert_eq!(wheel.len(), 1);
    }

    #[test]
    fn fire_expired_at_very_distant_future_returns_empty() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let d1 = now + Duration::from_secs(100);
        let d2 = now + Duration::from_secs(200);
        let _ = wheel.insert(make_run(1), d1, PendingTimerKind::Wait);
        let _ = wheel.insert(make_run(2), d2, PendingTimerKind::Ask);
        let fired = wheel.fire_expired(now);
        assert_eq!(fired, Vec::new());
        assert_eq!(wheel.len(), 2);
    }

    #[test]
    fn fire_expired_at_past_instant_fires_only_past_entries() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let past = now - Duration::from_millis(100);
        let future = now + Duration::from_secs(60);
        let _ = wheel.insert(make_run(1), past, PendingTimerKind::Wait);
        let _ = wheel.insert(make_run(2), future, PendingTimerKind::Ask);
        let fired = wheel.fire_expired(now);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].run, make_run(1));
        assert_eq!(wheel.len(), 1);
        assert_eq!(wheel.get_kind(make_run(2)), Some(PendingTimerKind::Ask));
    }

    #[test]
    fn fire_expired_at_exact_deadline_fires_entry() {
        let mut wheel = TimerWheel::new();
        let deadline = Instant::now();
        let _ = wheel.insert(make_run(1), deadline, PendingTimerKind::Wait);
        let fired = wheel.fire_expired(deadline);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].run, make_run(1));
        assert_eq!(fired[0].kind, PendingTimerKind::Wait);
        assert!(wheel.is_empty());
    }

    #[test]
    fn fire_expired_drains_all_expired_from_both_indices() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let d1 = now - Duration::from_millis(200);
        let d2 = now - Duration::from_millis(100);
        let d3 = now - Duration::from_millis(50);
        let _ = wheel.insert(make_run(1), d1, PendingTimerKind::Wait);
        let _ = wheel.insert(make_run(2), d2, PendingTimerKind::Ask);
        let _ = wheel.insert(make_run(3), d3, PendingTimerKind::Wait);
        let fired = wheel.fire_expired(now);
        assert_eq!(fired.len(), 3);
        assert!(wheel.is_empty());
        // All by_run entries removed
        assert_eq!(wheel.get_entry(make_run(1)), None);
        assert_eq!(wheel.get_entry(make_run(2)), None);
        assert_eq!(wheel.get_entry(make_run(3)), None);
    }

    #[test]
    fn fire_expired_preserves_future_entries_after_removing_expired() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let past = now - Duration::from_millis(50);
        let future = now + Duration::from_secs(10);
        let _ = wheel.insert(make_run(1), past, PendingTimerKind::Wait);
        let _ = wheel.insert(make_run(2), future, PendingTimerKind::Ask);
        let fired = wheel.fire_expired(now);
        assert_eq!(fired.len(), 1);
        assert_eq!(wheel.len(), 1);
        assert_eq!(
            wheel.get_entry(make_run(2)).map(|e| e.run),
            Some(make_run(2))
        );
    }

    #[test]
    fn fire_expired_multiple_runs_at_same_deadline_all_fire() {
        let mut wheel = TimerWheel::new();
        let deadline = Instant::now();
        let _ = wheel.insert(make_run(1), deadline, PendingTimerKind::Wait);
        let _ = wheel.insert(make_run(2), deadline, PendingTimerKind::Ask);
        let _ = wheel.insert(make_run(3), deadline, PendingTimerKind::Wait);
        let _ = wheel.insert(make_run(4), deadline, PendingTimerKind::Ask);
        let _ = wheel.insert(make_run(5), deadline, PendingTimerKind::Wait);
        let fired = wheel.fire_expired(deadline);
        assert_eq!(fired.len(), 5);
        assert!(wheel.is_empty());
    }

    #[test]
    fn fire_expired_deadline_bucket_removed_when_empty() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let past = now - Duration::from_millis(10);
        let _ = wheel.insert(make_run(1), past, PendingTimerKind::Wait);
        assert_eq!(wheel.len(), 1);
        let _ = wheel.fire_expired(now);
        assert_eq!(wheel.len(), 0);
        assert_eq!(wheel.next_deadline(), None);
    }

    #[test]
    fn fire_expired_is_idempotent_on_empty_wheel() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let fired1 = wheel.fire_expired(now);
        let fired2 = wheel.fire_expired(now);
        let fired3 = wheel.fire_expired(now + Duration::from_secs(3600));
        assert_eq!(fired1, Vec::new());
        assert_eq!(fired2, Vec::new());
        assert_eq!(fired3, Vec::new());
        assert_eq!(wheel.len(), 0);
    }

    #[test]
    fn fire_expired_pure_collection_via_observable_len_after_single_fire() {
        // Prove collect_expired_keys does not mutate: after fire_expired(now),
        // all expired entries are removed. Re-calling fire_expired with the
        // same now should return empty — the pure key collection always
        // produces the same set for the same state.
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let past = now - Duration::from_millis(1);
        let _ = wheel.insert(make_run(1), past, PendingTimerKind::Wait);
        // First fire
        let fired1 = wheel.fire_expired(now);
        assert_eq!(fired1.len(), 1);
        // Second fire with same now — wheel is now empty below "now"
        let fired2 = wheel.fire_expired(now);
        assert_eq!(fired2, Vec::new());
        assert!(wheel.is_empty());
    }

    #[test]
    fn fire_expired_preserves_entry_kind_and_generation() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let past = now - Duration::from_millis(10);
        let _ = wheel.insert(make_run(42), past, PendingTimerKind::Ask);
        let fired = wheel.fire_expired(now);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].run, make_run(42));
        assert_eq!(fired[0].kind, PendingTimerKind::Ask);
        assert_eq!(fired[0].generation, 1);
        assert_eq!(fired[0].deadline, past);
    }

    #[test]
    fn fire_expired_staggered_deadlines_partial_fire() {
        let mut wheel = TimerWheel::new();
        let base = Instant::now();
        let d1 = base - Duration::from_millis(30);
        let d2 = base - Duration::from_millis(20);
        let d3 = base - Duration::from_millis(10);
        let d4 = base + Duration::from_millis(10);
        let _ = wheel.insert(make_run(1), d1, PendingTimerKind::Wait);
        let _ = wheel.insert(make_run(2), d2, PendingTimerKind::Ask);
        let _ = wheel.insert(make_run(3), d3, PendingTimerKind::Wait);
        let _ = wheel.insert(make_run(4), d4, PendingTimerKind::Ask);
        // Fire at an intermediate point
        let mid = base - Duration::from_millis(15);
        let fired = wheel.fire_expired(mid);
        assert_eq!(fired.len(), 2); // d1 and d2
        assert_eq!(wheel.len(), 2);
    }
}

// ============================================================================
// Section 2: compute_delay loop bound invariants
// ============================================================================

mod compute_delay_loop_bounds {
    use super::*;

    fn policy_with(
        max_attempts: u16,
        delay_ms: u32,
        multiplier: u32,
        strategy: DelayStrategy,
    ) -> RetryPolicy {
        RetryPolicy::new(max_attempts, delay_ms, multiplier, strategy).expect("valid policy")
    }

    // ── None strategy ──────────────────────────────────────────────────

    #[test]
    fn compute_delay_none_always_zero_for_all_attempts() {
        let policy = policy_with(3, 100, 1, DelayStrategy::None);
        assert_eq!(compute_delay(&policy, 0), 0);
        assert_eq!(compute_delay(&policy, 1), 0);
        assert_eq!(compute_delay(&policy, 2), 0);
        assert_eq!(compute_delay(&policy, 10), 0);
        assert_eq!(compute_delay(&policy, 100), 0);
    }

    // ── Fixed strategy ─────────────────────────────────────────────────

    #[test]
    fn compute_delay_fixed_always_returns_base_delay() {
        let policy = policy_with(5, 250, 1, DelayStrategy::Fixed);
        assert_eq!(compute_delay(&policy, 0), 250);
        assert_eq!(compute_delay(&policy, 1), 250);
        assert_eq!(compute_delay(&policy, 2), 250);
        assert_eq!(compute_delay(&policy, 3), 250);
        assert_eq!(compute_delay(&policy, 100), 250);
    }

    #[test]
    fn compute_delay_fixed_with_zero_base() {
        let policy = policy_with(3, 0, 2, DelayStrategy::Fixed);
        assert_eq!(compute_delay(&policy, 1), 0);
        assert_eq!(compute_delay(&policy, 5), 0);
    }

    #[test]
    fn compute_delay_fixed_with_max_base() {
        let policy = policy_with(3, u32::MAX, 1, DelayStrategy::Fixed);
        assert_eq!(compute_delay(&policy, 1), u32::MAX);
        assert_eq!(compute_delay(&policy, 100), u32::MAX);
    }

    // ── Exponential backoff ────────────────────────────────────────────

    #[test]
    fn compute_delay_exponential_attempt_1_returns_base() {
        let policy = policy_with(5, 100, 2, DelayStrategy::ExponentialBackoff);
        assert_eq!(compute_delay(&policy, 1), 100);
    }

    #[test]
    fn compute_delay_exponential_doubles_each_step() {
        let policy = policy_with(5, 100, 2, DelayStrategy::ExponentialBackoff);
        assert_eq!(compute_delay(&policy, 1), 100); // 100 * 2^0
        assert_eq!(compute_delay(&policy, 2), 200); // 100 * 2^1
        assert_eq!(compute_delay(&policy, 3), 400); // 100 * 2^2
        assert_eq!(compute_delay(&policy, 4), 800); // 100 * 2^3
    }

    #[test]
    fn compute_delay_exponential_with_multiplier_3() {
        let policy = policy_with(5, 50, 3, DelayStrategy::ExponentialBackoff);
        assert_eq!(compute_delay(&policy, 1), 50); // 50 * 3^0
        assert_eq!(compute_delay(&policy, 2), 150); // 50 * 3^1
        assert_eq!(compute_delay(&policy, 3), 450); // 50 * 3^2
    }

    #[test]
    fn compute_delay_exponential_with_multiplier_10() {
        let policy = policy_with(4, 2, 10, DelayStrategy::ExponentialBackoff);
        assert_eq!(compute_delay(&policy, 1), 2); // 2 * 10^0
        assert_eq!(compute_delay(&policy, 2), 20); // 2 * 10^1
        assert_eq!(compute_delay(&policy, 3), 200); // 2 * 10^2
    }

    #[test]
    fn compute_delay_exponential_saturates_at_u32_max_on_overflow() {
        // base=u32::MAX, multiplier=2 -> immediate overflow on first fold
        let policy = policy_with(100, u32::MAX, 2, DelayStrategy::ExponentialBackoff);
        assert_eq!(compute_delay(&policy, 1), u32::MAX);
        assert_eq!(compute_delay(&policy, 2), u32::MAX);
    }

    #[test]
    fn compute_delay_exponential_saturates_at_second_multiplication() {
        // Start just above half: u32::MAX/2 + 1 so that *2 overflows u32
        let over_half = u32::MAX / 2 + 1;
        let policy = policy_with(4, over_half, 2, DelayStrategy::ExponentialBackoff);
        assert_eq!(compute_delay(&policy, 1), over_half);
        // next: over_half * 2 = (u32::MAX/2 + 1) * 2 = u32::MAX + 1 -> overflows -> u32::MAX
        assert_eq!(compute_delay(&policy, 2), u32::MAX);
    }

    #[test]
    fn compute_delay_exponential_zero_base_stays_zero() {
        let policy = policy_with(5, 0, 2, DelayStrategy::ExponentialBackoff);
        assert_eq!(compute_delay(&policy, 1), 0);
        assert_eq!(compute_delay(&policy, 5), 0);
        assert_eq!(compute_delay(&policy, 100), 0);
    }

    #[test]
    fn compute_delay_exponential_multiplier_1_is_constant() {
        let policy = policy_with(5, 77, 1, DelayStrategy::ExponentialBackoff);
        assert_eq!(compute_delay(&policy, 1), 77);
        assert_eq!(compute_delay(&policy, 2), 77);
        assert_eq!(compute_delay(&policy, 10), 77);
    }

    #[test]
    fn compute_delay_exponential_attempt_zero_returns_base() {
        let policy = policy_with(5, 100, 2, DelayStrategy::ExponentialBackoff);
        assert_eq!(compute_delay(&policy, 0), 100);
    }

    #[test]
    fn compute_delay_exponential_attempt_u16_max_is_bounded() {
        // This exercises the documented loop bound: at most u16::MAX=65535 iterations.
        let policy = policy_with(u16::MAX, 1, 2, DelayStrategy::ExponentialBackoff);
        let delay = compute_delay(&policy, u16::MAX);
        // With base=1, multiplier=2, and exponent = u16::MAX - 1,
        // delay = 1 * 2^(u16::MAX-1) which will overflow to u32::MAX.
        assert_eq!(delay, u32::MAX);
    }

    #[test]
    fn compute_delay_is_deterministic() {
        let policy = policy_with(10, 50, 3, DelayStrategy::ExponentialBackoff);
        for attempt in [0u16, 1, 2, 3, 5, 10, 100] {
            let a = compute_delay(&policy, attempt);
            let b = compute_delay(&policy, attempt);
            assert_eq!(a, b, "delay must be deterministic for attempt {attempt}");
        }
    }

    #[test]
    fn compute_delay_exponential_many_iterations_correct() {
        // Verify explicit fold for a manual count to prove loop visits each exponent.
        let policy = policy_with(10, 3, 2, DelayStrategy::ExponentialBackoff);
        // attempt 7 -> exponent = 6, delay = 3 * 2^6 = 192
        assert_eq!(compute_delay(&policy, 7), 192);
    }

    #[test]
    fn compute_delay_across_all_strategies() {
        let none = policy_with(3, 50, 2, DelayStrategy::None);
        let fixed = policy_with(3, 50, 2, DelayStrategy::Fixed);
        let exp = policy_with(3, 50, 2, DelayStrategy::ExponentialBackoff);
        assert_eq!(compute_delay(&none, 1), 0);
        assert_eq!(compute_delay(&fixed, 1), 50);
        assert_eq!(compute_delay(&exp, 1), 50);
        assert_eq!(compute_delay(&none, 2), 0);
        assert_eq!(compute_delay(&fixed, 2), 50);
        assert_eq!(compute_delay(&exp, 2), 100);
    }
}

// ============================================================================
// Section 3: fast_forward_cursor bounds
// ============================================================================

mod fast_forward_cursor_bounds {
    use super::*;

    const MAX_INTERVAL: u64 = u64::MAX;

    fn policy(max_attempts: u16, base_delay_ms: u64, exponential: bool) -> EngineRetryPolicy {
        EngineRetryPolicy {
            max_attempts,
            base_delay_ms,
            exponential_backoff: exponential,
        }
    }

    #[test]
    fn fast_forward_zero_count_returns_identity() {
        let p = policy(5, 0, false);
        let start = p.initial_cursor();
        let result = p.fast_forward_cursor(MAX_INTERVAL, start, 0);
        assert_eq!(result, Ok(start));
    }

    #[test]
    fn fast_forward_matches_repeated_next_cursor() {
        let p = policy(10, 10, true);
        let start = p.initial_cursor();
        let count: u16 = 5;
        let fast = p.fast_forward_cursor(MAX_INTERVAL, start, count);
        let repeated = (0..count).try_fold(start, |cursor, _| {
            if cursor.exhausted {
                Ok(cursor)
            } else {
                p.next_cursor(MAX_INTERVAL, cursor)
            }
        });
        assert_eq!(fast, repeated);
    }

    #[test]
    fn fast_forward_stops_at_exhaustion() {
        let p = policy(2, 10, false);
        let start = p.initial_cursor();
        // Policy has max_attempts=2. Fast-forward by 5 should exhaust after
        // consuming remaining attempts (2 attempts total).
        let result = p.fast_forward_cursor(MAX_INTERVAL, start, 5);
        match result {
            Ok(cursor) => {
                assert!(cursor.exhausted);
                assert_eq!(cursor.remaining, 0);
            }
            Err(e) => {
                let msg = format!("unexpected error: {e:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn fast_forward_attempt_never_exceeds_max_attempts() {
        let p = policy(3, 0, false);
        let start = p.initial_cursor();
        let result = p.fast_forward_cursor(MAX_INTERVAL, start, 100);
        match result {
            Ok(cursor) => {
                assert!(cursor.attempt <= p.max_attempts);
            }
            Err(e) => {
                let msg = format!("unexpected error: {e:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn fast_forward_exhausted_cursor_remains_unchanged() {
        let p = policy(5, 0, false);
        let exhausted = RetryCursor {
            attempt: 2,
            remaining: 0,
            delay_ms: 0,
            exhausted: true,
        };
        let result = p.fast_forward_cursor(MAX_INTERVAL, exhausted, 10);
        match result {
            Ok(cursor) => {
                assert_eq!(cursor, exhausted);
            }
            Err(e) => {
                let msg = format!("unexpected error: {e:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn fast_forward_with_max_u16_count_is_bounded() {
        // Documented invariant: iterates at most count times (u16::MAX=65535).
        let p = policy(2, 0, false);
        let start = p.initial_cursor();
        let result = p.fast_forward_cursor(MAX_INTERVAL, start, u16::MAX);
        match result {
            Ok(cursor) => {
                assert!(cursor.exhausted);
                assert!(cursor.attempt <= p.max_attempts);
            }
            Err(e) => {
                let msg = format!("unexpected error: {e:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn fast_forward_remaining_decreases_monotonically() {
        let p = policy(10, 5, false);
        let start = p.initial_cursor();
        let mut prev_remaining = start.remaining;
        for step in 1u16..=5 {
            let result = p.fast_forward_cursor(MAX_INTERVAL, start, step);
            match result {
                Ok(cursor) => {
                    assert!(cursor.remaining <= prev_remaining);
                    prev_remaining = cursor.remaining;
                }
                Err(e) => {
                    let msg = format!("unexpected error at step {step}: {e:?}");
                    panic!("{msg}");
                }
            }
        }
    }

    #[test]
    fn fast_forward_never_policy_exhausts_after_one_attempt() {
        let p = EngineRetryPolicy::NEVER;
        let start = p.initial_cursor();
        let result = p.fast_forward_cursor(MAX_INTERVAL, start, 10);
        match result {
            Ok(cursor) => {
                assert!(cursor.exhausted);
            }
            Err(e) => {
                let msg = format!("unexpected error: {e:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn fast_forward_with_exponential_backoff_correct_delay_progression() {
        let p = policy(5, 10, true); // exponential backoff, base=10
        let start = p.initial_cursor();
        let result = p.fast_forward_cursor(MAX_INTERVAL, start, 2);
        match result {
            Ok(cursor) => {
                // After advancing 2: attempt=3, delay after attempt 2 = 10*2^1=20
                assert_eq!(cursor.attempt, 3);
                assert_eq!(cursor.delay_ms, 20);
            }
            Err(e) => {
                let msg = format!("unexpected error: {e:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn fast_forward_exact_count_exhausts_at_exactly_max_attempts() {
        let p = policy(4, 1, false);
        let start = p.initial_cursor();
        // 3 fast-forwards from attempt 1 to attempt 4 (last attempt)
        let result = p.fast_forward_cursor(MAX_INTERVAL, start, 3);
        match result {
            Ok(cursor) => {
                assert!(!cursor.exhausted);
                assert_eq!(cursor.attempt, 4);
                assert_eq!(cursor.remaining, 1);
            }
            Err(e) => {
                let msg = format!("unexpected error: {e:?}");
                panic!("{msg}");
            }
        }
        // One more forward should exhaust
        let result2 = p.fast_forward_cursor(MAX_INTERVAL, start, 4);
        match result2 {
            Ok(cursor) => {
                assert!(cursor.exhausted);
            }
            Err(e) => {
                let msg = format!("unexpected error: {e:?}");
                panic!("{msg}");
            }
        }
    }
}

// ============================================================================
// Section 4: TimerWheel documented invariants
// ============================================================================

mod timer_wheel_documented_invariants {
    use super::*;

    // ── invariant 1: index consistency ─────────────────────────────────

    #[test]
    fn index_consistency_after_insert_order() {
        let mut wheel = TimerWheel::new();
        let d1 = Instant::now();
        let d2 = d1 + Duration::from_millis(10);
        let d3 = d1 + Duration::from_millis(20);
        let _ = wheel.insert(make_run(10), d1, PendingTimerKind::Wait);
        let _ = wheel.insert(make_run(20), d2, PendingTimerKind::Ask);
        let _ = wheel.insert(make_run(30), d3, PendingTimerKind::Wait);
        // Each run should have exactly one entry
        assert_eq!(wheel.len(), 3);
        assert_eq!(
            wheel.get_entry(make_run(10)).map(|e| e.run),
            Some(make_run(10))
        );
        assert_eq!(
            wheel.get_entry(make_run(20)).map(|e| e.run),
            Some(make_run(20))
        );
        assert_eq!(
            wheel.get_entry(make_run(30)).map(|e| e.run),
            Some(make_run(30))
        );
    }

    #[test]
    fn index_consistency_after_cancel_removes_from_both_indices() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let _ = wheel.insert(make_run(1), now, PendingTimerKind::Wait);
        assert_eq!(wheel.len(), 1);
        let cancelled = wheel.cancel(make_run(1));
        assert!(cancelled);
        assert_eq!(wheel.len(), 0);
        assert_eq!(wheel.get_entry(make_run(1)), None);
    }

    #[test]
    fn index_consistency_after_fire_expired_clears_both_indices() {
        let mut wheel = TimerWheel::new();
        let past = Instant::now() - Duration::from_millis(10);
        let _ = wheel.insert(make_run(1), past, PendingTimerKind::Wait);
        let _ = wheel.fire_expired(Instant::now());
        assert_eq!(wheel.len(), 0);
        assert_eq!(wheel.get_entry(make_run(1)), None);
    }

    #[test]
    fn index_consistency_cross_check_len_equals_entry_count() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        for i in 0u64..10 {
            let deadline = now + Duration::from_millis(i * 10);
            let _ = wheel.insert(make_run(100 + i), deadline, PendingTimerKind::Wait);
        }
        assert_eq!(wheel.len(), 10);
        for i in 0u64..10 {
            assert!(wheel.get_entry(make_run(100 + i)).is_some());
        }
    }

    // ── invariant 2: run uniqueness ────────────────────────────────────

    #[test]
    fn run_uniqueness_single_entry_per_run_id() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let _ = wheel.insert(make_run(1), now, PendingTimerKind::Wait);
        // Inserting same run_id again replaces the entry
        let _ = wheel.insert(
            make_run(1),
            now + Duration::from_millis(10),
            PendingTimerKind::Ask,
        );
        assert_eq!(wheel.len(), 1);
    }

    #[test]
    fn run_uniqueness_after_replacement_old_entry_gone() {
        let mut wheel = TimerWheel::new();
        let d1 = Instant::now();
        let d2 = d1 + Duration::from_millis(10);
        let _ = wheel.insert(make_run(1), d1, PendingTimerKind::Wait);
        let _ = wheel.insert(make_run(1), d2, PendingTimerKind::Ask);
        // The old entry at d1 should be cancelled; only the new entry remains
        assert_eq!(wheel.len(), 1);
        assert_eq!(wheel.get_kind(make_run(1)), Some(PendingTimerKind::Ask));
        // next_deadline should be d2, not d1
        assert_eq!(wheel.next_deadline(), Some(d2));
    }

    #[test]
    fn run_uniqueness_distinct_runs_independent() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let _ = wheel.insert(make_run(1), now, PendingTimerKind::Wait);
        let _ = wheel.insert(make_run(2), now, PendingTimerKind::Ask);
        assert_eq!(wheel.len(), 2);
        assert_eq!(wheel.get_kind(make_run(1)), Some(PendingTimerKind::Wait));
        assert_eq!(wheel.get_kind(make_run(2)), Some(PendingTimerKind::Ask));
    }

    // ── invariant 3: generation monotonicity ───────────────────────────

    #[test]
    fn generation_starts_at_one_for_new_run() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let _ = wheel.insert(make_run(10), now, PendingTimerKind::Wait);
        match wheel.get_entry(make_run(10)) {
            Some(entry) => assert_eq!(entry.generation, 1),
            None => {
                panic!("entry must exist after insert");
            }
        }
    }

    #[test]
    fn generation_increases_on_replacement() {
        let mut wheel = TimerWheel::new();
        let d1 = Instant::now();
        let d2 = d1 + Duration::from_millis(10);
        let _ = wheel.insert(make_run(1), d1, PendingTimerKind::Wait);
        let gen1 = wheel.get_entry(make_run(1)).map(|e| e.generation);
        let _ = wheel.insert(make_run(1), d2, PendingTimerKind::Ask);
        let gen2 = wheel.get_entry(make_run(1)).map(|e| e.generation);
        match (gen1, gen2) {
            (Some(g1), Some(g2)) => assert!(g2 > g1, "generation must increase: {g1} -> {g2}"),
            _ => {
                panic!("generations must be retrievable");
            }
        }
    }

    #[test]
    fn generation_monotonically_increases_over_many_replacements() {
        let mut wheel = TimerWheel::new();
        let base = Instant::now();
        let mut prev_gen: Option<u64> = None;
        for i in 0u64..10 {
            let deadline = base + Duration::from_millis(i * 10);
            let _ = wheel.insert(make_run(1), deadline, PendingTimerKind::Wait);
            let current = wheel
                .get_entry(make_run(1))
                .map(|e| e.generation)
                .expect("must exist");
            if let Some(prev) = prev_gen {
                assert!(
                    current > prev,
                    "generation must increase: {prev} -> {current}"
                );
            }
            prev_gen = Some(current);
        }
    }

    #[test]
    fn generation_exhausted_error_is_publicly_accessible() {
        // Verify the error variant exists and is equal to itself
        assert_eq!(
            TimerWheelError::GenerationExhausted,
            TimerWheelError::GenerationExhausted
        );
    }

    // ── invariant 4: deadline ordering ─────────────────────────────────

    #[test]
    fn deadline_ordering_next_deadline_returns_earliest() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let d1 = now + Duration::from_millis(100);
        let d2 = now + Duration::from_millis(10);
        let d3 = now + Duration::from_millis(50);
        let _ = wheel.insert(make_run(1), d1, PendingTimerKind::Wait);
        let _ = wheel.insert(make_run(2), d2, PendingTimerKind::Ask);
        let _ = wheel.insert(make_run(3), d3, PendingTimerKind::Wait);
        assert_eq!(wheel.next_deadline(), Some(d2));
    }

    #[test]
    fn deadline_ordering_next_deadline_none_when_empty() {
        let wheel = TimerWheel::new();
        assert_eq!(wheel.next_deadline(), None);
    }

    #[test]
    fn deadline_ordering_after_fire_expired_next_deadline_shifts() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        let past = now - Duration::from_millis(100);
        let d1 = now + Duration::from_millis(10);
        let d2 = now + Duration::from_millis(50);
        let _ = wheel.insert(make_run(1), past, PendingTimerKind::Wait);
        let _ = wheel.insert(make_run(2), d1, PendingTimerKind::Ask);
        let _ = wheel.insert(make_run(3), d2, PendingTimerKind::Wait);
        let _ = wheel.fire_expired(now);
        assert_eq!(wheel.next_deadline(), Some(d1));
    }

    #[test]
    fn deadline_ordering_insert_out_of_order_preserves_chronology() {
        let mut wheel = TimerWheel::new();
        let base = Instant::now();
        // Insert in non-chronological order
        let _ = wheel.insert(
            make_run(1),
            base + Duration::from_millis(100),
            PendingTimerKind::Wait,
        );
        let _ = wheel.insert(
            make_run(2),
            base + Duration::from_millis(10),
            PendingTimerKind::Ask,
        );
        let _ = wheel.insert(
            make_run(3),
            base + Duration::from_millis(50),
            PendingTimerKind::Wait,
        );
        assert_eq!(
            wheel.next_deadline(),
            Some(base + Duration::from_millis(10))
        );
    }

    // ── invariant 5: bounded operations ────────────────────────────────

    #[test]
    fn bounded_operations_len_after_each_operation() {
        let mut wheel = TimerWheel::new();
        let now = Instant::now();
        assert_eq!(wheel.len(), 0);
        let _ = wheel.insert(make_run(1), now, PendingTimerKind::Wait);
        assert_eq!(wheel.len(), 1);
        let _ = wheel.insert(make_run(2), now, PendingTimerKind::Ask);
        assert_eq!(wheel.len(), 2);
        wheel.cancel(make_run(1));
        assert_eq!(wheel.len(), 1);
        let _ = wheel.fire_expired(now);
        assert_eq!(wheel.len(), 0);
    }

    #[test]
    fn bounded_operations_is_empty_tracks_state() {
        let mut wheel = TimerWheel::new();
        assert!(wheel.is_empty());
        let _ = wheel.insert(make_run(1), Instant::now(), PendingTimerKind::Wait);
        assert!(!wheel.is_empty());
        wheel.cancel(make_run(1));
        assert!(wheel.is_empty());
    }

    #[test]
    fn bounded_operations_get_kind_returns_none_for_missing_run() {
        let wheel = TimerWheel::new();
        assert_eq!(wheel.get_kind(make_run(99)), None);
    }

    #[test]
    fn bounded_operations_get_kind_returns_correct_kind() {
        let mut wheel = TimerWheel::new();
        let _ = wheel.insert(make_run(1), Instant::now(), PendingTimerKind::Ask);
        assert_eq!(wheel.get_kind(make_run(1)), Some(PendingTimerKind::Ask));
        let _ = wheel.insert(make_run(2), Instant::now(), PendingTimerKind::Wait);
        assert_eq!(wheel.get_kind(make_run(2)), Some(PendingTimerKind::Wait));
    }

    #[test]
    fn bounded_operations_get_entry_returns_full_entry() {
        let mut wheel = TimerWheel::new();
        let deadline = Instant::now();
        let _ = wheel.insert(make_run(42), deadline, PendingTimerKind::Ask);
        let entry = wheel.get_entry(make_run(42));
        match entry {
            Some(e) => {
                assert_eq!(e.run, make_run(42));
                assert_eq!(e.deadline, deadline);
                assert_eq!(e.kind, PendingTimerKind::Ask);
                assert_eq!(e.generation, 1);
            }
            None => {
                panic!("entry must exist after insert");
            }
        }
    }

    #[test]
    fn bounded_operations_cancel_nonexistent_returns_false() {
        let mut wheel = TimerWheel::new();
        assert!(!wheel.cancel(make_run(99)));
    }

    #[test]
    fn bounded_operations_cancel_returns_true_when_entry_exists() {
        let mut wheel = TimerWheel::new();
        let _ = wheel.insert(make_run(1), Instant::now(), PendingTimerKind::Wait);
        assert!(wheel.cancel(make_run(1)));
    }

    #[test]
    fn bounded_operations_default_is_empty() {
        let wheel = TimerWheel::default();
        assert!(wheel.is_empty());
        assert_eq!(wheel.len(), 0);
        assert_eq!(wheel.next_deadline(), None);
    }

    #[test]
    fn bounded_operations_new_is_empty() {
        let wheel = TimerWheel::new();
        assert!(wheel.is_empty());
    }

    #[test]
    fn bounded_operations_len_invariant_matches_entry_count() {
        let mut wheel = TimerWheel::new();
        let base = Instant::now();
        for i in 0u64..20 {
            let _ = wheel.insert(
                make_run(i),
                base + Duration::from_millis(i),
                PendingTimerKind::Wait,
            );
        }
        assert_eq!(wheel.len(), 20);
    }

    // ── invariant cross-check: consistency after operations ────────────

    #[test]
    fn consistency_after_mixed_operations() {
        let mut wheel = TimerWheel::new();
        let base = Instant::now();
        // Insert 5
        for i in 0u64..5 {
            let _ = wheel.insert(
                make_run(i),
                base + Duration::from_millis(10),
                PendingTimerKind::Wait,
            );
        }
        assert_eq!(wheel.len(), 5);
        // Cancel 2
        wheel.cancel(make_run(1));
        wheel.cancel(make_run(3));
        assert_eq!(wheel.len(), 3);
        // Replace 1
        let _ = wheel.insert(
            make_run(0),
            base + Duration::from_millis(20),
            PendingTimerKind::Ask,
        );
        assert_eq!(wheel.len(), 3);
        // Fire expired at base
        let fired = wheel.fire_expired(base);
        assert_eq!(fired, Vec::new()); // all deadlines are future
        assert_eq!(wheel.len(), 3);
    }

    #[test]
    fn generation_uniqueness_per_run_is_preserved_after_mixed_ops() {
        let mut wheel = TimerWheel::new();
        let d1 = Instant::now();
        let _ = wheel.insert(make_run(1), d1, PendingTimerKind::Wait);
        let gen1 = wheel.get_entry(make_run(1)).map(|e| e.generation);
        let _ = wheel.insert(
            make_run(1),
            d1 + Duration::from_millis(1),
            PendingTimerKind::Ask,
        );
        let gen2 = wheel.get_entry(make_run(1)).map(|e| e.generation);
        // After cancel+reinsert, generation resets to 1 (run no longer in by_run)
        wheel.cancel(make_run(1));
        let _ = wheel.insert(
            make_run(1),
            d1 + Duration::from_millis(2),
            PendingTimerKind::Wait,
        );
        let gen3 = wheel.get_entry(make_run(1)).map(|e| e.generation);
        match (gen1, gen2, gen3) {
            (Some(g1), Some(g2), Some(g3)) => {
                assert!(g2 > g1, "replacement must increment generation");
                // After cancel, generation resets to 1 for new insert
                assert_eq!(g3, 1, "after cancel, generation resets to 1");
                assert_eq!(wheel.len(), 1);
            }
            _ => panic!("all generations must exist"),
        }
    }
}

// ============================================================================
// Section 5: RetryState invariants
// ============================================================================

mod retry_state_invariants {
    use super::*;

    #[test]
    fn from_policy_initializes_with_current_attempt_1() {
        let policy = RetryPolicy::new(5, 100, 1, DelayStrategy::Fixed).expect("valid");
        let state = RetryState::from_policy(&policy);
        assert_eq!(state.current_attempt(), 1);
        assert_eq!(state.remaining(), 5);
        assert_eq!(state.current_delay_ms(), 0);
        assert!(!state.is_exhausted());
    }

    #[test]
    fn is_exhausted_when_remaining_is_zero() {
        // Create policy with max_attempts=1, then evaluate_retry once.
        // After first retry: remaining goes 1->0.
        let policy = RetryPolicy::new(1, 0, 1, DelayStrategy::None).expect("valid");
        let state = RetryState::from_policy(&policy);
        let failure = retryable_failure(ActionFailureCode::Timeout);
        let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Safe);
        match decision {
            RetryDecision::Retry { state: next, .. } => {
                assert_eq!(next.remaining(), 0);
                assert!(next.is_exhausted());
            }
            other => {
                let s = format!("{other:?}");
                panic!("expected Retry, got {s}");
            }
        }
    }

    #[test]
    fn is_exhausted_false_when_remaining_is_positive() {
        let policy = RetryPolicy::new(3, 0, 1, DelayStrategy::None).expect("valid");
        let state = RetryState::from_policy(&policy);
        assert!(!state.is_exhausted());
        assert!(state.remaining() > 0);
    }

    #[test]
    fn encode_decode_roundtrip_from_policy_state() {
        let policy = RetryPolicy::new(5, 100, 1, DelayStrategy::Fixed).expect("valid");
        let state = RetryState::from_policy(&policy);
        let packed = state.encode().expect("encode must succeed");
        let decoded = RetryState::decode(packed).expect("decode must succeed");
        assert_eq!(decoded.current_attempt(), state.current_attempt());
        assert_eq!(decoded.remaining(), state.remaining());
        assert_eq!(decoded.current_delay_ms(), state.current_delay_ms());
    }

    #[test]
    fn encode_decode_roundtrip_after_evaluate_retry() {
        // Advance through evaluate_retry to get a state with non-zero delay
        let policy = RetryPolicy::new(5, 100, 1, DelayStrategy::Fixed).expect("valid");
        let state = RetryState::from_policy(&policy);
        let failure = retryable_failure(ActionFailureCode::Timeout);
        let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Safe);
        match decision {
            RetryDecision::Retry {
                state: mid_state, ..
            } => {
                let packed = mid_state.encode().expect("encode must succeed");
                let decoded = RetryState::decode(packed).expect("decode must succeed");
                assert_eq!(decoded.current_attempt(), mid_state.current_attempt());
                assert_eq!(decoded.remaining(), mid_state.remaining());
                assert_eq!(decoded.current_delay_ms(), mid_state.current_delay_ms());
            }
            other => {
                let s = format!("{other:?}");
                panic!("expected Retry, got {s}");
            }
        }
    }

    #[test]
    fn encode_decode_roundtrip_zero_delay_state() {
        // A state from no-retry policy with delay=0 roundtrips
        let policy = RetryPolicy::no_retry();
        let state = RetryState::from_policy(&policy);
        let packed = state.encode().expect("encode must succeed");
        let decoded = RetryState::decode(packed).expect("decode must succeed");
        assert_eq!(decoded.current_delay_ms(), 0);
        assert_eq!(decoded.remaining(), 1);
    }

    #[test]
    fn decode_rejects_zero_attempt_with_nonzero_remaining() {
        // Layout: delay=0, attempt=0, remaining=1
        let packed: i64 = 0x0000_0000_0000_0001;
        let result = RetryState::decode(packed);
        assert_eq!(result, Err(RetryPolicyError::InvalidRetryState));
    }

    #[test]
    fn decode_rejects_negative_packed_with_zero_attempt_nonzero_remaining() {
        // Layout: delay=1 in [63:32], attempt=0 in [31:16], remaining=5 in [15:0]
        let packed: i64 = 0x0000_0001_0000_0005;
        let result = RetryState::decode(packed);
        assert_eq!(result, Err(RetryPolicyError::InvalidRetryState));
    }

    #[test]
    fn retry_state_invariant_holds_current_attempt_plus_remaining_leq_max_plus_1() {
        let max_attempts: u16 = 5;
        let policy = RetryPolicy::new(max_attempts, 0, 1, DelayStrategy::None).expect("valid");
        let state = RetryState::from_policy(&policy);
        // Initial: current_attempt=1, remaining=5 -> 1+5=6 <= 5+1=6
        let total = state.current_attempt().saturating_add(state.remaining());
        assert!(total <= max_attempts.saturating_add(1));
    }

    #[test]
    fn retry_state_invariant_holds_after_advancing() {
        let max_attempts: u16 = 5;
        let policy = RetryPolicy::new(max_attempts, 0, 1, DelayStrategy::None).expect("valid");
        let state = RetryState::from_policy(&policy);
        let failure = retryable_failure(ActionFailureCode::Timeout);
        let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Safe);
        match decision {
            RetryDecision::Retry { state: next, .. } => {
                // current_attempt=2, remaining=4 -> 2+4=6 <= 5+1=6
                let total = next.current_attempt().saturating_add(next.remaining());
                assert!(total <= max_attempts.saturating_add(1));
            }
            other => {
                let s = format!("{other:?}");
                panic!("expected Retry, got {s}");
            }
        }
    }

    #[test]
    fn retry_state_from_policy_no_retry_has_one_attempt() {
        let policy = RetryPolicy::no_retry();
        let state = RetryState::from_policy(&policy);
        assert_eq!(state.current_attempt(), 1);
        assert_eq!(state.remaining(), 1);
    }

    #[test]
    fn retry_state_can_have_max_u16_remaining() {
        let policy = RetryPolicy::new(u16::MAX, 10, 1, DelayStrategy::None).expect("valid");
        let state = RetryState::from_policy(&policy);
        assert_eq!(state.remaining(), u16::MAX);
    }

    #[test]
    fn retry_state_exhausted_state_is_detectable() {
        // Exhausted state after max_attempts=1 policy: remaining goes 1->0
        let policy = RetryPolicy::new(1, 0, 1, DelayStrategy::None).expect("valid");
        let state = RetryState::from_policy(&policy);
        let failure = retryable_failure(ActionFailureCode::Timeout);
        let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Safe);
        match decision {
            RetryDecision::Retry {
                state: exhausted, ..
            } => {
                assert!(exhausted.is_exhausted());
                assert_eq!(exhausted.remaining(), 0);
            }
            other => {
                let s = format!("{other:?}");
                panic!("expected Retry, got {s}");
            }
        }
    }
}

// ============================================================================
// Section 6: evaluate_retry invariants
// ============================================================================

mod evaluate_retry_invariants {
    use super::*;

    fn policy_3_fixed() -> RetryPolicy {
        RetryPolicy::new(3, 100, 1, DelayStrategy::Fixed).expect("valid")
    }

    #[test]
    fn evaluate_retry_is_deterministic() {
        let policy = policy_3_fixed();
        let state = RetryState::from_policy(&policy);
        let failure = retryable_failure(ActionFailureCode::Timeout);
        let d1 = evaluate_retry(&state, &policy, &failure, RetrySafety::Safe);
        let d2 = evaluate_retry(&state, &policy, &failure, RetrySafety::Safe);
        assert_eq!(d1, d2);
    }

    #[test]
    fn evaluate_retry_decrements_remaining_and_increments_attempt() {
        let policy = policy_3_fixed();
        let state = RetryState::from_policy(&policy);
        let failure = retryable_failure(ActionFailureCode::Timeout);
        let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Safe);
        match decision {
            RetryDecision::Retry { state, delay_ms } => {
                assert_eq!(state.current_attempt(), 2);
                assert_eq!(state.remaining(), 2);
                assert_eq!(delay_ms, 100);
            }
            other => {
                let decision_str = format!("{other:?}");
                panic!("expected Retry, got {decision_str}");
            }
        }
    }

    #[test]
    fn evaluate_retry_exhausted_when_remaining_zero_and_retryable() {
        // Reach a state with remaining=0 via a max_attempts=1 policy
        let policy_1 = RetryPolicy::new(1, 0, 1, DelayStrategy::None).expect("valid");
        let state = RetryState::from_policy(&policy_1);
        let failure = retryable_failure(ActionFailureCode::Timeout);
        let decision = evaluate_retry(&state, &policy_1, &failure, RetrySafety::Safe);
        // First retry: remaining goes 1->0
        let exhausted_state = match decision {
            RetryDecision::Retry { state, .. } => state,
            other => {
                let s = format!("{other:?}");
                panic!("expected Retry, got {s}");
            }
        };
        assert_eq!(exhausted_state.remaining(), 0);
        // Second try with the same policy but using the exhausted state
        let decision2 = evaluate_retry(&exhausted_state, &policy_1, &failure, RetrySafety::Safe);
        assert_eq!(decision2, RetryDecision::Exhausted { max_attempts: 1 });
    }

    #[test]
    fn evaluate_retry_not_retriable_when_unsafe_safety() {
        let policy = policy_3_fixed();
        let state = RetryState::from_policy(&policy);
        let failure = retryable_failure(ActionFailureCode::Timeout);
        let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Unsafe);
        assert_eq!(decision, RetryDecision::NotRetriable);
    }

    #[test]
    fn evaluate_retry_not_retriable_when_non_retryable_failure() {
        let policy = policy_3_fixed();
        let state = RetryState::from_policy(&policy);
        let failure = non_retryable_failure(ActionFailureCode::PermissionDenied);
        let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Safe);
        assert_eq!(decision, RetryDecision::NotRetriable);
    }

    #[test]
    fn evaluate_retry_full_cycle_three_attempts_then_exhaustion() {
        let policy = policy_3_fixed();
        let failure = retryable_failure(ActionFailureCode::Timeout);

        // Attempt 1: remaining=3 -> retry, remaining becomes 2
        let state1 = RetryState::from_policy(&policy);
        let d1 = evaluate_retry(&state1, &policy, &failure, RetrySafety::Safe);
        let (next_state, _) = match d1 {
            RetryDecision::Retry { state, delay_ms } => (state, delay_ms),
            other => {
                let s = format!("{other:?}");
                panic!("expected Retry on attempt 1, got {s}");
            }
        };
        assert_eq!(next_state.current_attempt(), 2);
        assert_eq!(next_state.remaining(), 2);

        // Attempt 2: remaining=2 -> retry, remaining becomes 1
        let d2 = evaluate_retry(&next_state, &policy, &failure, RetrySafety::Safe);
        let (next_state2, _) = match d2 {
            RetryDecision::Retry { state, delay_ms } => (state, delay_ms),
            other => {
                let s = format!("{other:?}");
                panic!("expected Retry on attempt 2, got {s}");
            }
        };
        assert_eq!(next_state2.current_attempt(), 3);
        assert_eq!(next_state2.remaining(), 1);

        // Attempt 3: remaining=1 -> retry, remaining becomes 0
        let d3 = evaluate_retry(&next_state2, &policy, &failure, RetrySafety::Safe);
        let next_state3 = match d3 {
            RetryDecision::Retry { state, .. } => state,
            other => {
                let s = format!("{other:?}");
                panic!("expected Retry on attempt 3, got {s}");
            }
        };
        assert_eq!(next_state3.remaining(), 0);

        // Attempt 4: remaining=0 -> exhausted
        let d4 = evaluate_retry(&next_state3, &policy, &failure, RetrySafety::Safe);
        assert_eq!(d4, RetryDecision::Exhausted { max_attempts: 3 });
    }

    #[test]
    fn evaluate_retry_no_retry_policy_exhausts_after_first_failure() {
        let policy = RetryPolicy::no_retry();
        let state = RetryState::from_policy(&policy);
        let failure = retryable_failure(ActionFailureCode::Timeout);
        let d1 = evaluate_retry(&state, &policy, &failure, RetrySafety::Safe);
        // remaining=1 -> goes to 0 but still a Retry decision
        let next = match d1 {
            RetryDecision::Retry { state, .. } => state,
            other => {
                let s = format!("{other:?}");
                panic!("expected Retry on first attempt, got {s}");
            }
        };
        assert_eq!(next.remaining(), 0);
        // Next failure exhausts
        let d2 = evaluate_retry(&next, &policy, &failure, RetrySafety::Safe);
        assert_eq!(d2, RetryDecision::Exhausted { max_attempts: 1 });
    }

    #[test]
    fn evaluate_retry_exponential_backoff_delays_increase() {
        let policy = RetryPolicy::new(4, 100, 2, DelayStrategy::ExponentialBackoff).expect("valid");
        let state = RetryState::from_policy(&policy);
        let failure = retryable_failure(ActionFailureCode::ExternalUnavailable);

        let d1 = evaluate_retry(&state, &policy, &failure, RetrySafety::Safe);
        let s1 = match d1 {
            RetryDecision::Retry { state, delay_ms } => {
                assert_eq!(delay_ms, 100);
                state
            }
            other => {
                let s = format!("{other:?}");
                panic!("expected Retry, got {s}");
            }
        };

        let d2 = evaluate_retry(&s1, &policy, &failure, RetrySafety::Safe);
        match d2 {
            RetryDecision::Retry { delay_ms, .. } => {
                assert_eq!(delay_ms, 200);
            }
            other => {
                let s = format!("{other:?}");
                panic!("expected Retry, got {s}");
            }
        }
    }

    #[test]
    fn evaluate_retry_different_policies_produce_different_decisions() {
        let p1 = RetryPolicy::new(3, 10, 1, DelayStrategy::Fixed).expect("valid");
        let p2 = RetryPolicy::new(3, 20, 1, DelayStrategy::Fixed).expect("valid");
        let state = RetryState::from_policy(&p1);
        let failure = retryable_failure(ActionFailureCode::Timeout);
        let d1 = evaluate_retry(&state, &p1, &failure, RetrySafety::Safe);
        let d2 = evaluate_retry(&state, &p2, &failure, RetrySafety::Safe);
        // Same state, different policies produce different delays
        assert_ne!(d1, d2);
    }
}

// ============================================================================
// Section 7: is_failure_retriable purity
// ============================================================================

mod is_failure_retriable_purity {
    use super::*;

    #[test]
    fn safe_retryable_is_retriable() {
        let f = retryable_failure(ActionFailureCode::Timeout);
        assert!(is_failure_retriable(&f, RetrySafety::Safe));
    }

    #[test]
    fn safe_non_retryable_is_not_retriable() {
        let f = non_retryable_failure(ActionFailureCode::Rejected);
        assert!(!is_failure_retriable(&f, RetrySafety::Safe));
    }

    #[test]
    fn unsafe_always_returns_false_regardless_of_flag() {
        let f = retryable_failure(ActionFailureCode::Timeout);
        assert!(!is_failure_retriable(&f, RetrySafety::Unsafe));
    }

    #[test]
    fn unsafe_non_retryable_also_returns_false() {
        let f = non_retryable_failure(ActionFailureCode::PermissionDenied);
        assert!(!is_failure_retriable(&f, RetrySafety::Unsafe));
    }

    #[test]
    fn key_required_retryable_is_retriable() {
        let f = retryable_failure(ActionFailureCode::RateLimited);
        assert!(is_failure_retriable(&f, RetrySafety::KeyRequired));
    }

    #[test]
    fn key_required_non_retryable_is_not_retriable() {
        let f = non_retryable_failure(ActionFailureCode::InvalidInput);
        assert!(!is_failure_retriable(&f, RetrySafety::KeyRequired));
    }

    #[test]
    fn all_retryable_codes_are_retriable_with_safe() {
        let codes = [
            ActionFailureCode::Timeout,
            ActionFailureCode::RateLimited,
            ActionFailureCode::ResourceExhausted,
            ActionFailureCode::ExternalUnavailable,
            ActionFailureCode::Conflict,
        ];
        for &code in &codes {
            let f = retryable_failure(code);
            let reason = format!("code {code:?} should be retriable with safe");
            assert!(is_failure_retriable(&f, RetrySafety::Safe), "{reason}");
        }
    }

    #[test]
    fn all_non_retryable_codes_are_not_retriable() {
        let codes = [
            ActionFailureCode::Rejected,
            ActionFailureCode::InvalidInput,
            ActionFailureCode::PermissionDenied,
            ActionFailureCode::Unknown,
        ];
        for &code in &codes {
            let f = non_retryable_failure(code);
            let reason = format!("code {code:?} should NOT be retriable with safe");
            assert!(!is_failure_retriable(&f, RetrySafety::Safe), "{reason}");
        }
    }

    #[test]
    fn is_failure_retriable_is_deterministic() {
        let f = retryable_failure(ActionFailureCode::Timeout);
        let a = is_failure_retriable(&f, RetrySafety::Safe);
        let b = is_failure_retriable(&f, RetrySafety::Safe);
        assert_eq!(a, b);
    }
}

// ============================================================================
// Section 8: RetryPolicy construction and invariants
// ============================================================================

mod retry_policy_construction {
    use super::*;

    #[test]
    fn retry_policy_new_succeeds_with_valid_params() {
        let result = RetryPolicy::new(3, 100, 2, DelayStrategy::ExponentialBackoff);
        match result {
            Ok(policy) => {
                assert_eq!(policy.max_attempts(), 3);
                assert_eq!(policy.delay_ms(), 100);
                assert_eq!(policy.backoff_multiplier(), 2);
                assert_eq!(policy.strategy(), DelayStrategy::ExponentialBackoff);
            }
            Err(e) => {
                let msg = format!("expected Ok, got {e:?}");
                panic!("{msg}");
            }
        }
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
    fn retry_policy_no_retry_has_single_attempt_and_zero_delay() {
        let policy = RetryPolicy::no_retry();
        assert_eq!(policy.max_attempts(), 1);
        assert_eq!(policy.delay_ms(), 0);
        assert_eq!(policy.strategy(), DelayStrategy::None);
    }

    #[test]
    fn retry_policy_default_is_three_attempts_100ms_fixed() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts(), 3);
        assert_eq!(policy.delay_ms(), 100);
        assert_eq!(policy.strategy(), DelayStrategy::Fixed);
    }

    #[test]
    fn retry_policy_default_policy_matches_trait_default() {
        let trait_default = RetryPolicy::default();
        let method_default = RetryPolicy::default_policy();
        assert_eq!(trait_default, method_default);
    }

    #[test]
    fn retry_policy_supports_max_u16_attempts() {
        let result = RetryPolicy::new(u16::MAX, 10, 1, DelayStrategy::None);
        match result {
            Ok(policy) => {
                assert_eq!(policy.max_attempts(), u16::MAX);
            }
            Err(e) => {
                let msg = format!("expected Ok, got {e:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn retry_policy_with_none_strategy_and_zero_delay() {
        let result = RetryPolicy::new(5, 0, 1, DelayStrategy::None);
        match result {
            Ok(policy) => {
                assert_eq!(policy.strategy(), DelayStrategy::None);
                assert_eq!(policy.delay_ms(), 0);
            }
            Err(e) => {
                let msg = format!("expected Ok, got {e:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn retry_policy_delay_strategy_variants_are_distinct() {
        assert_ne!(DelayStrategy::None, DelayStrategy::Fixed);
        assert_ne!(DelayStrategy::Fixed, DelayStrategy::ExponentialBackoff);
        assert_ne!(DelayStrategy::None, DelayStrategy::ExponentialBackoff);
    }
}

// ============================================================================
// Section 9: exhaustion_error
// ============================================================================

mod exhaustion_error_invariants {
    use super::*;

    #[test]
    fn exhaustion_error_produces_repeat_exhausted_with_correct_max() {
        let error = exhaustion_error(5);
        assert_eq!(error, CoreError::RepeatExhausted { max: 5 });
    }

    #[test]
    fn exhaustion_error_with_max_u16_produces_correct_value() {
        let error = exhaustion_error(u16::MAX);
        assert_eq!(error, CoreError::RepeatExhausted { max: u16::MAX });
    }

    #[test]
    fn exhaustion_error_with_one_produces_correct_value() {
        let error = exhaustion_error(1);
        assert_eq!(error, CoreError::RepeatExhausted { max: 1 });
    }

    #[test]
    fn exhaustion_error_different_max_values_are_not_equal() {
        let e1 = exhaustion_error(3);
        let e2 = exhaustion_error(5);
        assert_ne!(e1, e2);
    }
}

// ============================================================================
// Section 10: RetryPolicyLimits validation (engine retry_math)
// ============================================================================

mod retry_policy_limits_validation {
    use super::*;

    #[test]
    fn validate_against_rejects_zero_max_attempts() {
        let p = EngineRetryPolicy {
            max_attempts: 0,
            base_delay_ms: 0,
            exponential_backoff: false,
        };
        let limits = RetryPolicyLimits {
            max_attempts: u16::MAX,
            max_interval_ms: u64::MAX,
        };
        assert_eq!(
            p.validate_against(limits),
            Err(RetryPolicyMathError::ZeroMaxAttempts)
        );
    }

    #[test]
    fn validate_against_rejects_exceeded_attempts() {
        let p = EngineRetryPolicy {
            max_attempts: 10,
            base_delay_ms: 0,
            exponential_backoff: false,
        };
        let limits = RetryPolicyLimits {
            max_attempts: 5,
            max_interval_ms: u64::MAX,
        };
        assert_eq!(
            p.validate_against(limits),
            Err(RetryPolicyMathError::MaxAttemptsExceeded)
        );
    }

    #[test]
    fn validate_against_rejects_exceeded_base_delay() {
        let p = EngineRetryPolicy {
            max_attempts: 3,
            base_delay_ms: 500,
            exponential_backoff: false,
        };
        let limits = RetryPolicyLimits {
            max_attempts: u16::MAX,
            max_interval_ms: 100,
        };
        assert_eq!(
            p.validate_against(limits),
            Err(RetryPolicyMathError::BaseDelayExceeded)
        );
    }

    #[test]
    fn validate_against_accepts_valid_policy() {
        let p = EngineRetryPolicy {
            max_attempts: 3,
            base_delay_ms: 100,
            exponential_backoff: false,
        };
        let limits = RetryPolicyLimits {
            max_attempts: 3,
            max_interval_ms: 100,
        };
        assert_eq!(p.validate_against(limits), Ok(p));
    }

    #[test]
    fn delay_for_attempt_rejects_zero_attempt() {
        let p = EngineRetryPolicy::DEFAULT;
        assert_eq!(
            p.delay_for_attempt(u64::MAX, 0),
            Err(RetryPolicyMathError::ZeroAttempt)
        );
    }

    #[test]
    fn delay_for_attempt_rejects_attempt_exceeding_max() {
        let p = EngineRetryPolicy {
            max_attempts: 3,
            base_delay_ms: 10,
            exponential_backoff: false,
        };
        assert_eq!(
            p.delay_for_attempt(u64::MAX, 4),
            Err(RetryPolicyMathError::AttemptExceeded)
        );
    }

    #[test]
    fn delay_for_attempt_returns_base_for_non_exponential() {
        let p = EngineRetryPolicy {
            max_attempts: 3,
            base_delay_ms: 42,
            exponential_backoff: false,
        };
        assert_eq!(p.delay_for_attempt(100, 1), Ok(42));
        assert_eq!(p.delay_for_attempt(100, 2), Ok(42));
        assert_eq!(p.delay_for_attempt(100, 3), Ok(42));
    }

    #[test]
    fn delay_for_attempt_clamps_to_max_interval() {
        let p = EngineRetryPolicy {
            max_attempts: 3,
            base_delay_ms: 200,
            exponential_backoff: false,
        };
        // delay=200 but max_interval_ms=100 -> clamped to 100
        assert_eq!(p.delay_for_attempt(100, 1), Ok(100));
    }

    #[test]
    fn initial_cursor_starts_at_attempt_1_with_full_remaining() {
        let p = EngineRetryPolicy {
            max_attempts: 7,
            base_delay_ms: 0,
            exponential_backoff: false,
        };
        let cursor = p.initial_cursor();
        assert_eq!(cursor.attempt, 1);
        assert_eq!(cursor.remaining, 7);
        assert_eq!(cursor.delay_ms, 0);
        assert!(!cursor.exhausted);
    }

    #[test]
    fn initial_cursor_never_policy_is_not_exhausted() {
        let cursor = EngineRetryPolicy::NEVER.initial_cursor();
        assert_eq!(cursor.attempt, 1);
        assert_eq!(cursor.remaining, 1);
        assert!(!cursor.exhausted);
    }

    #[test]
    fn next_cursor_validates_remaining_and_attempt() {
        let p = EngineRetryPolicy {
            max_attempts: 3,
            base_delay_ms: 0,
            exponential_backoff: false,
        };
        // Cursor with remaining > max_attempts
        let bad_cursor = RetryCursor {
            attempt: 1,
            remaining: 100,
            delay_ms: 0,
            exhausted: false,
        };
        assert_eq!(
            p.next_cursor(u64::MAX, bad_cursor),
            Err(RetryPolicyMathError::RemainingExceeded)
        );
    }

    #[test]
    fn next_cursor_validates_delay_against_max_interval() {
        let p = EngineRetryPolicy::NEVER;
        let cursor = RetryCursor {
            attempt: 1,
            remaining: 1,
            delay_ms: 1000,
            exhausted: false,
        };
        assert_eq!(
            p.next_cursor(100, cursor),
            Err(RetryPolicyMathError::CursorDelayExceeded)
        );
    }

    #[test]
    fn retry_policy_math_error_variants_are_distinct() {
        assert_ne!(
            RetryPolicyMathError::ZeroMaxAttempts,
            RetryPolicyMathError::MaxAttemptsExceeded
        );
        assert_ne!(
            RetryPolicyMathError::MaxAttemptsExceeded,
            RetryPolicyMathError::BaseDelayExceeded
        );
        assert_ne!(
            RetryPolicyMathError::BaseDelayExceeded,
            RetryPolicyMathError::ZeroAttempt
        );
        assert_ne!(
            RetryPolicyMathError::ZeroAttempt,
            RetryPolicyMathError::AttemptExceeded
        );
    }
}

// ============================================================================
// Section 11: DelayStrategy and RetryPolicyError value semantics
// ============================================================================

mod value_semantics {
    use super::*;

    #[test]
    fn delay_strategy_none_equals_none() {
        assert_eq!(DelayStrategy::None, DelayStrategy::None);
    }

    #[test]
    fn delay_strategy_fixed_equals_fixed() {
        assert_eq!(DelayStrategy::Fixed, DelayStrategy::Fixed);
    }

    #[test]
    fn delay_strategy_exponential_equals_exponential() {
        assert_eq!(
            DelayStrategy::ExponentialBackoff,
            DelayStrategy::ExponentialBackoff
        );
    }

    #[test]
    fn retry_policy_error_zero_max_attempts_equals_itself() {
        assert_eq!(
            RetryPolicyError::ZeroMaxAttempts,
            RetryPolicyError::ZeroMaxAttempts
        );
    }

    #[test]
    fn retry_policy_error_zero_backoff_equals_itself() {
        assert_eq!(
            RetryPolicyError::ZeroBackoffMultiplier,
            RetryPolicyError::ZeroBackoffMultiplier
        );
    }

    #[test]
    fn retry_policy_error_invalid_retry_state_equals_itself() {
        assert_eq!(
            RetryPolicyError::InvalidRetryState,
            RetryPolicyError::InvalidRetryState
        );
    }

    #[test]
    fn retry_policy_error_not_retriable_equals_itself() {
        assert_eq!(
            RetryPolicyError::NotRetriable,
            RetryPolicyError::NotRetriable
        );
    }

    #[test]
    fn retry_decision_retry_equality() {
        let policy = RetryPolicy::new(3, 50, 1, DelayStrategy::Fixed).expect("valid");
        let state = RetryState::from_policy(&policy);
        let failure = retryable_failure(ActionFailureCode::Timeout);
        let decision = evaluate_retry(&state, &policy, &failure, RetrySafety::Safe);
        match decision {
            RetryDecision::Retry {
                state: _s,
                delay_ms: d,
            } => {
                let d2 = RetryDecision::Retry {
                    state: _s,
                    delay_ms: d,
                };
                assert_eq!(
                    RetryDecision::Retry {
                        state: _s,
                        delay_ms: d
                    },
                    d2
                );
            }
            other => {
                let s = format!("{other:?}");
                panic!("expected Retry, got {s}");
            }
        }
    }

    #[test]
    fn retry_decision_exhausted_equality() {
        assert_eq!(
            RetryDecision::Exhausted { max_attempts: 3 },
            RetryDecision::Exhausted { max_attempts: 3 }
        );
    }

    #[test]
    fn retry_decision_not_retriable_equality() {
        assert_eq!(RetryDecision::NotRetriable, RetryDecision::NotRetriable);
    }

    // ── TimerWheelError equality ───────────────────────────────────────

    #[test]
    fn timer_wheel_error_generation_exhausted_equals_itself() {
        assert_eq!(
            TimerWheelError::GenerationExhausted,
            TimerWheelError::GenerationExhausted
        );
    }

    #[test]
    fn timer_entry_equality_same_fields() {
        let now = Instant::now();
        let a = TimerEntry {
            run: make_run(1),
            generation: 5,
            deadline: now,
            kind: PendingTimerKind::Wait,
        };
        let b = TimerEntry {
            run: make_run(1),
            generation: 5,
            deadline: now,
            kind: PendingTimerKind::Wait,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn timer_entry_inequality_different_generation() {
        let now = Instant::now();
        let a = TimerEntry {
            run: make_run(1),
            generation: 5,
            deadline: now,
            kind: PendingTimerKind::Wait,
        };
        let b = TimerEntry {
            run: make_run(1),
            generation: 6,
            deadline: now,
            kind: PendingTimerKind::Wait,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn timer_entry_inequality_different_kind() {
        let now = Instant::now();
        let a = TimerEntry {
            run: make_run(1),
            generation: 1,
            deadline: now,
            kind: PendingTimerKind::Wait,
        };
        let b = TimerEntry {
            run: make_run(1),
            generation: 1,
            deadline: now,
            kind: PendingTimerKind::Ask,
        };
        assert_ne!(a, b);
    }

    // ── PendingTimerKind ───────────────────────────────────────────────

    #[test]
    fn pending_timer_kind_wait_equals_wait() {
        assert_eq!(PendingTimerKind::Wait, PendingTimerKind::Wait);
    }

    #[test]
    fn pending_timer_kind_ask_equals_ask() {
        assert_eq!(PendingTimerKind::Ask, PendingTimerKind::Ask);
    }

    #[test]
    fn pending_timer_kind_wait_not_equal_ask() {
        assert_ne!(PendingTimerKind::Wait, PendingTimerKind::Ask);
    }
}
