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
//! PS-005: Duplicate Delayed-Action Key — behavior tests (E1-E3).
//!
//! Tests `TimerWheel` behavior for duplicate key insertion:
//! - Identical duplicate replaces (new generation) preserving one entry
//! - Same-run re-insert after cancel starts fresh
//! - Different-run inserts do not collide

use std::time::{Duration, Instant};
use vb_core::ids::RunId;
use vb_runtime::shard::timer_wheel::TimerWheel;
use vb_runtime::shard::types::PendingTimerKind;

fn run(id: u64) -> RunId {
    RunId::new(id)
}

// ---------- Behavior E1: Identical duplicate (same run, same kind) replaces ----------

#[test]
fn identical_duplicate_replaces_existing_timer_with_new_deadline() {
    let mut wheel = TimerWheel::new();
    let early = Instant::now() + Duration::from_millis(100);
    let later = Instant::now() + Duration::from_millis(500);

    assert_eq!(wheel.insert(run(1), early, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(1), later, PendingTimerKind::Wait), Ok(()));

    // Only one entry exists after replacement
    assert_eq!(wheel.len(), 1);
    let entry = wheel.get_entry(run(1)).expect("entry should exist");
    assert_eq!(entry.deadline, later);
    assert_eq!(entry.kind, PendingTimerKind::Wait);
}

#[test]
fn identical_duplicate_increments_generation() {
    let mut wheel = TimerWheel::new();
    let d1 = Instant::now();
    let d2 = d1 + Duration::from_millis(100);

    assert_eq!(wheel.insert(run(1), d1, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.get_entry(run(1)).expect("1st").generation, 1);

    assert_eq!(wheel.insert(run(1), d2, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.get_entry(run(1)).expect("2nd").generation, 2);
}

#[test]
fn identical_duplicate_preserves_single_entry_with_latest_metadata() {
    let mut wheel = TimerWheel::new();
    let d1 = Instant::now() + Duration::from_millis(200);
    let d2 = Instant::now() + Duration::from_millis(400);

    // Insert as Wait
    assert_eq!(wheel.insert(run(1), d1, PendingTimerKind::Wait), Ok(()));
    // Replace with same run different kind — kind is part of the identity
    assert_eq!(wheel.insert(run(1), d2, PendingTimerKind::Ask), Ok(()));

    let entry = wheel.get_entry(run(1)).expect("entry should exist");
    assert_eq!(entry.kind, PendingTimerKind::Ask);
    assert_eq!(entry.deadline, d2);
    assert_eq!(entry.generation, 2);
    assert_eq!(wheel.len(), 1);
}

// ---------- Behavior E3: New key creates fresh entry ----------

#[test]
fn new_run_creates_entry_without_affecting_existing_timers() {
    let mut wheel = TimerWheel::new();
    let now = Instant::now();

    assert_eq!(wheel.insert(run(1), now, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.len(), 1);

    // Insert a different run
    assert_eq!(wheel.insert(run(2), now, PendingTimerKind::Ask), Ok(()));
    assert_eq!(wheel.len(), 2);

    // Both entries exist with correct properties
    assert_eq!(
        wheel.get_entry(run(1)).expect("run1").kind,
        PendingTimerKind::Wait
    );
    assert_eq!(
        wheel.get_entry(run(2)).expect("run2").kind,
        PendingTimerKind::Ask
    );
}

#[test]
fn new_run_entry_gets_generation_one() {
    let mut wheel = TimerWheel::new();
    assert_eq!(
        wheel.insert(run(10), Instant::now(), PendingTimerKind::Wait),
        Ok(())
    );
    assert_eq!(wheel.get_entry(run(10)).expect("entry").generation, 1);
}

// ---------- Same run, different kind (divergent duplicate equivalent) ----------

#[test]
fn same_run_different_kind_replaces_and_changes_kind() {
    let mut wheel = TimerWheel::new();
    let deadline = Instant::now();

    assert_eq!(
        wheel.insert(run(1), deadline, PendingTimerKind::Wait),
        Ok(())
    );
    assert_eq!(wheel.get_kind(run(1)), Some(PendingTimerKind::Wait));

    // Replace with same run but Ask kind
    assert_eq!(
        wheel.insert(run(1), deadline, PendingTimerKind::Ask),
        Ok(())
    );
    assert_eq!(wheel.get_kind(run(1)), Some(PendingTimerKind::Ask));

    // Only one entry exists
    assert_eq!(wheel.len(), 1);
}

#[test]
fn same_run_different_deadline_replaces_while_preserving_count() {
    let mut wheel = TimerWheel::new();
    let d1 = Instant::now() + Duration::from_millis(100);
    let d2 = Instant::now() + Duration::from_millis(999);

    assert_eq!(wheel.insert(run(1), d1, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.get_entry(run(1)).expect("1st").deadline, d1);

    assert_eq!(wheel.insert(run(1), d2, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.get_entry(run(1)).expect("2nd").deadline, d2);
    assert_eq!(wheel.len(), 1);
}

// ---------- Cancel then re-insert ----------

#[test]
fn cancel_then_reinsert_for_same_run_starts_fresh() {
    let mut wheel = TimerWheel::new();
    let deadline = Instant::now();

    assert_eq!(
        wheel.insert(run(1), deadline, PendingTimerKind::Wait),
        Ok(())
    );
    assert!(wheel.cancel(run(1)));

    // Re-inserting the same run starts generation at 1
    assert_eq!(
        wheel.insert(run(1), deadline, PendingTimerKind::Ask),
        Ok(())
    );
    assert_eq!(wheel.get_entry(run(1)).expect("re-inserted").generation, 1);
    assert_eq!(wheel.len(), 1);
}
