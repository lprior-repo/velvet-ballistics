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
//! Behavior tests for `TimerWheel` — wait/ask deadline tracking.
//!
//! Covers: creation, expiration, cancellation, overflow, ordering,
//! repeating patterns, zero-duration, capacity, precision.

use std::time::{Duration, Instant};

use vb_runtime::shard::RunId;
use vb_runtime::shard::timer_wheel::TimerWheel;
use vb_runtime::shard::types::PendingTimerKind;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn run(id: u64) -> RunId {
    RunId::new(id)
}

fn deadline_at(ms_from_now: i64) -> Instant {
    if ms_from_now >= 0 {
        Instant::now() + Duration::from_millis(ms_from_now as u64)
    } else {
        Instant::now() - Duration::from_millis((-ms_from_now) as u64)
    }
}

// ---------------------------------------------------------------------------
// 1. Timer creation with various durations
// ---------------------------------------------------------------------------

#[test]
fn insert_succeeds_with_millisecond_duration() {
    let mut wheel = TimerWheel::new();
    let deadline = deadline_at(1);
    let result = wheel.insert(run(1), deadline, PendingTimerKind::Wait);
    assert_eq!(result, Ok(()));
    assert_eq!(wheel.len(), 1);
}

#[test]
fn insert_succeeds_with_second_duration() {
    let mut wheel = TimerWheel::new();
    let deadline = deadline_at(1000);
    let result = wheel.insert(run(1), deadline, PendingTimerKind::Wait);
    assert_eq!(result, Ok(()));
    assert_eq!(wheel.len(), 1);
}

#[test]
fn insert_succeeds_with_minute_duration() {
    let mut wheel = TimerWheel::new();
    let deadline = deadline_at(60_000);
    let result = wheel.insert(run(1), deadline, PendingTimerKind::Ask);
    assert_eq!(result, Ok(()));
    assert_eq!(wheel.len(), 1);
}

#[test]
fn insert_succeeds_with_hour_duration() {
    let mut wheel = TimerWheel::new();
    let deadline = deadline_at(3_600_000);
    let result = wheel.insert(run(1), deadline, PendingTimerKind::Wait);
    assert_eq!(result, Ok(()));
    assert_eq!(wheel.len(), 1);
}

#[test]
fn insert_succeeds_with_large_duration() {
    let mut wheel = TimerWheel::new();
    let deadline = deadline_at(86_400_000);
    let result = wheel.insert(run(1), deadline, PendingTimerKind::Wait);
    assert_eq!(result, Ok(()));
}

#[test]
fn insert_succeeds_with_ask_kind() {
    let mut wheel = TimerWheel::new();
    let deadline = deadline_at(100);
    let result = wheel.insert(run(1), deadline, PendingTimerKind::Ask);
    assert_eq!(result, Ok(()));
    assert_eq!(wheel.get_kind(run(1)), Some(PendingTimerKind::Ask));
}

#[test]
fn insert_succeeds_with_wait_kind() {
    let mut wheel = TimerWheel::new();
    let deadline = deadline_at(100);
    let result = wheel.insert(run(1), deadline, PendingTimerKind::Wait);
    assert_eq!(result, Ok(()));
    assert_eq!(wheel.get_kind(run(1)), Some(PendingTimerKind::Wait));
}

#[test]
fn insert_replaces_existing_timer_and_updates_deadline() {
    let mut wheel = TimerWheel::new();
    let early = deadline_at(100);
    let later = deadline_at(500);
    assert_eq!(wheel.insert(run(1), early, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(1), later, PendingTimerKind::Ask), Ok(()));
    assert_eq!(wheel.len(), 1);
    assert_eq!(wheel.next_deadline(), Some(later));
}

// ---------------------------------------------------------------------------
// 2. Timer expiration and callback
// ---------------------------------------------------------------------------

#[test]
fn fire_expired_returns_timers_in_deadline_order() {
    let mut wheel = TimerWheel::new();
    let now = Instant::now();
    let d1 = now - Duration::from_millis(300);
    let d2 = now - Duration::from_millis(200);
    let d3 = now - Duration::from_millis(100);

    assert_eq!(wheel.insert(run(1), d1, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(2), d3, PendingTimerKind::Ask), Ok(()));
    assert_eq!(wheel.insert(run(3), d2, PendingTimerKind::Wait), Ok(()));

    let fired = wheel.fire_expired(now);
    assert_eq!(fired.len(), 3);
    assert_eq!(fired[0].deadline, d1);
    assert_eq!(fired[1].deadline, d2);
    assert_eq!(fired[2].deadline, d3);
}

#[test]
fn fire_expired_excludes_future_timers() {
    let mut wheel = TimerWheel::new();
    let now = Instant::now();
    let past = now - Duration::from_millis(50);
    let future = now + Duration::from_secs(10);

    assert_eq!(wheel.insert(run(1), past, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(2), future, PendingTimerKind::Ask), Ok(()));

    let fired = wheel.fire_expired(now);
    assert_eq!(fired.len(), 1);
    assert_eq!(fired[0].run, run(1));
    assert!(!wheel.is_empty());
}

#[test]
fn fire_expired_with_empty_wheel_returns_empty_vec() {
    let mut wheel = TimerWheel::new();
    let fired = wheel.fire_expired(Instant::now());
    assert!(fired.is_empty());
}

#[test]
fn fire_expired_clears_by_run_index() {
    let mut wheel = TimerWheel::new();
    let past = Instant::now() - Duration::from_millis(10);
    assert_eq!(wheel.insert(run(1), past, PendingTimerKind::Wait), Ok(()));
    let _fired = wheel.fire_expired(Instant::now());
    assert!(wheel.get_entry(run(1)).is_none());
    assert!(wheel.get_kind(run(1)).is_none());
}

// ---------------------------------------------------------------------------
// 3. Timer cancellation
// ---------------------------------------------------------------------------

#[test]
fn cancel_removes_active_timer_and_returns_true() {
    let mut wheel = TimerWheel::new();
    let deadline = deadline_at(500);
    assert_eq!(
        wheel.insert(run(1), deadline, PendingTimerKind::Wait),
        Ok(())
    );
    let removed = wheel.cancel(run(1));
    assert!(removed);
    assert!(wheel.is_empty());
}

#[test]
fn cancel_returns_false_for_nonexistent_run() {
    let mut wheel = TimerWheel::new();
    assert!(!wheel.cancel(run(99)));
}

#[test]
fn cancel_returns_false_after_previous_cancellation() {
    let mut wheel = TimerWheel::new();
    let deadline = deadline_at(500);
    assert_eq!(
        wheel.insert(run(1), deadline, PendingTimerKind::Wait),
        Ok(())
    );
    assert!(wheel.cancel(run(1)));
    assert!(!wheel.cancel(run(1)));
}

#[test]
fn cancel_returns_false_after_expiration_fire() {
    let mut wheel = TimerWheel::new();
    let past = Instant::now() - Duration::from_millis(50);
    assert_eq!(wheel.insert(run(1), past, PendingTimerKind::Wait), Ok(()));
    let _fired = wheel.fire_expired(Instant::now());
    assert!(!wheel.cancel(run(1)));
}

#[test]
fn cancelled_timer_not_included_in_fire_expired() {
    let mut wheel = TimerWheel::new();
    let now = Instant::now();
    let past = now - Duration::from_millis(100);
    assert_eq!(wheel.insert(run(1), past, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(2), past, PendingTimerKind::Ask), Ok(()));
    assert!(wheel.cancel(run(1)));

    let fired = wheel.fire_expired(now);
    assert_eq!(fired.len(), 1);
    assert_eq!(fired[0].run, run(2));
}

#[test]
fn insert_after_cancel_succeeds_and_starts_generation_at_one() {
    let mut wheel = TimerWheel::new();
    let d1 = deadline_at(100);
    assert_eq!(wheel.insert(run(1), d1, PendingTimerKind::Wait), Ok(()));
    assert!(wheel.cancel(run(1)));

    let d2 = deadline_at(200);
    assert_eq!(wheel.insert(run(1), d2, PendingTimerKind::Ask), Ok(()));
    let entry = wheel
        .get_entry(run(1))
        .expect("entry should exist after re-insert");
    assert_eq!(entry.generation, 1);
}

// ---------------------------------------------------------------------------
// 4. Cancel all
// ---------------------------------------------------------------------------

#[test]
fn cancel_all_timers_leaves_wheel_empty() {
    let mut wheel = TimerWheel::new();
    let now = Instant::now();
    for i in 0..10u64 {
        let deadline = now + Duration::from_millis(i * 10);
        assert_eq!(
            wheel.insert(run(i), deadline, PendingTimerKind::Wait),
            Ok(())
        );
    }
    for i in 0..10u64 {
        wheel.cancel(run(i));
    }
    assert!(wheel.is_empty());
    assert_eq!(wheel.len(), 0);
    assert!(wheel.next_deadline().is_none());
}

// ---------------------------------------------------------------------------
// 5. Zero-duration timers
// ---------------------------------------------------------------------------

#[test]
fn zero_duration_timer_fires_at_exact_now() {
    let mut wheel = TimerWheel::new();
    let now = Instant::now();
    assert_eq!(wheel.insert(run(1), now, PendingTimerKind::Wait), Ok(()));
    let fired = wheel.fire_expired(now);
    assert_eq!(fired.len(), 1);
    assert_eq!(fired[0].run, run(1));
}

#[test]
fn past_deadline_timer_fires_immediately() {
    let mut wheel = TimerWheel::new();
    let past = deadline_at(-1000);
    assert_eq!(wheel.insert(run(1), past, PendingTimerKind::Wait), Ok(()));
    let fired = wheel.fire_expired(Instant::now());
    assert_eq!(fired.len(), 1);
    assert_eq!(fired[0].run, run(1));
}

#[test]
fn exact_deadline_boundary_is_inclusive() {
    let mut wheel = TimerWheel::new();
    let deadline = Instant::now();
    assert_eq!(
        wheel.insert(run(1), deadline, PendingTimerKind::Ask),
        Ok(())
    );
    let fired = wheel.fire_expired(deadline);
    assert_eq!(fired.len(), 1);
    assert_eq!(fired[0].kind, PendingTimerKind::Ask);
}

// ---------------------------------------------------------------------------
// 6. Timer ordering (earliest first)
// ---------------------------------------------------------------------------

#[test]
fn next_deadline_returns_earliest_after_mixed_inserts() {
    let mut wheel = TimerWheel::new();
    let now = Instant::now();
    let d1 = now + Duration::from_millis(300);
    let d2 = now + Duration::from_millis(10);
    let d3 = now + Duration::from_millis(200);

    assert_eq!(wheel.insert(run(1), d1, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(2), d3, PendingTimerKind::Ask), Ok(()));
    assert_eq!(wheel.insert(run(3), d2, PendingTimerKind::Wait), Ok(()));

    assert_eq!(wheel.next_deadline(), Some(d2));
}

#[test]
fn next_deadline_updates_after_earliest_is_cancelled() {
    let mut wheel = TimerWheel::new();
    let now = Instant::now();
    let d1 = now + Duration::from_millis(10);
    let d2 = now + Duration::from_millis(100);

    assert_eq!(wheel.insert(run(1), d1, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(2), d2, PendingTimerKind::Ask), Ok(()));
    assert_eq!(wheel.next_deadline(), Some(d1));

    wheel.cancel(run(1));
    assert_eq!(wheel.next_deadline(), Some(d2));
}

#[test]
fn next_deadline_none_when_all_timers_cancelled() {
    let mut wheel = TimerWheel::new();
    let deadline = deadline_at(100);

    assert_eq!(
        wheel.insert(run(1), deadline, PendingTimerKind::Wait),
        Ok(())
    );
    assert_eq!(
        wheel.insert(run(2), deadline, PendingTimerKind::Ask),
        Ok(())
    );
    assert_eq!(wheel.next_deadline(), Some(deadline));

    wheel.cancel(run(1));
    wheel.cancel(run(2));
    assert!(wheel.next_deadline().is_none());
}

#[test]
fn insert_out_of_deadline_order_still_sorts_correctly() {
    let mut wheel = TimerWheel::new();
    let now = Instant::now();

    let d4 = now + Duration::from_millis(400);
    let d1 = now + Duration::from_millis(100);
    let d3 = now + Duration::from_millis(300);
    let d2 = now + Duration::from_millis(200);

    assert_eq!(wheel.insert(run(4), d4, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(1), d1, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(3), d3, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(2), d2, PendingTimerKind::Wait), Ok(()));

    let fired = wheel.fire_expired(d4);
    assert_eq!(fired.len(), 4);
    assert_eq!(fired[0].deadline, d1);
    assert_eq!(fired[1].deadline, d2);
    assert_eq!(fired[2].deadline, d3);
    assert_eq!(fired[3].deadline, d4);
}

// ---------------------------------------------------------------------------
// 7. Repeating timers
// ---------------------------------------------------------------------------

#[test]
fn repeating_timer_pattern_cancel_and_reinsert_three_cycles() {
    let mut wheel = TimerWheel::new();
    for cycle in 0u64..3 {
        let run_id = run(1);
        let deadline = deadline_at(100);
        assert_eq!(
            wheel.insert(run_id, deadline, PendingTimerKind::Wait),
            Ok(()),
            "cycle {} insert failed",
            cycle
        );
        assert_eq!(wheel.len(), 1);
        assert!(wheel.cancel(run_id));
        assert!(wheel.is_empty());
    }
}

#[test]
fn repeating_timer_pattern_with_increasing_deadlines() {
    let mut wheel = TimerWheel::new();
    for cycle in 1u64..=5 {
        let deadline = deadline_at(cycle as i64 * 100);
        assert_eq!(
            wheel.insert(run(1), deadline, PendingTimerKind::Wait),
            Ok(())
        );
        assert_eq!(wheel.next_deadline(), Some(deadline));
        let fired = wheel.fire_expired(deadline);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].run, run(1));
        assert!(wheel.is_empty());
    }
}

#[test]
fn two_runs_repeating_independently_with_different_periods() {
    let mut wheel = TimerWheel::new();
    let base = Instant::now();

    let d1 = base + Duration::from_millis(10);
    assert_eq!(wheel.insert(run(1), d1, PendingTimerKind::Wait), Ok(()));
    let d2 = base + Duration::from_millis(50);
    assert_eq!(wheel.insert(run(2), d2, PendingTimerKind::Ask), Ok(()));

    let fired = wheel.fire_expired(d1);
    assert_eq!(fired.len(), 1);
    assert_eq!(fired[0].run, run(1));

    let d1_again = d1 + Duration::from_millis(10);
    assert_eq!(
        wheel.insert(run(1), d1_again, PendingTimerKind::Wait),
        Ok(())
    );

    let fired2 = wheel.fire_expired(d2);
    assert_eq!(fired2.len(), 2);
}

// ---------------------------------------------------------------------------
// 8. Generation overflow
// ---------------------------------------------------------------------------

// generation_overflow is verified in the inline #[cfg(test)] module
// in crates/vb_runtime/src/shard/timer_wheel.rs via
// replacement_generation_overflow_fails_closed

#[test]
fn generation_increments_on_replacement() {
    let mut wheel = TimerWheel::new();
    let d1 = deadline_at(100);
    assert_eq!(wheel.insert(run(1), d1, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.get_entry(run(1)).expect("first insert").generation, 1);

    let d2 = deadline_at(200);
    assert_eq!(wheel.insert(run(1), d2, PendingTimerKind::Ask), Ok(()));
    assert_eq!(
        wheel.get_entry(run(1)).expect("second insert").generation,
        2
    );

    let d3 = deadline_at(300);
    assert_eq!(wheel.insert(run(1), d3, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.get_entry(run(1)).expect("third insert").generation, 3);
}

// ---------------------------------------------------------------------------
// 9. Maximum timers capacity
// ---------------------------------------------------------------------------

#[test]
fn capacity_thousand_timers_all_fire_correctly() {
    let mut wheel = TimerWheel::new();
    let count = 1000u64;
    let base = Instant::now();
    for i in 0..count {
        let deadline = base + Duration::from_millis(i);
        assert_eq!(
            wheel.insert(run(i), deadline, PendingTimerKind::Wait),
            Ok(())
        );
    }
    assert_eq!(wheel.len(), count as usize);

    let far_future = base + Duration::from_millis(count + 100);
    let fired = wheel.fire_expired(far_future);
    assert_eq!(fired.len(), count as usize);
    assert!(wheel.is_empty());
}

#[test]
fn capacity_five_thousand_timers_insert_and_drain() {
    let mut wheel = TimerWheel::new();
    let count = 5000u64;
    let base = Instant::now();
    for i in 0..count {
        let deadline = base + Duration::from_micros(i);
        assert_eq!(
            wheel.insert(run(i), deadline, PendingTimerKind::Wait),
            Ok(())
        );
    }
    assert_eq!(wheel.len(), count as usize);

    let far_future = base + Duration::from_secs(10);
    let fired = wheel.fire_expired(far_future);
    assert_eq!(fired.len(), count as usize);
    assert!(wheel.is_empty());
}

#[test]
fn capacity_fifty_timers_same_deadline_all_fire_correctly() {
    let mut wheel = TimerWheel::new();
    let deadline = Instant::now();
    for i in 0..50u64 {
        assert_eq!(
            wheel.insert(run(i), deadline, PendingTimerKind::Wait),
            Ok(())
        );
    }
    let fired = wheel.fire_expired(deadline);
    assert_eq!(fired.len(), 50);
    let mut runs_seen: Vec<u64> = fired.iter().map(|e| e.run.get()).collect();
    runs_seen.sort();
    assert_eq!(runs_seen, (0u64..50).collect::<Vec<_>>());
}

#[test]
fn capacity_insert_many_timers_then_cancel_every_second_one() {
    let mut wheel = TimerWheel::new();
    let count = 200u64;
    let base = Instant::now();
    for i in 0..count {
        let deadline = base + Duration::from_millis(i);
        assert_eq!(
            wheel.insert(run(i), deadline, PendingTimerKind::Wait),
            Ok(())
        );
    }

    for i in (0..count).step_by(2) {
        assert!(wheel.cancel(run(i)));
    }

    let expected_remaining = (count / 2) as usize;
    assert_eq!(wheel.len(), expected_remaining);

    let far_future = base + Duration::from_secs(10);
    let fired = wheel.fire_expired(far_future);
    assert_eq!(fired.len(), expected_remaining);
    assert!(wheel.is_empty());
}

// ---------------------------------------------------------------------------
// 10. Timer precision
// ---------------------------------------------------------------------------

#[test]
fn precision_nanosecond_scale_deadlines_are_handled() {
    let mut wheel = TimerWheel::new();
    let base = Instant::now();
    let d1 = base + Duration::from_nanos(100);
    let d2 = base + Duration::from_nanos(250);
    let d3 = base + Duration::from_nanos(500);

    assert_eq!(wheel.insert(run(1), d1, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(2), d2, PendingTimerKind::Ask), Ok(()));
    assert_eq!(wheel.insert(run(3), d3, PendingTimerKind::Wait), Ok(()));

    let fired = wheel.fire_expired(d3);
    assert_eq!(fired.len(), 3);
    assert_eq!(fired[0].deadline, d1);
    assert_eq!(fired[1].deadline, d2);
    assert_eq!(fired[2].deadline, d3);
}

#[test]
fn precision_microsecond_scale_deadlines_maintain_ordering() {
    let mut wheel = TimerWheel::new();
    let base = Instant::now();
    let d1 = base + Duration::from_micros(1);
    let d2 = base + Duration::from_micros(2);
    let d3 = base + Duration::from_micros(3);

    assert_eq!(wheel.insert(run(3), d3, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(1), d1, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(2), d2, PendingTimerKind::Wait), Ok(()));

    let fired = wheel.fire_expired(d3);
    assert_eq!(fired[0].deadline, d1);
    assert_eq!(fired[1].deadline, d2);
    assert_eq!(fired[2].deadline, d3);
}

#[test]
fn precision_deadline_one_nanosecond_apart_are_distinct() {
    let mut wheel = TimerWheel::new();
    let base = Instant::now();
    let d1 = base + Duration::from_nanos(1);
    let d2 = base + Duration::from_nanos(2);

    assert_eq!(wheel.insert(run(1), d2, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(2), d1, PendingTimerKind::Ask), Ok(()));

    assert_eq!(wheel.next_deadline(), Some(d1));
}

#[test]
fn precision_identical_timestamps_returned_in_any_order() {
    let mut wheel = TimerWheel::new();
    let deadline = Instant::now();
    assert_eq!(
        wheel.insert(run(10), deadline, PendingTimerKind::Wait),
        Ok(())
    );
    assert_eq!(
        wheel.insert(run(20), deadline, PendingTimerKind::Ask),
        Ok(())
    );
    assert_eq!(
        wheel.insert(run(30), deadline, PendingTimerKind::Wait),
        Ok(())
    );

    let fired = wheel.fire_expired(deadline);
    assert_eq!(fired.len(), 3);
    let runs: Vec<u64> = fired.iter().map(|e| e.run.get()).collect();
    assert!(runs.contains(&10));
    assert!(runs.contains(&20));
    assert!(runs.contains(&30));
}

// ---------------------------------------------------------------------------
// 11. Miscellaneous / state queries
// ---------------------------------------------------------------------------

#[test]
fn default_constructor_creates_empty_wheel() {
    let wheel = TimerWheel::default();
    assert!(wheel.is_empty());
    assert_eq!(wheel.len(), 0);
    assert!(wheel.next_deadline().is_none());
}

#[test]
fn get_entry_returns_correct_timer_metadata() {
    let mut wheel = TimerWheel::new();
    let deadline = deadline_at(500);

    assert_eq!(
        wheel.insert(run(1), deadline, PendingTimerKind::Ask),
        Ok(())
    );

    let entry = wheel.get_entry(run(1)).expect("entry should exist");
    assert_eq!(entry.run, run(1));
    assert_eq!(entry.deadline, deadline);
    assert_eq!(entry.kind, PendingTimerKind::Ask);
    assert_eq!(entry.generation, 1);
}

#[test]
fn get_entry_returns_none_for_unknown_run() {
    let wheel = TimerWheel::new();
    assert!(wheel.get_entry(run(42)).is_none());
}

#[test]
fn is_empty_returns_false_after_insert() {
    let mut wheel = TimerWheel::new();
    assert!(wheel.is_empty());
    assert_eq!(
        wheel.insert(run(1), deadline_at(100), PendingTimerKind::Wait),
        Ok(())
    );
    assert!(!wheel.is_empty());
}

#[test]
fn len_accurately_reflects_current_state() {
    let mut wheel = TimerWheel::new();
    assert_eq!(wheel.len(), 0);

    for i in 0u64..5 {
        assert_eq!(
            wheel.insert(
                run(i),
                deadline_at((i + 1) as i64 * 100),
                PendingTimerKind::Wait
            ),
            Ok(())
        );
        assert_eq!(wheel.len(), (i + 1) as usize);
    }

    for i in 0u64..5 {
        wheel.cancel(run(i));
        assert_eq!(wheel.len(), (4 - i) as usize);
    }
}

#[test]
fn fire_expired_after_inserting_then_replacing_same_run_returns_only_final_entry() {
    let mut wheel = TimerWheel::new();
    let past = Instant::now() - Duration::from_millis(200);
    let future = Instant::now() + Duration::from_millis(200);

    assert_eq!(wheel.insert(run(1), past, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(1), future, PendingTimerKind::Ask), Ok(()));

    let fired = wheel.fire_expired(Instant::now());
    assert_eq!(fired.len(), 0);
    assert_eq!(wheel.len(), 1);
    assert_eq!(
        wheel.get_entry(run(1)).expect("replaced entry").kind,
        PendingTimerKind::Ask
    );
}
