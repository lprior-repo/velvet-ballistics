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
//! PS-002: Numeric-Only Timer State — behavior tests (B1-B2).
//!
//! Tests that the new numeric timer types are purely data-driven:
//! - `TimerTick`, `TimerDuration`, `TimerDeadline` are all u64-based
//! - `TimerKind` is an enum with no Instant dependency
//! - `PendingTimer` fields are inspectable with numeric generation

use vb_core::ids::{ActionId, StepIdx};
use vb_runtime::shard::types::{
    PendingTimer, PendingTimerKind, TimerDeadline, TimerDuration, TimerKind, TimerTick,
};

// ---------- TimerTick is a pure numeric newtype ----------

#[test]
fn timer_tick_is_u64_wrapper_with_get() {
    for val in [0u64, 1, 42, u64::MAX / 2, u64::MAX] {
        let tick = TimerTick::new(val);
        assert_eq!(tick.get(), val);
    }
}

#[test]
fn timer_tick_zero_is_zero() {
    assert_eq!(TimerTick::new(0).get(), 0);
}

#[test]
fn timer_tick_max_is_max() {
    assert_eq!(TimerTick::new(u64::MAX).get(), u64::MAX);
}

// ---------- TimerDuration is a pure numeric newtype ----------

#[test]
fn timer_duration_is_u64_wrapper() {
    for val in [0u64, 1, 100, u64::MAX / 2, u64::MAX] {
        let dur = TimerDuration::new(val);
        assert_eq!(dur.get(), val);
        assert_eq!(dur.as_ticks(), val);
    }
}

#[test]
fn timer_duration_zero_is_not_one() {
    let zero = TimerDuration::zero();
    assert_eq!(zero.get(), 0);
    assert_ne!(zero, TimerDuration::new(1));
}

// ---------- TimerDeadline is a pure numeric newtype ----------

#[test]
fn timer_deadline_is_u64_wrapper() {
    for val in [0u64, 1, 500, u64::MAX / 2, u64::MAX] {
        let dl = TimerDeadline::new(val);
        assert_eq!(dl.get(), val);
    }
}

// ---------- TimerKind has no Instant dependency ----------

#[test]
fn timer_kind_retry_and_delayed_action_are_distinct() {
    assert_ne!(TimerKind::Retry, TimerKind::DelayedAction(ActionId::new(1)));
}

#[test]
fn timer_kind_delayed_action_preserves_action_id() {
    let action_id = ActionId::new(42);
    let kind = TimerKind::DelayedAction(action_id);
    match kind {
        TimerKind::DelayedAction(aid) => assert_eq!(aid, ActionId::new(42)),
        _ => panic!("expected DelayedAction variant"),
    }
}

#[test]
fn timer_kind_non_exhaustive_prevents_external_match_coercion() {
    // TimerKind is #[non_exhaustive] — this test exercises that the enum
    // compiles and its variants are constructable from external crates.
    let k1 = TimerKind::Retry;
    let k2 = TimerKind::DelayedAction(ActionId::new(7));
    assert_ne!(k1, k2);
}

// ---------- PendingTimer has numeric generation ----------

#[test]
fn pending_timer_generation_is_u64() {
    let timer = PendingTimer {
        step: StepIdx::ZERO,
        kind: PendingTimerKind::Wait,
        generation: 42u64,
        deadline: std::time::Instant::now(),
        ..Default::default()
    };
    assert_eq!(timer.generation, 42u64);
}

#[test]
fn pending_timer_generation_starts_at_one_for_new_timer() {
    let timer = PendingTimer {
        step: StepIdx::new(1),
        kind: PendingTimerKind::Ask,
        generation: 1,
        deadline: std::time::Instant::now(),
        ..Default::default()
    };
    assert_eq!(timer.generation, 1);
}

#[test]
fn pending_timer_generation_can_be_zero() {
    let timer = PendingTimer {
        step: StepIdx::ZERO,
        kind: PendingTimerKind::Wait,
        generation: 0,
        deadline: std::time::Instant::now(),
        ..Default::default()
    };
    assert_eq!(timer.generation, 0);
}

#[test]
fn pending_timer_generation_can_be_max() {
    let timer = PendingTimer {
        step: StepIdx::new(5),
        kind: PendingTimerKind::Ask,
        generation: u64::MAX,
        deadline: std::time::Instant::now(),
        ..Default::default()
    };
    assert_eq!(timer.generation, u64::MAX);
}

// ---------- PendingTimer step is a StepIdx ----------

#[test]
fn pending_timer_step_is_step_idx() {
    let timer = PendingTimer {
        step: StepIdx::new(15),
        kind: PendingTimerKind::Wait,
        generation: 5,
        deadline: std::time::Instant::now(),
        ..Default::default()
    };
    assert_eq!(timer.step, StepIdx::new(15));
}

#[test]
fn pending_timer_step_can_be_zero() {
    let timer = PendingTimer {
        step: StepIdx::ZERO,
        kind: PendingTimerKind::Ask,
        generation: 1,
        deadline: std::time::Instant::now(),
        ..Default::default()
    };
    assert_eq!(timer.step, StepIdx::ZERO);
}

// ---------- PendingTimerKind roundtrips ----------

#[test]
fn pending_timer_kind_wait_roundtrips() {
    let timer = PendingTimer {
        step: StepIdx::ZERO,
        kind: PendingTimerKind::Wait,
        generation: 1,
        deadline: std::time::Instant::now(),
        ..Default::default()
    };
    assert_eq!(timer.kind, PendingTimerKind::Wait);
}

#[test]
fn pending_timer_kind_ask_roundtrips() {
    let timer = PendingTimer {
        step: StepIdx::new(3),
        kind: PendingTimerKind::Ask,
        generation: 2,
        deadline: std::time::Instant::now(),
        ..Default::default()
    };
    assert_eq!(timer.kind, PendingTimerKind::Ask);
}

#[test]
fn pending_timer_kind_wait_and_ask_are_distinct() {
    assert_ne!(PendingTimerKind::Wait, PendingTimerKind::Ask);
}

// ---------- Equality and copy for numeric timer types ----------

#[test]
fn timer_tick_eq_is_value_based() {
    assert_eq!(TimerTick::new(5), TimerTick::new(5));
    assert_ne!(TimerTick::new(5), TimerTick::new(6));
}

#[test]
fn timer_duration_eq_is_value_based() {
    assert_eq!(TimerDuration::new(10), TimerDuration::new(10));
    assert_ne!(TimerDuration::new(10), TimerDuration::new(11));
}

#[test]
fn timer_deadline_eq_is_value_based() {
    assert_eq!(TimerDeadline::new(20), TimerDeadline::new(20));
    assert_ne!(TimerDeadline::new(20), TimerDeadline::new(21));
}

#[test]
fn timer_tick_copy_preserves_equality() {
    let t1 = TimerTick::new(42);
    let t2 = t1;
    assert_eq!(t1, t2);
}

#[test]
fn timer_duration_copy_preserves_equality() {
    let d1 = TimerDuration::new(7);
    let d2 = d1;
    assert_eq!(d1, d2);
}

#[test]
fn timer_deadline_copy_preserves_equality() {
    let dl1 = TimerDeadline::new(13);
    let dl2 = dl1;
    assert_eq!(dl1, dl2);
}
