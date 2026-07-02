#![forbid(unsafe_code)]
//! Counter tests for the seeded autonomous scheduler facade.
//!
//! `step_count` and `decision_count` are the two monotonic counters
//! the scheduler exposes. `step_count` advances once per
//! `tick_shard` / `tick_all` call (after the runtime accepts the
//! directive); `decision_count` advances once per
//! `decide_boundary` call. The tests below pin both invariants and
//! prove they are not aliased.

use crate::scheduler::tests::fixtures::{FIXTURE_SEED_A, make_scheduler};
use crate::scheduler::types::{BoundaryChoice, BoundaryPolicy};

#[test]
fn scheduler_step_counter_increments_per_tick() {
    let mut scheduler = make_scheduler(FIXTURE_SEED_A, BoundaryPolicy::First);
    assert_eq!(scheduler.step_count(), 0);
    let _ = scheduler.tick_shard(0, BoundaryChoice::Free);
    assert_eq!(scheduler.step_count(), 1);
    let _ = scheduler.tick_all();
    assert_eq!(scheduler.step_count(), 2);
}

#[test]
fn scheduler_decision_counter_increments_per_decide() {
    let mut scheduler = make_scheduler(FIXTURE_SEED_A, BoundaryPolicy::First);
    assert_eq!(scheduler.decision_count(), 0);
    let _ = scheduler.decide_boundary(BoundaryChoice::Free);
    assert_eq!(scheduler.decision_count(), 1);
    let _ = scheduler.decide_boundary(BoundaryChoice::Free);
    assert_eq!(scheduler.decision_count(), 2);
}

#[test]
fn step_and_decision_counters_diverge_after_decide_without_tick() {
    // `decide_boundary` does not bump `step_count` (only
    // `tick_shard` / `tick_all` do); calling it twice leaves
    // `step_count` at 0 while `decision_count` reaches 2.
    let mut scheduler = make_scheduler(FIXTURE_SEED_A, BoundaryPolicy::First);
    let _ = scheduler.decide_boundary(BoundaryChoice::Free);
    let _ = scheduler.decide_boundary(BoundaryChoice::Free);
    assert_eq!(scheduler.step_count(), 0);
    assert_eq!(scheduler.decision_count(), 2);
}

#[test]
fn decision_count_advances_inside_tick_shard() {
    // `tick_shard` calls `decide_boundary` internally, so both
    // counters should advance per call (one each).
    let mut scheduler = make_scheduler(FIXTURE_SEED_A, BoundaryPolicy::First);
    let _ = scheduler.tick_shard(0, BoundaryChoice::Free);
    assert_eq!(scheduler.step_count(), 1);
    assert_eq!(scheduler.decision_count(), 1);
    let _ = scheduler.tick_shard(0, BoundaryChoice::Free);
    assert_eq!(scheduler.step_count(), 2);
    assert_eq!(scheduler.decision_count(), 2);
}
