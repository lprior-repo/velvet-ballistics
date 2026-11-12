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

use vb_runtime::shard::types::{TimerDeadline, TimerDuration, TimerTick};

// ---------- Behavior I1: Zero duration follows documented branch ----------

#[test]
fn zero_duration_deadline_equals_current_tick() {
    let current_tick = TimerTick::new(100);
    let zero_dur = TimerDuration::zero();
    let deadline = TimerDeadline::from_tick_and_duration(current_tick, zero_dur);
    assert_eq!(deadline, Some(TimerDeadline::new(100)));
}

#[test]
fn zero_duration_timer_is_past_at_current_tick() {
    let current = TimerTick::new(50);
    let deadline = TimerDeadline::new(50); // zero-duration deadline
    assert!(deadline.is_past(current));
}

#[test]
fn zero_duration_with_zero_tick_is_zero_deadline() {
    let tick = TimerTick::new(0);
    let dur = TimerDuration::zero();
    let deadline = TimerDeadline::from_tick_and_duration(tick, dur);
    assert_eq!(deadline, Some(TimerDeadline::new(0)));
}

#[test]
fn zero_duration_timer_is_not_past_before_current_tick() {
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
    // Same inputs always produce same outputs
    let tick = TimerTick::new(30);
    let dur = TimerDuration::zero();
    let r1 = TimerDeadline::from_tick_and_duration(tick, dur);
    let r2 = TimerDeadline::from_tick_and_duration(tick, dur);
    assert_eq!(r1, r2);
}

#[test]
fn zero_duration_same_input_produces_same_outcome_across_calls() {
    let tick = TimerTick::new(42);
    let dur = TimerDuration::zero();
    for _ in 0..100 {
        let deadline = TimerDeadline::from_tick_and_duration(tick, dur);
        assert_eq!(deadline, Some(TimerDeadline::new(42)));
    }
}

#[test]
fn zero_duration_with_various_current_ticks_always_equals_current() {
    for tick_val in [0u64, 1, 10, 100, 1000, u64::MAX / 2] {
        let tick = TimerTick::new(tick_val);
        let dur = TimerDuration::zero();
        let deadline = TimerDeadline::from_tick_and_duration(tick, dur);
        assert_eq!(deadline, Some(TimerDeadline::new(tick_val)));
    }
}

#[test]
fn zero_duration_does_not_create_future_deadline() {
    let tick = TimerTick::new(100);
    let dur = TimerDuration::zero();
    let deadline = TimerDeadline::from_tick_and_duration(tick, dur);
    assert_eq!(deadline, Some(TimerDeadline::new(100)));
    // Safe: just verified is Some
    let deadline = deadline.expect("zero duration from_tick_and_duration should never overflow");
    // Deadline is at exactly tick 100
    assert!(deadline.is_past(TimerTick::new(100)));
    assert!(deadline.is_past(TimerTick::new(101)));
    assert!(!deadline.is_past(TimerTick::new(99)));
}

// ---------- TimerDuration zero() consistency ----------

#[test]
fn timer_duration_zero_is_identity_for_addition() {
    let tick = TimerTick::new(77);
    let zero = TimerDuration::zero();
    assert_eq!(tick.checked_add(zero), Some(tick));
}

#[test]
fn timer_duration_zero_produces_same_deadline_as_no_duration() {
    let tick = TimerTick::new(55);
    let zero = TimerDuration::zero();
    let deadline_with_zero = TimerDeadline::from_tick_and_duration(tick, zero);
    // The deadline should equal the tick itself
    assert_eq!(deadline_with_zero, Some(TimerDeadline::new(55)));
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
