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
//! PS-007: Clock Advancement — behavior tests (G1-G5).
//!
//! Tests the numeric timer seam `advance_clock_to` and `current_tick` API
//! on `Shard`. Uses deterministic `TimerTick` values instead of `Instant`.

use vb_core::ids::RunId;
use vb_runtime::shard::types::{Shard, ShardConfig, TimerTick};

fn run(id: u64) -> RunId {
    RunId::new(id)
}

// ---------- Behavior G1: Backward clock advance rejected ----------

#[test]
fn advance_clock_to_rejects_backward_tick_returns_error() {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(100)), Ok(()));
    let result = shard.advance_clock_to(TimerTick::new(50));
    assert_eq!(result, Err(vb_runtime::RuntimeError::InvalidTimerFire));
    // Current tick must be preserved after rejection
    assert_eq!(shard.current_tick(), TimerTick::new(100));
}

#[test]
fn advance_clock_to_backward_tick_preserves_current_tick() {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(1000)), Ok(()));
    // Attempt to go backward
    let _ = shard.advance_clock_to(TimerTick::new(500));
    // Tick must remain 1000
    assert_eq!(shard.current_tick(), TimerTick::new(1000));
}

#[test]
fn advance_clock_to_rejects_single_tick_backward() {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(10)), Ok(()));
    assert_eq!(
        shard.advance_clock_to(TimerTick::new(9)),
        Err(vb_runtime::RuntimeError::InvalidTimerFire)
    );
    assert_eq!(shard.current_tick(), TimerTick::new(10));
}

// ---------- Behavior G2: Equal tick advance is a no-op ----------

#[test]
fn advance_clock_to_same_tick_is_noop() {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(42)), Ok(()));
    assert_eq!(shard.advance_clock_to(TimerTick::new(42)), Ok(()));
    assert_eq!(shard.current_tick(), TimerTick::new(42));
}

#[test]
fn advance_clock_to_same_zero_tick_is_noop() {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.current_tick(), TimerTick::new(0));
    assert_eq!(shard.advance_clock_to(TimerTick::new(0)), Ok(()));
    assert_eq!(shard.current_tick(), TimerTick::new(0));
}

// ---------- Behavior G3: Forward advance fires due timers ----------

#[test]
fn advance_clock_to_forward_increments_current_tick() {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(50)), Ok(()));
    assert_eq!(shard.current_tick(), TimerTick::new(50));
    assert_eq!(shard.advance_clock_to(TimerTick::new(100)), Ok(()));
    assert_eq!(shard.current_tick(), TimerTick::new(100));
}

#[test]
fn advance_clock_to_multiple_forward_steps_are_monotonic() {
    let mut shard = Shard::new(ShardConfig::default());
    let ticks = [1u64, 5, 10, 50, 100, 500, 1000];
    for (i, &tick) in ticks.iter().enumerate() {
        assert_eq!(shard.advance_clock_to(TimerTick::new(tick)), Ok(()));
        assert_eq!(shard.current_tick(), TimerTick::new(tick));
        // Monotonic: each tick is >= previous
        if i > 0 {
            assert!(tick >= ticks[i - 1]);
        }
    }
}

#[test]
fn advance_clock_to_large_jump_succeeds() {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(0)), Ok(()));
    assert_eq!(shard.advance_clock_to(TimerTick::new(1_000_000)), Ok(()));
    assert_eq!(shard.current_tick(), TimerTick::new(1_000_000));
}

// ---------- Behavior G5: Maximum tick boundary ----------

#[test]
fn advance_clock_to_accepts_max_u64_tick() {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(u64::MAX)), Ok(()));
    assert_eq!(shard.current_tick(), TimerTick::new(u64::MAX));
}

#[test]
fn advance_clock_to_max_tick_then_reject_any_subsequent() {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(u64::MAX)), Ok(()));
    // Any tick < u64::MAX is now backward
    assert_eq!(
        shard.advance_clock_to(TimerTick::new(u64::MAX - 1)),
        Err(vb_runtime::RuntimeError::InvalidTimerFire)
    );
    // Equal tick is still OK (no-op)
    assert_eq!(shard.advance_clock_to(TimerTick::new(u64::MAX)), Ok(()));
}

// ---------- current_tick availability ----------

#[test]
fn current_tick_starts_at_zero_for_new_shard() {
    let shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.current_tick(), TimerTick::new(0));
}

#[test]
fn current_tick_returns_consistent_value() {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(77)), Ok(()));
    // Multiple reads all return the same value
    for _ in 0..10 {
        assert_eq!(shard.current_tick(), TimerTick::new(77));
    }
}

// ---------- Shard status includes tick state ----------

#[test]
fn shard_status_available_after_clock_advance() {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(100)), Ok(()));
    let status = shard.status();
    // Status is available; tick does not corrupt shard state
    assert!(status.running);
}
