#![forbid(unsafe_code)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::expect_used,
    clippy::get_first,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
//! PS-001: Deadline Arithmetic Safety — behavior tests (A1-A7).
//!
//! Tests numeric `TimerTick`, `TimerDuration`, and `TimerDeadline` construction,
//! checked arithmetic, overflow safety, and ordering. All values are deterministic
//! u64-based newtypes — no `Instant` or wall-clock time.

use vb_runtime::shard::types::{TimerDeadline, TimerDeadlineError, TimerDuration, TimerTick};

// =========================================================================
// Behavior A1: TimerTick constructor accepts valid u64
// =========================================================================

#[test]
fn timer_tick_new_accepts_zero() {
    let tick = TimerTick::new(0);
    assert_eq!(tick.get(), 0);
}

#[test]
fn timer_tick_new_accepts_midrange_value() {
    let tick = TimerTick::new(500_000);
    assert_eq!(tick.get(), 500_000);
}

#[test]
fn timer_tick_new_accepts_max_u64() {
    let tick = TimerTick::new(u64::MAX);
    assert_eq!(tick.get(), u64::MAX);
}

// =========================================================================
// Behavior A2: TimerTick boundaries
// =========================================================================

#[test]
fn timer_tick_equality_is_exact() {
    assert_eq!(TimerTick::new(42), TimerTick::new(42));
    assert_ne!(TimerTick::new(42), TimerTick::new(43));
    assert_ne!(TimerTick::new(0), TimerTick::new(1));
}

#[test]
fn timer_tick_ordering_is_by_inner_u64() {
    assert!(TimerTick::new(0) < TimerTick::new(1));
    assert!(TimerTick::new(10) > TimerTick::new(5));
    assert!(TimerTick::new(100) >= TimerTick::new(100));
    assert!(TimerTick::new(100) <= TimerTick::new(100));
}

#[test]
fn timer_tick_copy_preserves_value() {
    let t1 = TimerTick::new(99);
    let t2 = t1;
    assert_eq!(t1, t2);
    assert_eq!(t2.get(), 99);
}

// =========================================================================
// Behavior A3: TimerDuration constructor
// =========================================================================

#[test]
fn timer_duration_new_accepts_one() {
    let dur = TimerDuration::new(1);
    assert_eq!(dur.get(), 1);
}

#[test]
fn timer_duration_new_accepts_midrange_value() {
    let dur = TimerDuration::new(10_000);
    assert_eq!(dur.get(), 10_000);
    assert_eq!(dur.as_ticks(), 10_000);
}

#[test]
fn timer_duration_new_accepts_max_u64() {
    let dur = TimerDuration::new(u64::MAX);
    assert_eq!(dur.get(), u64::MAX);
}

#[test]
fn timer_duration_zero_is_zero() {
    let dur = TimerDuration::zero();
    assert_eq!(dur.get(), 0);
    assert_eq!(dur.as_ticks(), 0);
}

#[test]
fn timer_duration_equality_is_exact() {
    assert_eq!(TimerDuration::new(10), TimerDuration::new(10));
    assert_ne!(TimerDuration::new(10), TimerDuration::new(11));
}

#[test]
fn timer_duration_ordering_is_by_inner_u64() {
    assert!(TimerDuration::new(1) < TimerDuration::new(2));
    assert!(TimerDuration::new(100) > TimerDuration::new(50));
    assert!(TimerDuration::new(5) >= TimerDuration::new(5));
}

// =========================================================================
// Behavior A5: TimerDeadline from tick+duration computes checked sum
// =========================================================================

#[test]
fn timer_deadline_from_tick_and_duration_computes_exact_sum() {
    let tick = TimerTick::new(10);
    let dur = TimerDuration::new(20);
    let deadline = TimerDeadline::from_tick_and_duration(tick, dur);
    assert_eq!(deadline, Ok(TimerDeadline::new(30)));
}

#[test]
fn timer_deadline_from_tick_and_duration_zero_duration_rejected_with_zero_duration_error() {
    let tick = TimerTick::new(50);
    let dur = TimerDuration::zero();
    let err = TimerDeadline::from_tick_and_duration(tick, dur).unwrap_err();
    assert_eq!(err, TimerDeadlineError::ZeroDuration);
}

#[test]
fn timer_deadline_from_tick_and_duration_max_no_overflow() {
    // u64::MAX - 10 + 10 = u64::MAX (no overflow)
    let tick = TimerTick::new(u64::MAX - 10);
    let dur = TimerDuration::new(10);
    let deadline = TimerDeadline::from_tick_and_duration(tick, dur);
    assert_eq!(deadline, Ok(TimerDeadline::new(u64::MAX)));
}

#[test]
fn timer_deadline_from_tick_and_duration_zero_tick_zero_duration_rejected() {
    let tick = TimerTick::new(0);
    let dur = TimerDuration::new(0);
    let err = TimerDeadline::from_tick_and_duration(tick, dur).unwrap_err();
    assert_eq!(err, TimerDeadlineError::ZeroDuration);
}

#[test]
fn timer_deadline_from_tick_zero_tick_with_duration() {
    let tick = TimerTick::new(0);
    let dur = TimerDuration::new(100);
    let deadline = TimerDeadline::from_tick_and_duration(tick, dur);
    assert_eq!(deadline, Ok(TimerDeadline::new(100)));
}

// =========================================================================
// Behavior A6: TimerDeadline overflow returns Overflow error
// =========================================================================

#[test]
fn timer_deadline_from_tick_and_duration_overflow_returns_overflow_error() {
    let tick = TimerTick::new(u64::MAX);
    let dur = TimerDuration::new(1);
    let err = TimerDeadline::from_tick_and_duration(tick, dur).unwrap_err();
    assert_eq!(err, TimerDeadlineError::Overflow);
}

#[test]
fn timer_deadline_from_tick_and_duration_max_tick_plus_one_returns_overflow_error() {
    let tick = TimerTick::new(u64::MAX - 1);
    let dur = TimerDuration::new(2);
    let err = TimerDeadline::from_tick_and_duration(tick, dur).unwrap_err();
    assert_eq!(err, TimerDeadlineError::Overflow);
}

#[test]
fn timer_deadline_from_tick_and_duration_both_max_overflow() {
    let tick = TimerTick::new(u64::MAX);
    let dur = TimerDuration::new(u64::MAX);
    let err = TimerDeadline::from_tick_and_duration(tick, dur).unwrap_err();
    assert_eq!(err, TimerDeadlineError::Overflow);
}

#[test]
fn timer_deadline_from_tick_and_duration_near_max_no_overflow() {
    let tick = TimerTick::new(u64::MAX - 5);
    let dur = TimerDuration::new(5);
    assert_eq!(
        TimerDeadline::from_tick_and_duration(tick, dur),
        Ok(TimerDeadline::new(u64::MAX))
    );
}

// =========================================================================
// Behavior A7: TimerDeadline new_absolute and is_past
// =========================================================================

#[test]
fn timer_deadline_new_accepts_any_u64() {
    assert_eq!(TimerDeadline::new(0).get(), 0);
    assert_eq!(TimerDeadline::new(500).get(), 500);
    assert_eq!(TimerDeadline::new(u64::MAX).get(), u64::MAX);
}

#[test]
fn timer_deadline_is_past_when_current_equals_deadline() {
    let current = TimerTick::new(50);
    let deadline = TimerDeadline::new(50);
    assert!(deadline.is_past(current));
}

#[test]
fn timer_deadline_is_past_when_current_exceeds_deadline() {
    let current = TimerTick::new(100);
    let deadline = TimerDeadline::new(50);
    assert!(deadline.is_past(current));
}

#[test]
fn timer_deadline_is_not_past_when_current_before_deadline() {
    let current = TimerTick::new(40);
    let deadline = TimerDeadline::new(50);
    assert!(!deadline.is_past(current));
}

#[test]
fn timer_deadline_is_past_with_max_values() {
    let current = TimerTick::new(u64::MAX);
    let deadline = TimerDeadline::new(u64::MAX);
    assert!(deadline.is_past(current));
}

#[test]
fn timer_deadline_is_not_past_when_deadline_is_max_and_current_is_zero() {
    let current = TimerTick::new(0);
    let deadline = TimerDeadline::new(u64::MAX);
    assert!(!deadline.is_past(current));
}

// =========================================================================
// TimerDeadline ordering and equality
// =========================================================================

#[test]
fn timer_deadline_ordering_is_by_inner_u64() {
    assert!(TimerDeadline::new(10) < TimerDeadline::new(20));
    assert!(TimerDeadline::new(100) > TimerDeadline::new(50));
    assert_eq!(TimerDeadline::new(7), TimerDeadline::new(7));
    assert_ne!(TimerDeadline::new(7), TimerDeadline::new(8));
}

#[test]
fn timer_deadline_copy_preserves_value() {
    let d1 = TimerDeadline::new(30);
    let d2 = d1;
    assert_eq!(d1, d2);
    assert_eq!(d2.get(), 30);
}

// =========================================================================
// TimerTick checked_add (deadline construction alternative)
// =========================================================================

#[test]
fn timer_tick_checked_add_succeeds_within_range() {
    let tick = TimerTick::new(100);
    let dur = TimerDuration::new(50);
    assert_eq!(tick.checked_add(dur), Some(TimerTick::new(150)));
}

#[test]
fn timer_tick_checked_add_returns_none_on_overflow() {
    let tick = TimerTick::new(u64::MAX);
    let dur = TimerDuration::new(1);
    assert_eq!(tick.checked_add(dur), None);
}

#[test]
fn timer_tick_checked_add_zero_duration_returns_same_tick() {
    let tick = TimerTick::new(42);
    let dur = TimerDuration::zero();
    assert_eq!(tick.checked_add(dur), Some(tick));
}

// =========================================================================
// TimerTick has_elapsed
// =========================================================================

#[test]
fn timer_tick_has_elapsed_when_tick_equals_deadline() {
    let tick = TimerTick::new(100);
    let deadline = TimerDeadline::new(100);
    assert!(tick.has_elapsed(deadline));
}

#[test]
fn timer_tick_has_elapsed_when_tick_is_past_deadline() {
    let tick = TimerTick::new(101);
    let deadline = TimerDeadline::new(100);
    assert!(tick.has_elapsed(deadline));
}

#[test]
fn timer_tick_has_not_elapsed_when_tick_is_before_deadline() {
    let tick = TimerTick::new(99);
    let deadline = TimerDeadline::new(100);
    assert!(!tick.has_elapsed(deadline));
}

#[test]
fn timer_tick_has_elapsed_with_zero_values() {
    assert!(TimerTick::new(0).has_elapsed(TimerDeadline::new(0)));
    assert!(!TimerTick::new(0).has_elapsed(TimerDeadline::new(1)));
}
