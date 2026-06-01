// =========================================================================
// types.rs — Unit Tests for Shard Type Definitions
// =========================================================================
// Extracted from vb_runtime/src/shard/types.rs inline test module.
// Tests PendingTimer, PendingTimerKind, ShardCommandQueue, ShardConfig,
// RuntimeState, RuntimeEvent, TimerTick, TimerDuration, TimerDeadline, TimerKind.

use super::*;
use std::time::Instant;

// Items from types.rs not re-exported by shard module
use crate::shard::types::{
    is_valid_command_queue_capacity, ShardCommandQueue,
};
use vb_core::ids::StepIdx;

// ---- PendingTimer ----

#[test]
fn pending_timer_constructor_sets_fields_correctly() {
    let deadline = Instant::now();
    let timer = PendingTimer {
        step: StepIdx::new(3),
        kind: PendingTimerKind::Ask,
        generation: 42,
        deadline,
    };
    assert_eq!(timer.step, StepIdx::new(3));
    assert_eq!(timer.kind, PendingTimerKind::Ask);
    assert_eq!(timer.generation, 42);
    assert_eq!(timer.deadline, deadline);
}

#[test]
fn pending_timer_matches_authority_when_all_fields_match() {
    let deadline = Instant::now();
    let timer = PendingTimer {
        step: StepIdx::ZERO,
        kind: PendingTimerKind::Wait,
        generation: 5,
        deadline,
    };
    assert!(timer.matches_authority(5, deadline, PendingTimerKind::Wait));
}

#[test]
fn pending_timer_matches_authority_rejects_wrong_generation() {
    let timer = PendingTimer {
        step: StepIdx::ZERO,
        kind: PendingTimerKind::Wait,
        generation: 5,
        deadline: Instant::now(),
    };
    assert!(!timer.matches_authority(6, timer.deadline, PendingTimerKind::Wait));
}

#[test]
fn pending_timer_matches_authority_rejects_wrong_kind() {
    let timer = PendingTimer {
        step: StepIdx::ZERO,
        kind: PendingTimerKind::Wait,
        generation: 3,
        deadline: Instant::now(),
    };
    assert!(!timer.matches_authority(3, timer.deadline, PendingTimerKind::Ask));
}

#[test]
fn pending_timer_is_copy() {
    let t1 = PendingTimer {
        step: StepIdx::new(1),
        kind: PendingTimerKind::Ask,
        generation: 7,
        deadline: Instant::now(),
    };
    let t2 = t1;
    assert_eq!(t1.generation, t2.generation);
    assert_eq!(t1.step, t2.step);
    assert_eq!(t1.kind, t2.kind);
}

// ---- PendingTimerKind ----

#[test]
fn pending_timer_kind_wait_and_ask_are_distinct() {
    assert_ne!(PendingTimerKind::Wait, PendingTimerKind::Ask);
    assert_eq!(PendingTimerKind::Wait, PendingTimerKind::Wait);
    assert_eq!(PendingTimerKind::Ask, PendingTimerKind::Ask);
}

// ---- is_valid_command_queue_capacity ----

#[test]
fn is_valid_capacity_accepts_one() {
    assert!(is_valid_command_queue_capacity(1));
}

#[test]
fn is_valid_capacity_accepts_max() {
    assert!(is_valid_command_queue_capacity(MAX_COMMAND_QUEUE_CAPACITY));
}

#[test]
fn is_valid_capacity_rejects_zero() {
    assert!(!is_valid_command_queue_capacity(0));
}

#[test]
fn is_valid_capacity_rejects_exceeding_max() {
    assert!(!is_valid_command_queue_capacity(
        MAX_COMMAND_QUEUE_CAPACITY + 1
    ));
}

// ---- ShardCommandQueue ----

#[test]
fn command_queue_new_accepts_valid_capacity() {
    let result = ShardCommandQueue::new(64);
    match result {
        Ok(q) => {
            assert_eq!(q.capacity(), 64);
            assert!(q.is_empty());
            assert!(!q.is_full());
        }
        Err(_e) => panic!("unexpected error constructing queue"),
    }
}

#[test]
fn command_queue_new_rejects_zero_capacity() {
    let result = ShardCommandQueue::new(0);
    match result {
        Err(RuntimeError::CommandQueueCapacityExceeded { capacity, max }) => {
            assert_eq!(capacity, 0);
            assert_eq!(max, MAX_COMMAND_QUEUE_CAPACITY);
        }
        _other => panic!("unexpected result constructing queue"),
    }
}

#[test]
fn command_queue_new_rejects_exceeding_max() {
    let result = ShardCommandQueue::new(MAX_COMMAND_QUEUE_CAPACITY + 1);
    assert!(result.is_err());
    match result {
        Err(RuntimeError::CommandQueueCapacityExceeded { .. }) => {}
        _other => panic!("unexpected result constructing queue"),
    }
}

#[test]
fn command_queue_remaining_capacity_decreases_after_enqueue() {
    let q = ShardCommandQueue::new(2).unwrap();
    assert_eq!(q.remaining_capacity(), 2);
    let cmd = ShardCommand::Shutdown;
    assert_eq!(q.enqueue(cmd), Ok(()));
    assert_eq!(q.remaining_capacity(), 1);
}

#[test]
fn command_queue_is_full_after_filling_to_capacity() {
    let q = ShardCommandQueue::new(1).unwrap();
    assert!(!q.is_full());
    assert_eq!(q.enqueue(ShardCommand::Shutdown), Ok(()));
    assert!(q.is_full());
}

#[test]
fn command_queue_enqueue_rejects_when_full() {
    let q = ShardCommandQueue::new(1).unwrap();
    assert_eq!(q.enqueue(ShardCommand::Shutdown), Ok(()));
    let result = q.enqueue(ShardCommand::Shutdown);
    assert_eq!(result, Err(RuntimeError::QueueFull));
}

#[test]
fn command_queue_pop_returns_fifo_order() {
    let q = ShardCommandQueue::new(2).unwrap();
    let cmd1 = ShardCommand::Shutdown;
    let cmd2 = ShardCommand::Cancel {
        run: RunId::new(1),
        reason: None,
    };
    assert_eq!(q.enqueue(cmd1.clone()), Ok(()));
    assert_eq!(q.enqueue(cmd2.clone()), Ok(()));
    assert_eq!(q.pop(), Some(cmd1));
    assert_eq!(q.pop(), Some(cmd2));
    assert!(q.is_empty());
}

#[test]
fn command_queue_len_accurately_reports_count() {
    let q = ShardCommandQueue::new(8).unwrap();
    assert_eq!(q.len(), 0);
    assert_eq!(q.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(q.len(), 1);
    assert_eq!(q.enqueue(ShardCommand::Shutdown), Ok(()));
    assert_eq!(q.len(), 2);
    q.pop();
    assert_eq!(q.len(), 1);
}

// ---- ShardConfig ----

#[test]
fn shard_config_default_has_valid_capacity() {
    let config = ShardConfig::default();
    assert!(config.command_queue_capacity > 0);
    assert!(is_valid_command_queue_capacity(
        config.command_queue_capacity
    ));
}

// ---- RuntimeState ----

#[test]
fn runtime_state_resumable_is_resumable() {
    assert!(RuntimeState::Resumable.is_resumable());
}

#[test]
fn runtime_state_running_is_not_resumable() {
    assert!(!RuntimeState::Running.is_resumable());
}

#[test]
fn runtime_state_failed_is_not_resumable() {
    assert!(!RuntimeState::Failed.is_resumable());
}

#[test]
fn runtime_state_initial_is_not_resumable() {
    assert!(!RuntimeState::Initial.is_resumable());
}

// ---- RuntimeEvent ----

#[test]
fn runtime_event_await_action_is_resumable() {
    assert!(RuntimeEvent::AwaitAction.is_resumable());
}

#[test]
fn runtime_event_await_timer_is_resumable() {
    assert!(RuntimeEvent::AwaitTimer.is_resumable());
}

#[test]
fn runtime_event_fail_is_terminal() {
    assert!(RuntimeEvent::Fail.is_terminal());
}

#[test]
fn runtime_event_drive_finished_is_terminal() {
    assert!(RuntimeEvent::DriveFinished.is_terminal());
}

// ---- Numeric Timer Types ----

#[test]
fn timer_tick_new_returns_expected_value() {
    let tick = TimerTick::new(42);
    assert_eq!(tick.get(), 42);
}

#[test]
fn timer_tick_checked_add_succeeds() {
    let tick = TimerTick::new(10);
    let dur = TimerDuration::new(5);
    let result = tick.checked_add(dur);
    assert_eq!(result, Some(TimerTick::new(15)));
}

#[test]
fn timer_tick_checked_add_returns_none_on_overflow() {
    let tick = TimerTick::new(u64::MAX);
    let dur = TimerDuration::new(1);
    assert_eq!(tick.checked_add(dur), None);
}

#[test]
fn timer_tick_has_elapsed_when_tick_eq_deadline() {
    let tick = TimerTick::new(100);
    let deadline = TimerDeadline::new(100);
    assert!(tick.has_elapsed(deadline));
}

#[test]
fn timer_tick_has_elapsed_when_tick_past_deadline() {
    let tick = TimerTick::new(101);
    let deadline = TimerDeadline::new(100);
    assert!(tick.has_elapsed(deadline));
}

#[test]
fn timer_tick_has_not_elapsed_when_before_deadline() {
    let tick = TimerTick::new(99);
    let deadline = TimerDeadline::new(100);
    assert!(!tick.has_elapsed(deadline));
}

#[test]
fn timer_tick_ord_sorts_correctly() {
    let a = TimerTick::new(10);
    let b = TimerTick::new(20);
    assert!(a < b);
    assert_eq!(a, TimerTick::new(10));
}

#[test]
fn timer_duration_new_and_get() {
    let dur = TimerDuration::new(30);
    assert_eq!(dur.get(), 30);
    assert_eq!(dur.as_ticks(), 30);
}

#[test]
fn timer_duration_zero_is_zero() {
    let dur = TimerDuration::zero();
    assert_eq!(dur.get(), 0);
}

#[test]
fn timer_duration_ord_sorts_correctly() {
    let a = TimerDuration::new(5);
    let b = TimerDuration::new(10);
    assert!(a < b);
}

#[test]
fn timer_deadline_new_and_get() {
    let dl = TimerDeadline::new(77);
    assert_eq!(dl.get(), 77);
}

#[test]
fn timer_deadline_from_tick_and_duration_succeeds() {
    let tick = TimerTick::new(10);
    let dur = TimerDuration::new(20);
    let deadline = TimerDeadline::from_tick_and_duration(tick, dur);
    assert_eq!(deadline, Some(TimerDeadline::new(30)));
}

#[test]
fn timer_deadline_from_tick_and_duration_returns_none_on_overflow() {
    let tick = TimerTick::new(u64::MAX);
    let dur = TimerDuration::new(1);
    assert_eq!(TimerDeadline::from_tick_and_duration(tick, dur), None);
}

#[test]
fn timer_deadline_is_past_when_current_tick_is_equal() {
    let current = TimerTick::new(50);
    let deadline = TimerDeadline::new(50);
    assert!(deadline.is_past(current));
}

#[test]
fn timer_deadline_is_past_when_current_tick_exceeds() {
    let current = TimerTick::new(51);
    let deadline = TimerDeadline::new(50);
    assert!(deadline.is_past(current));
}

#[test]
fn timer_deadline_is_not_past_when_current_tick_before() {
    let current = TimerTick::new(49);
    let deadline = TimerDeadline::new(50);
    assert!(!deadline.is_past(current));
}

#[test]
fn timer_deadline_ord_sorts_correctly() {
    let a = TimerDeadline::new(15);
    let b = TimerDeadline::new(25);
    assert!(a < b);
}

#[test]
fn timer_kind_retry_and_delayed_action_are_distinct() {
    use vb_core::ids::ActionId;
    let kind1 = TimerKind::Retry;
    let kind2 = TimerKind::DelayedAction(ActionId::new(1));
    assert_ne!(kind1, kind2);
}

#[test]
fn timer_kind_delayed_action_preserves_action_id() {
    use vb_core::ids::ActionId;
    let action = ActionId::new(42);
    let kind = TimerKind::DelayedAction(action);
    match kind {
        TimerKind::DelayedAction(aid) => assert_eq!(aid, ActionId::new(42)),
        _ => panic!("expected DelayedAction variant"),
    }
}

#[test]
fn timer_tick_copy_and_eq_preserves_value() {
    let t1 = TimerTick::new(5);
    let t2 = t1;
    assert_eq!(t1, t2);
}

#[test]
fn timer_duration_copy_and_eq_preserves_value() {
    let d1 = TimerDuration::new(10);
    let d2 = d1;
    assert_eq!(d1, d2);
}

#[test]
fn timer_deadline_copy_and_eq_preserves_value() {
    let dl1 = TimerDeadline::new(20);
    let dl2 = dl1;
    assert_eq!(dl1, dl2);
}

// ---- Numeric Timer Boundary Coverage ----

#[test]
fn timer_tick_zero_get_returns_zero() {
    assert_eq!(TimerTick::new(0).get(), 0);
}

#[test]
fn timer_tick_max_get_returns_max() {
    assert_eq!(TimerTick::new(u64::MAX).get(), u64::MAX);
}

#[test]
fn timer_tick_checked_add_zero_returns_self() {
    let tick = TimerTick::new(7);
    assert_eq!(tick.checked_add(TimerDuration::zero()), Some(tick));
}

#[test]
fn timer_tick_checked_add_zero_to_zero_returns_zero() {
    let tick = TimerTick::new(0);
    assert_eq!(
        tick.checked_add(TimerDuration::zero()),
        Some(TimerTick::new(0))
    );
}

#[test]
fn timer_tick_checked_add_max_minus_one_plus_one_returns_max() {
    let tick = TimerTick::new(u64::MAX - 1);
    let dur = TimerDuration::new(1);
    assert_eq!(tick.checked_add(dur), Some(TimerTick::new(u64::MAX)));
}

#[test]
fn timer_tick_checked_add_max_plus_zero_returns_max() {
    let tick = TimerTick::new(u64::MAX);
    assert_eq!(
        tick.checked_add(TimerDuration::zero()),
        Some(TimerTick::new(u64::MAX))
    );
}

#[test]
fn timer_tick_checked_add_zero_plus_max_overflows() {
    // 0 + u64::MAX = u64::MAX (no overflow)
    let tick = TimerTick::new(0);
    let dur = TimerDuration::new(u64::MAX);
    assert_eq!(tick.checked_add(dur), Some(TimerTick::new(u64::MAX)));
}

#[test]
fn timer_tick_has_elapsed_zero_vs_zero_is_true() {
    assert!(TimerTick::new(0).has_elapsed(TimerDeadline::new(0)));
}

#[test]
fn timer_tick_has_elapsed_zero_vs_one_is_false() {
    assert!(!TimerTick::new(0).has_elapsed(TimerDeadline::new(1)));
}

#[test]
fn timer_tick_has_elapsed_max_vs_max_is_true() {
    assert!(TimerTick::new(u64::MAX).has_elapsed(TimerDeadline::new(u64::MAX)));
}

#[test]
fn timer_tick_has_elapsed_max_vs_max_minus_one_is_true() {
    assert!(TimerTick::new(u64::MAX).has_elapsed(TimerDeadline::new(u64::MAX - 1)));
}

#[test]
fn timer_tick_has_elapsed_max_minus_one_vs_max_is_false() {
    assert!(!TimerTick::new(u64::MAX - 1).has_elapsed(TimerDeadline::new(u64::MAX)));
}

#[test]
fn timer_tick_partial_cmp_is_consistent_with_ord() {
    let a = TimerTick::new(5);
    let b = TimerTick::new(10);
    assert_eq!(a.partial_cmp(&b), Some(std::cmp::Ordering::Less));
    assert_eq!(b.partial_cmp(&a), Some(std::cmp::Ordering::Greater));
    assert_eq!(a.partial_cmp(&a), Some(std::cmp::Ordering::Equal));
    // partial_cmp of equal values
    assert_eq!(
        TimerTick::new(3).partial_cmp(&TimerTick::new(3)),
        Some(std::cmp::Ordering::Equal)
    );
}

#[test]
fn timer_tick_hash_is_consistent_with_eq() {
    use std::hash::{Hash, Hasher};
    // Two equal values should have the same hash
    let t1 = TimerTick::new(42);
    let t2 = TimerTick::new(42);
    let mut h1 = std::collections::hash_map::DefaultHasher::new();
    let mut h2 = std::collections::hash_map::DefaultHasher::new();
    t1.hash(&mut h1);
    t2.hash(&mut h2);
    assert_eq!(h1.finish(), h2.finish());
}

#[test]
fn timer_duration_max_get_returns_max() {
    assert_eq!(TimerDuration::new(u64::MAX).get(), u64::MAX);
}

#[test]
fn timer_duration_one_get_returns_one() {
    assert_eq!(TimerDuration::new(1).get(), 1);
}

#[test]
fn timer_duration_partial_cmp_is_consistent_with_ord() {
    let a = TimerDuration::new(3);
    let b = TimerDuration::new(7);
    assert_eq!(a.partial_cmp(&b), Some(std::cmp::Ordering::Less));
    assert_eq!(b.partial_cmp(&a), Some(std::cmp::Ordering::Greater));
}

#[test]
fn timer_duration_hash_is_consistent_with_eq() {
    use std::hash::{Hash, Hasher};
    let d1 = TimerDuration::new(10);
    let d2 = TimerDuration::new(10);
    let mut h1 = std::collections::hash_map::DefaultHasher::new();
    let mut h2 = std::collections::hash_map::DefaultHasher::new();
    d1.hash(&mut h1);
    d2.hash(&mut h2);
    assert_eq!(h1.finish(), h2.finish());
}

#[test]
fn timer_deadline_max_get_returns_max() {
    assert_eq!(TimerDeadline::new(u64::MAX).get(), u64::MAX);
}

#[test]
fn timer_deadline_zero_get_returns_zero() {
    assert_eq!(TimerDeadline::new(0).get(), 0);
}

#[test]
fn timer_deadline_from_tick_and_duration_zero_plus_zero() {
    let tick = TimerTick::new(0);
    let dur = TimerDuration::new(0);
    assert_eq!(
        TimerDeadline::from_tick_and_duration(tick, dur),
        Some(TimerDeadline::new(0))
    );
}

#[test]
fn timer_deadline_from_tick_and_duration_one_plus_max_overflows() {
    let tick = TimerTick::new(1);
    let dur = TimerDuration::new(u64::MAX);
    assert_eq!(TimerDeadline::from_tick_and_duration(tick, dur), None);
}

#[test]
fn timer_deadline_from_tick_and_duration_max_plus_max_overflows() {
    let tick = TimerTick::new(u64::MAX);
    let dur = TimerDuration::new(u64::MAX);
    assert_eq!(TimerDeadline::from_tick_and_duration(tick, dur), None);
}

#[test]
fn timer_deadline_from_tick_and_duration_max_minus_two_plus_one() {
    let tick = TimerTick::new(u64::MAX - 2);
    let dur = TimerDuration::new(1);
    assert_eq!(
        TimerDeadline::from_tick_and_duration(tick, dur),
        Some(TimerDeadline::new(u64::MAX - 1))
    );
}

#[test]
fn timer_deadline_is_past_zero_vs_zero_is_true() {
    assert!(TimerDeadline::new(0).is_past(TimerTick::new(0)));
}

#[test]
fn timer_deadline_is_past_max_vs_max_is_true() {
    assert!(TimerDeadline::new(u64::MAX).is_past(TimerTick::new(u64::MAX)));
}

#[test]
fn timer_deadline_is_past_one_vs_zero_is_false() {
    assert!(!TimerDeadline::new(1).is_past(TimerTick::new(0)));
}

#[test]
fn timer_deadline_partial_cmp_is_consistent_with_ord() {
    let a = TimerDeadline::new(5);
    let b = TimerDeadline::new(10);
    assert_eq!(a.partial_cmp(&b), Some(std::cmp::Ordering::Less));
}

#[test]
fn timer_deadline_hash_is_consistent_with_eq() {
    use std::hash::{Hash, Hasher};
    let dl1 = TimerDeadline::new(15);
    let dl2 = TimerDeadline::new(15);
    let mut h1 = std::collections::hash_map::DefaultHasher::new();
    let mut h2 = std::collections::hash_map::DefaultHasher::new();
    dl1.hash(&mut h1);
    dl2.hash(&mut h2);
    assert_eq!(h1.finish(), h2.finish());
}

// ---- TimerKind variant coverage ----

#[test]
fn timer_kind_retry_equals_retry() {
    assert_eq!(TimerKind::Retry, TimerKind::Retry);
}

#[test]
fn timer_kind_delayed_action_equals_same_action_id() {
    use vb_core::ids::ActionId;
    assert_eq!(
        TimerKind::DelayedAction(ActionId::new(7)),
        TimerKind::DelayedAction(ActionId::new(7))
    );
}

#[test]
fn timer_kind_delayed_action_differs_with_different_action_id() {
    use vb_core::ids::ActionId;
    assert_ne!(
        TimerKind::DelayedAction(ActionId::new(1)),
        TimerKind::DelayedAction(ActionId::new(2))
    );
}

#[test]
fn timer_kind_clone_preserves_value() {
    use vb_core::ids::ActionId;
    let k1 = TimerKind::DelayedAction(ActionId::new(99));
    let k2 = k1;
    assert_eq!(k1, k2);
}

// ---- Debug format does not panic ----

#[test]
fn timer_tick_debug_format() {
    let tick = TimerTick::new(42);
    let s = format!("{:?}", tick);
    assert!(s.contains("42"));
}

#[test]
fn timer_duration_debug_format() {
    let dur = TimerDuration::new(10);
    let s = format!("{:?}", dur);
    assert!(s.contains("10"));
}

#[test]
fn timer_deadline_debug_format() {
    let dl = TimerDeadline::new(7);
    let s = format!("{:?}", dl);
    assert!(s.contains("7"));
}

#[test]
fn timer_kind_debug_format() {
    use vb_core::ids::ActionId;
    let k = TimerKind::Retry;
    let s = format!("{:?}", k);
    assert!(!s.is_empty());

    let k2 = TimerKind::DelayedAction(ActionId::new(42));
    let s2 = format!("{:?}", k2);
    assert!(s2.contains("42"));
}
