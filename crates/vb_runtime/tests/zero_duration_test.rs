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
//! PS-009: Zero-Duration Determinism — behavior tests (I1-I2).
//!
//! Tests that zero-duration timer values are handled deterministically
//! using the numeric timer types. `TimerDuration::zero()` produces a
//! duration of 0 ticks. When combined with `TimerTick`, the resulting
//! `TimerDeadline` is exactly equal to the current tick.

use vb_runtime::shard::types::{TimerDeadline, TimerDeadlineError, TimerDuration, TimerTick};

// ---------- Behavior I1: Zero duration is rejected at the API boundary ----------
//
// RS-107 fix: `from_tick_and_duration` rejects zero durations with a typed
// `TimerDeadlineError::ZeroDuration` instead of silently producing a deadline
// equal to the current tick (which would fire immediately).

#[test]
fn zero_duration_is_rejected_with_zero_duration_error_at_tick_100() {
    let current_tick = TimerTick::new(100);
    let zero_dur = TimerDuration::zero();
    let err = TimerDeadline::from_tick_and_duration(current_tick, zero_dur).unwrap_err();
    assert_eq!(err, TimerDeadlineError::ZeroDuration);
}

#[test]
fn zero_duration_timer_via_construction_would_fire_immediately_at_current_tick() {
    // Documents why the fix is necessary: a raw deadline equal to the
    // current tick is *already past*. Construction must reject this.
    let current = TimerTick::new(50);
    let deadline = TimerDeadline::new(50); // raw construction of zero-duration deadline
    assert!(deadline.is_past(current));
}

#[test]
fn zero_duration_with_zero_tick_is_rejected_with_zero_duration_error() {
    let tick = TimerTick::new(0);
    let dur = TimerDuration::zero();
    let err = TimerDeadline::from_tick_and_duration(tick, dur).unwrap_err();
    assert_eq!(err, TimerDeadlineError::ZeroDuration);
}

#[test]
fn zero_duration_timer_via_raw_new_is_not_past_before_current_tick() {
    let current = TimerTick::new(49);
    let deadline = TimerDeadline::new(50);
    // Deadline is at 50, current is at 49 — deadline not yet reached
    assert!(!deadline.is_past(current));
}

// ---------- Behavior I2: Zero duration never touches host time ----------
// The numeric types are pure data — no std::time API is used.
// This is a compile-time guarantee by the type system.

#[test]
fn zero_duration_output_is_pure_function_of_inputs() {
    // Same inputs always produce same outputs (both errors)
    let tick = TimerTick::new(30);
    let dur = TimerDuration::zero();
    let r1 = TimerDeadline::from_tick_and_duration(tick, dur).unwrap_err();
    let r2 = TimerDeadline::from_tick_and_duration(tick, dur).unwrap_err();
    assert_eq!(r1, r2);
}

#[test]
fn zero_duration_same_input_produces_same_error_across_calls() {
    let tick = TimerTick::new(42);
    let dur = TimerDuration::zero();
    for _ in 0..100 {
        let err = TimerDeadline::from_tick_and_duration(tick, dur).unwrap_err();
        assert_eq!(err, TimerDeadlineError::ZeroDuration);
    }
}

#[test]
fn zero_duration_with_various_current_ticks_all_rejected() {
    for tick_val in [0u64, 1, 10, 100, 1000, u64::MAX / 2] {
        let tick = TimerTick::new(tick_val);
        let dur = TimerDuration::zero();
        let err = TimerDeadline::from_tick_and_duration(tick, dur).unwrap_err();
        assert_eq!(err, TimerDeadlineError::ZeroDuration);
    }
}

#[test]
fn zero_duration_does_not_create_valid_future_deadline() {
    // Documents the rejected path: callers cannot synthesize a deadline
    // equal to the current tick via zero duration.
    let tick = TimerTick::new(100);
    let dur = TimerDuration::zero();
    let result = TimerDeadline::from_tick_and_duration(tick, dur);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), TimerDeadlineError::ZeroDuration);
}

// ---------- TimerDuration zero() consistency ----------

#[test]
fn timer_duration_zero_is_identity_for_addition() {
    // TimerTick::checked_add is pure arithmetic; zero is its identity.
    let tick = TimerTick::new(77);
    let zero = TimerDuration::zero();
    assert_eq!(tick.checked_add(zero), Some(tick));
}

#[test]
fn timer_duration_zero_is_rejected_at_deadline_construction() {
    let tick = TimerTick::new(55);
    let zero = TimerDuration::zero();
    let err = TimerDeadline::from_tick_and_duration(tick, zero).unwrap_err();
    assert_eq!(err, TimerDeadlineError::ZeroDuration);
}

#[test]
fn timer_duration_zero_as_ticks_is_zero() {
    assert_eq!(TimerDuration::zero().as_ticks(), 0);
}

// ---------- Non-zero durations produce future deadlines ----------

#[test]
fn non_zero_duration_produces_future_deadline_when_tick_is_zero() {
    let tick = TimerTick::new(0);
    let dur = TimerDuration::new(10);
    let deadline = TimerDeadline::from_tick_and_duration(tick, dur).unwrap();
    assert_eq!(deadline, TimerDeadline::new(10));
    assert!(!deadline.is_past(TimerTick::new(0)));
    assert!(deadline.is_past(TimerTick::new(10)));
}
