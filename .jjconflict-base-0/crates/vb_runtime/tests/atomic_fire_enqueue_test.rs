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
//! PS-010: Atomic Fire+Enqueue — behavior tests (J1-J3).
//!
//! Tests the atomicity of `TimerWheel` fire operations:
//! - Fire removes entries from both by_deadline and by_run indices
//! - Cancel followed by fire does not double-fire
//! - Fire on replaced timer removes only the latest entry
//! - Authority mismatch prevents fire

use std::time::{Duration, Instant};
use vb_core::ids::{RunId, StepIdx};
use vb_runtime::shard::timer_wheel::TimerWheel;
use vb_runtime::shard::types::{PendingTimer, PendingTimerKind};

fn run(id: u64) -> RunId {
    RunId::new(id)
}

// ---------- Behavior J1: Valid fire is fully atomic ----------

#[test]
fn fire_removes_timer_from_both_indices() {
    let mut wheel = TimerWheel::new();
    let past = Instant::now() - Duration::from_millis(50);

    assert_eq!(wheel.insert(run(1), past, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(2), past, PendingTimerKind::Ask), Ok(()));
    assert_eq!(wheel.len(), 2);

    let fired = wheel.fire_expired(Instant::now());
    assert_eq!(fired.len(), 2);

    // After fire, both runs should be gone from the wheel
    assert!(wheel.is_empty());
    assert!(wheel.get_entry(run(1)).is_none());
    assert!(wheel.get_entry(run(2)).is_none());
    assert!(wheel.get_kind(run(1)).is_none());
    assert!(wheel.get_kind(run(2)).is_none());
}

#[test]
fn fire_only_removes_expired_timers_not_future_ones() {
    let mut wheel = TimerWheel::new();
    let now = Instant::now();
    let past = now - Duration::from_millis(100);
    let future = now + Duration::from_secs(10);

    assert_eq!(wheel.insert(run(1), past, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(2), future, PendingTimerKind::Ask), Ok(()));
    assert_eq!(wheel.len(), 2);

    let fired = wheel.fire_expired(now);
    assert_eq!(fired.len(), 1);
    assert_eq!(fired[0].run, run(1));

    // Future timer still exists
    assert!(!wheel.is_empty());
    assert_eq!(wheel.len(), 1);
    assert!(wheel.get_entry(run(2)).is_some());
    assert!(wheel.get_entry(run(1)).is_none());
}

// ---------- Behavior J2: Queue full (cancel before fire) preserves others ----------

#[test]
fn cancel_before_fire_preserves_other_timers() {
    let mut wheel = TimerWheel::new();
    let past = Instant::now() - Duration::from_millis(50);

    assert_eq!(wheel.insert(run(1), past, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(2), past, PendingTimerKind::Ask), Ok(()));

    // Cancel one before fire — simulates capacity/authority gate
    assert!(wheel.cancel(run(1)));

    let fired = wheel.fire_expired(Instant::now());
    assert_eq!(fired.len(), 1);
    assert_eq!(fired[0].run, run(2));

    // Run 1 was already cancelled, run 2 is now fired
    assert!(wheel.is_empty());
}

// ---------- Behavior J3: Stale authority rejected before mutation ----------

#[test]
fn matches_authority_gate_prevents_fire_on_stale_generation() {
    // While TimerWheel itself doesn't have a stale-authority concept
    // (it fires anything at the deadline), the upper layer uses
    // PendingTimer::matches_authority as the gate. We test that gate here.

    let timer = PendingTimer {
        step: StepIdx::ZERO,
        kind: PendingTimerKind::Wait,
        generation: 5,
        deadline: Instant::now(),
    };

    // Stale generation (4 != 5) — should NOT match
    assert!(!timer.matches_authority(4, timer.deadline, PendingTimerKind::Wait));

    // Correct generation matches
    assert!(timer.matches_authority(5, timer.deadline, PendingTimerKind::Wait));
}

#[test]
fn replaced_timer_fires_only_latest_entry() {
    let mut wheel = TimerWheel::new();
    let past = Instant::now() - Duration::from_millis(200);
    let future = Instant::now() + Duration::from_millis(200);

    // Insert timer at past deadline
    assert_eq!(wheel.insert(run(1), past, PendingTimerKind::Wait), Ok(()));
    // Replace with future deadline (different kind)
    assert_eq!(wheel.insert(run(1), future, PendingTimerKind::Ask), Ok(()));

    // Fire at now — the past deadline should NOT fire because it was replaced
    let fired = wheel.fire_expired(Instant::now());
    assert_eq!(fired.len(), 0);
    assert_eq!(wheel.len(), 1);
    assert_eq!(
        wheel.get_entry(run(1)).expect("entry").kind,
        PendingTimerKind::Ask
    );
}

#[test]
fn canceled_timer_does_not_appear_in_fire_results() {
    let mut wheel = TimerWheel::new();
    let past = Instant::now() - Duration::from_millis(100);

    assert_eq!(wheel.insert(run(1), past, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(2), past, PendingTimerKind::Ask), Ok(()));
    assert_eq!(wheel.insert(run(3), past, PendingTimerKind::Wait), Ok(()));

    assert!(wheel.cancel(run(2))); // cancel middle one

    let fired = wheel.fire_expired(Instant::now());
    assert_eq!(fired.len(), 2);
    let runs: Vec<u64> = fired.iter().map(|e| e.run.get()).collect();
    assert!(runs.contains(&1));
    assert!(runs.contains(&3));
    assert!(!runs.contains(&2));
}

// ---------- Multiple successive fires ----------

#[test]
fn successive_fires_at_increasing_clock_times_work_correctly() {
    let mut wheel = TimerWheel::new();
    let base = Instant::now();
    let d1 = base + Duration::from_millis(10);
    let d2 = base + Duration::from_millis(20);
    let d3 = base + Duration::from_millis(30);

    assert_eq!(wheel.insert(run(1), d1, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(2), d2, PendingTimerKind::Ask), Ok(()));
    assert_eq!(wheel.insert(run(3), d3, PendingTimerKind::Wait), Ok(()));

    // Fire at d1 — only run(1) fires
    let f1 = wheel.fire_expired(d1);
    assert_eq!(f1.len(), 1);
    assert_eq!(f1[0].run, run(1));

    // Fire at d2 — run(2) fires
    let f2 = wheel.fire_expired(d2);
    assert_eq!(f2.len(), 1);
    assert_eq!(f2[0].run, run(2));

    // Fire at d3 — run(3) fires
    let f3 = wheel.fire_expired(d3);
    assert_eq!(f3.len(), 1);
    assert_eq!(f3[0].run, run(3));
    assert!(wheel.is_empty());
}
