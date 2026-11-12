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
//! E2E Tests: Full timer lifecycle scenarios.
//!
//! Tests the complete timer lifecycle across creation, fire, cancel,
//! replay-determinism, and cross-shard behavior via the `TimerWheel` API
//! and publicly-constructable `PendingTimer` value objects.

use std::time::{Duration, Instant};
use vb_core::ids::{RunId, StepIdx};
use vb_runtime::shard::timer_wheel::TimerWheel;
use vb_runtime::shard::types::{PendingTimer, PendingTimerKind};

fn run(id: u64) -> RunId {
    RunId::new(id)
}

// ---------- E2E 1: Full timer lifecycle ----------

#[test]
fn full_timer_lifecycle_insert_fire_cancel_reinsert() {
    let mut wheel = TimerWheel::new();

    // Phase 1: Insert timer
    let d1 = Instant::now() + Duration::from_millis(100);
    assert_eq!(wheel.insert(run(1), d1, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.len(), 1);
    assert_eq!(wheel.get_entry(run(1)).expect("entry").generation, 1);

    // Phase 2: Cancel before fire
    assert!(wheel.cancel(run(1)));
    assert!(wheel.is_empty());

    // Phase 3: Re-insert with new deadline
    let d2 = Instant::now() + Duration::from_millis(200);
    assert_eq!(wheel.insert(run(1), d2, PendingTimerKind::Ask), Ok(()));
    assert_eq!(wheel.get_entry(run(1)).expect("entry2").generation, 1);

    // Phase 4: Fire
    let fired = wheel.fire_expired(d2);
    assert_eq!(fired.len(), 1);
    assert_eq!(fired[0].run, run(1));
    assert_eq!(fired[0].kind, PendingTimerKind::Ask);
    assert!(wheel.is_empty());
}

// ---------- E2E 2: Timer with deadline overflow guard (generation) ----------

#[test]
fn timer_lifecycle_generation_tracks_correctly_through_cycle() {
    let mut wheel = TimerWheel::new();
    let base = Instant::now();

    // Create → fire → create → fire → create → fire (3 cycles)
    for cycle in 0..3u64 {
        let deadline = base + Duration::from_millis(cycle * 100);
        assert_eq!(
            wheel.insert(run(1), deadline, PendingTimerKind::Wait),
            Ok(())
        );

        let fired = wheel.fire_expired(deadline);
        assert_eq!(fired.len(), 1);
        assert_eq!(
            fired[0].generation, 1,
            "each new cycle starts generation at 1"
        );
    }
}

// ---------- E2E 3: Replay determinism ----------

#[test]
fn replay_determinism_identical_inserts_produce_identical_fire_results() {
    let deadline = Instant::now() - Duration::from_millis(500);

    // First run
    let mut w1 = TimerWheel::new();
    assert_eq!(w1.insert(run(1), deadline, PendingTimerKind::Wait), Ok(()));
    assert_eq!(w1.insert(run(2), deadline, PendingTimerKind::Ask), Ok(()));
    let f1 = w1.fire_expired(Instant::now());

    // Second run (replay)
    let mut w2 = TimerWheel::new();
    assert_eq!(w2.insert(run(1), deadline, PendingTimerKind::Wait), Ok(()));
    assert_eq!(w2.insert(run(2), deadline, PendingTimerKind::Ask), Ok(()));
    let f2 = w2.fire_expired(Instant::now());

    // Results must be identical
    assert_eq!(f1.len(), f2.len());
    for (a, b) in f1.iter().zip(f2.iter()) {
        assert_eq!(a.run, b.run);
        assert_eq!(a.generation, b.generation);
        assert_eq!(a.kind, b.kind);
    }
}

#[test]
fn replay_determinism_same_state_produces_consistent_len_before_and_after_fire() {
    for trial in 0..5 {
        let mut wheel = TimerWheel::new();
        let past = Instant::now() - Duration::from_millis(100);

        assert_eq!(wheel.insert(run(1), past, PendingTimerKind::Wait), Ok(()));
        assert_eq!(wheel.insert(run(2), past, PendingTimerKind::Ask), Ok(()));
        assert_eq!(wheel.insert(run(3), past, PendingTimerKind::Wait), Ok(()));

        let before_len = wheel.len();
        assert_eq!(before_len, 3, "trial {trial}");

        let fired = wheel.fire_expired(Instant::now());
        assert_eq!(fired.len(), 3, "trial {trial}");
        assert!(wheel.is_empty(), "trial {trial}");
    }
}

// ---------- E2E 4: Multiple independent runs with interleaved timers ----------

#[test]
fn multiple_runs_with_different_deadlines_independent_fire() {
    let mut wheel = TimerWheel::new();
    let base = Instant::now();

    // Run 1: fires at +10ms and +30ms
    // Run 2: fires at +20ms
    let d1 = base + Duration::from_millis(10);
    let d2 = base + Duration::from_millis(20);
    let d1b = base + Duration::from_millis(30);

    assert_eq!(wheel.insert(run(1), d1, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(2), d2, PendingTimerKind::Ask), Ok(()));

    // Fire at d1 — only run 1 fires
    let f1 = wheel.fire_expired(d1);
    assert_eq!(f1.len(), 1);
    assert_eq!(f1[0].run, run(1));

    // Re-insert run 1 at d1b
    assert_eq!(wheel.insert(run(1), d1b, PendingTimerKind::Wait), Ok(()));

    // Fire at d2 — run 2 fires
    let f2 = wheel.fire_expired(d2);
    assert_eq!(f2.len(), 1);
    assert_eq!(f2[0].run, run(2));

    // Fire at d1b — run 1 fires again
    let f3 = wheel.fire_expired(d1b);
    assert_eq!(f3.len(), 1);
    assert_eq!(f3[0].run, run(1));

    assert!(wheel.is_empty());
}

// ---------- E2E 5: Authority validation in complete cycle ----------

#[test]
fn authority_validation_prevents_mismatched_timer_from_firing() {
    let timer = PendingTimer {
        step: StepIdx::new(3),
        kind: PendingTimerKind::Wait,
        generation: 7,
        deadline: Instant::now(),
    };

    // Correct authority — should match
    assert!(timer.matches_authority(7, timer.deadline, PendingTimerKind::Wait));

    // Stale generation from earlier fire
    assert!(!timer.matches_authority(6, timer.deadline, PendingTimerKind::Wait));

    // Wrong kind (Ask vs Wait)
    assert!(!timer.matches_authority(7, timer.deadline, PendingTimerKind::Ask));

    // Wrong deadline
    let other_deadline = timer.deadline + Duration::from_secs(1);
    assert!(!timer.matches_authority(7, other_deadline, PendingTimerKind::Wait));
}

// ---------- E2E: Stress — many timers, many fires, many cancels ----------

#[test]
fn stress_many_timers_interleaved_with_cancels_and_fires() {
    let mut wheel = TimerWheel::new();
    let base = Instant::now();

    // Insert 50 timers
    for i in 0..50u64 {
        let deadline = base + Duration::from_millis(i * 10);
        assert_eq!(
            wheel.insert(run(i), deadline, PendingTimerKind::Wait),
            Ok(())
        );
    }
    assert_eq!(wheel.len(), 50);

    // Cancel every 3rd timer
    for i in (0..50u64).step_by(3) {
        wheel.cancel(run(i));
    }

    // Fire all past deadlines
    let far_future = base + Duration::from_secs(10);
    let fired = wheel.fire_expired(far_future);

    let expected = 50 - ((50 + 2) / 3); // 50 total - ~17 cancelled
    assert_eq!(fired.len(), expected);
    assert!(wheel.is_empty());
}
