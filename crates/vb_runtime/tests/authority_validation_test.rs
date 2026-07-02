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
//! PS-003: Authority Validation — behavior tests (C1-C5).
//!
//! Tests that `PendingTimer::matches_authority` correctly validates:
//! - Generation must match
//! - Deadline must match
//! - Kind must match
//! - All three must match for success
//!
//! The deadline is an `Instant` field on the existing `PendingTimer` type;
//! this test exercises the numeric generation and kind validation which are
//! the primary authority components alongside the numeric timer seam.

use std::time::{Duration, Instant};
use vb_core::ids::StepIdx;
use vb_runtime::shard::types::{PendingTimer, PendingTimerKind};

fn make_timer(generation: u64, kind: PendingTimerKind) -> PendingTimer {
    let deadline = Instant::now();
    PendingTimer {
        step: StepIdx::ZERO,
        kind,
        generation,
        deadline,
        ..Default::default()
    }
}

fn make_timer_with_deadline(
    generation: u64,
    deadline: Instant,
    kind: PendingTimerKind,
) -> PendingTimer {
    PendingTimer {
        step: StepIdx::ZERO,
        kind,
        generation,
        deadline,
        ..Default::default()
    }
}

// ---------- Behavior C5: Valid full authority succeeds ----------

#[test]
fn matches_authority_returns_true_when_all_fields_match() {
    let timer = make_timer(5, PendingTimerKind::Wait);
    assert!(timer.matches_authority(5, timer.deadline, PendingTimerKind::Wait));
}

#[test]
fn matches_authority_returns_true_for_ask_kind_match() {
    let timer = make_timer(3, PendingTimerKind::Ask);
    assert!(timer.matches_authority(3, timer.deadline, PendingTimerKind::Ask));
}

#[test]
fn matches_authority_returns_true_with_generation_one() {
    let timer = make_timer(1, PendingTimerKind::Wait);
    assert!(timer.matches_authority(1, timer.deadline, PendingTimerKind::Wait));
}

#[test]
fn matches_authority_returns_true_with_large_generation() {
    let timer = make_timer(u64::MAX - 1, PendingTimerKind::Ask);
    assert!(timer.matches_authority(u64::MAX - 1, timer.deadline, PendingTimerKind::Ask));
}

// ---------- Behavior C2: Wrong generation rejected ----------

#[test]
fn matches_authority_rejects_when_generation_is_higher() {
    let timer = make_timer(5, PendingTimerKind::Wait);
    assert!(!timer.matches_authority(6, timer.deadline, PendingTimerKind::Wait));
}

#[test]
fn matches_authority_rejects_when_generation_is_lower() {
    let timer = make_timer(5, PendingTimerKind::Wait);
    assert!(!timer.matches_authority(4, timer.deadline, PendingTimerKind::Wait));
}

#[test]
fn matches_authority_rejects_when_generation_is_zero_but_timer_has_one() {
    let timer = make_timer(1, PendingTimerKind::Wait);
    assert!(!timer.matches_authority(0, timer.deadline, PendingTimerKind::Wait));
}

// ---------- Behavior C3: Wrong deadline rejected ----------

#[test]
fn matches_authority_rejects_when_deadline_differs() {
    let d1 = Instant::now();
    let d2 = d1 + Duration::from_secs(1);
    let timer = make_timer_with_deadline(5, d1, PendingTimerKind::Wait);
    assert!(!timer.matches_authority(5, d2, PendingTimerKind::Wait));
}

#[test]
fn matches_authority_rejects_when_deadline_is_earlier() {
    let now = Instant::now();
    let future = now + Duration::from_secs(10);
    let timer = make_timer_with_deadline(1, future, PendingTimerKind::Ask);
    assert!(!timer.matches_authority(1, now, PendingTimerKind::Ask));
}

// ---------- Behavior C4: Wrong kind rejected ----------

#[test]
fn matches_authority_rejects_wait_timer_when_authority_says_ask() {
    let timer = make_timer(3, PendingTimerKind::Wait);
    assert!(!timer.matches_authority(3, timer.deadline, PendingTimerKind::Ask));
}

#[test]
fn matches_authority_rejects_ask_timer_when_authority_says_wait() {
    let timer = make_timer(3, PendingTimerKind::Ask);
    assert!(!timer.matches_authority(3, timer.deadline, PendingTimerKind::Wait));
}

// ---------- Combined mismatches ----------

#[test]
fn matches_authority_rejects_when_generation_and_deadline_both_mismatch() {
    let timer = make_timer(5, PendingTimerKind::Wait);
    let other_deadline = timer.deadline + Duration::from_secs(1);
    assert!(!timer.matches_authority(10, other_deadline, PendingTimerKind::Wait));
}

#[test]
fn matches_authority_rejects_when_generation_and_kind_both_mismatch() {
    let timer = make_timer(5, PendingTimerKind::Wait);
    assert!(!timer.matches_authority(10, timer.deadline, PendingTimerKind::Ask));
}

#[test]
fn matches_authority_rejects_when_deadline_and_kind_both_mismatch() {
    let timer = make_timer(5, PendingTimerKind::Wait);
    let other_deadline = timer.deadline + Duration::from_secs(1);
    assert!(!timer.matches_authority(5, other_deadline, PendingTimerKind::Ask));
}

#[test]
fn matches_authority_rejects_when_all_fields_mismatch() {
    let timer = make_timer(5, PendingTimerKind::Wait);
    let other_deadline = timer.deadline + Duration::from_secs(1);
    assert!(!timer.matches_authority(10, other_deadline, PendingTimerKind::Ask));
}

// ---------- Edge cases ----------

#[test]
fn matches_authority_handles_max_generation_correctly() {
    let timer = make_timer(u64::MAX, PendingTimerKind::Wait);
    assert!(timer.matches_authority(u64::MAX, timer.deadline, PendingTimerKind::Wait));
    assert!(!timer.matches_authority(u64::MAX - 1, timer.deadline, PendingTimerKind::Wait));
}

#[test]
fn matches_authority_handles_generation_zero_correctly() {
    let timer = make_timer(0, PendingTimerKind::Ask);
    assert!(timer.matches_authority(0, timer.deadline, PendingTimerKind::Ask));
    assert!(!timer.matches_authority(1, timer.deadline, PendingTimerKind::Ask));
}

// ---------- Numeric generation validation with TimerTick boundary ----------

#[test]
fn generation_validation_works_independent_of_tick_value() {
    // Generation validation is about u64 equality, not about tick values
    let timer = make_timer(42, PendingTimerKind::Wait);
    // Same generation with matching deadline and kind succeeds
    assert!(timer.matches_authority(42, timer.deadline, PendingTimerKind::Wait));
    // Generation 42 does not match 43
    assert!(!timer.matches_authority(43, timer.deadline, PendingTimerKind::Wait));
}
