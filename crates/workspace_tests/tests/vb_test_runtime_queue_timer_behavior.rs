//! BEHAVIOR tests for vb_runtime timer wheel.
//!
//! Tests cover:
//! - Timer wheel scheduling behavior
//! - Overflow and boundedness behavior
//! - Exact timing assertions
//!
//! Note: The BoundedActionCompletionQueue (action_queue) is not publicly exported
//! from vb_runtime. Timer wheel tests use the public vb_runtime::shard::timer_wheel API.

use vb_core::ids::RunId;
use vb_runtime::shard::timer_wheel::TimerWheel;
use vb_runtime::shard::types::PendingTimerKind;

// ============================================================================
// GROUP: Timer Wheel Insert and Cancel Behavior
// ============================================================================

/// Verifies insert creates a timer and cancel removes it.
#[test]
fn timer_wheel_insert_and_cancel_round_trip() {
    let mut wheel = TimerWheel::new();
    let now = std::time::Instant::now();

    assert!(wheel.is_empty(), "new timer wheel must be empty");

    wheel.insert(RunId::new(1), now, PendingTimerKind::Wait).unwrap();
    assert!(!wheel.is_empty(), "wheel must not be empty after insert");
    assert_eq!(wheel.len(), 1, "wheel must have 1 timer after insert");

    let cancelled = wheel.cancel(RunId::new(1));
    assert!(cancelled, "cancel must return true for existing timer");
    assert!(wheel.is_empty(), "wheel must be empty after cancel");
}

/// Verifies cancel returns false for nonexistent timer.
#[test]
fn timer_wheel_cancel_nonexistent_returns_false() {
    let mut wheel = TimerWheel::new();
    let result = wheel.cancel(RunId::new(99));
    assert!(!result, "cancel must return false for nonexistent timer");
}

/// Verifies replacing an existing timer updates the deadline and kind.
#[test]
fn timer_wheel_replace_updates_deadline_and_kind() {
    let mut wheel = TimerWheel::new();
    let now = std::time::Instant::now();
    let d1 = now + std::time::Duration::from_millis(10);
    let d2 = now + std::time::Duration::from_millis(20);

    // Insert with Wait kind
    wheel.insert(RunId::new(1), d1, PendingTimerKind::Wait).unwrap();
    assert_eq!(wheel.get_kind(RunId::new(1)), Some(PendingTimerKind::Wait));
    assert_eq!(wheel.next_deadline(), Some(d1));

    // Replace with Ask kind and later deadline
    wheel.insert(RunId::new(1), d2, PendingTimerKind::Ask).unwrap();
    assert_eq!(wheel.len(), 1, "wheel must still have only 1 timer after replace");
    assert_eq!(wheel.get_kind(RunId::new(1)), Some(PendingTimerKind::Ask));
    assert_eq!(wheel.next_deadline(), Some(d2), "next deadline must be the new deadline");
}

// ============================================================================
// GROUP: Timer Wheel Fire Expired Behavior
// ============================================================================

/// Verifies fire_expired returns only timers whose deadline has passed.
#[test]
fn timer_wheel_fire_expired_returns_only_past_deadlines() {
    let mut wheel = TimerWheel::new();
    let now = std::time::Instant::now();
    let past = now - std::time::Duration::from_millis(100);
    let future = now + std::time::Duration::from_secs(60);

    wheel.insert(RunId::new(1), past, PendingTimerKind::Wait).unwrap();
    wheel.insert(RunId::new(2), future, PendingTimerKind::Ask).unwrap();

    let fired = wheel.fire_expired(now);

    assert_eq!(fired.len(), 1, "only 1 timer must fire (the past one)");
    assert_eq!(fired[0].run, RunId::new(1), "fired timer must be run 1 (the past deadline)");
    assert_eq!(wheel.len(), 1, "wheel must still contain the future timer");
}

/// Verifies fire_expired at exact deadline fires the timer.
#[test]
fn timer_wheel_fire_expired_at_exact_deadline_fires() {
    let mut wheel = TimerWheel::new();
    let deadline = std::time::Instant::now();

    wheel.insert(RunId::new(1), deadline, PendingTimerKind::Wait).unwrap();
    let fired = wheel.fire_expired(deadline);

    assert_eq!(fired.len(), 1, "timer must fire at exact deadline");
    assert_eq!(fired[0].run, RunId::new(1));
}

/// Verifies fire_expired drains all expired timers in deadline order.
#[test]
fn timer_wheel_fire_expired_returns_deadline_order() {
    let mut wheel = TimerWheel::new();
    let now = std::time::Instant::now();
    let d1 = now - std::time::Duration::from_millis(200);
    let d2 = now - std::time::Duration::from_millis(100);

    wheel.insert(RunId::new(1), d1, PendingTimerKind::Wait).unwrap();
    wheel.insert(RunId::new(2), d2, PendingTimerKind::Ask).unwrap();

    let fired = wheel.fire_expired(now);

    assert_eq!(fired.len(), 2, "both expired timers must fire");
    // Must be in deadline order (d1 before d2)
    assert_eq!(fired[0].run, RunId::new(1), "earlier deadline must fire first");
    assert_eq!(fired[1].run, RunId::new(2), "later deadline must fire second");
    assert!(wheel.is_empty(), "wheel must be empty after draining all expired");
}

/// Verifies multiple timers at same deadline all fire together.
#[test]
fn timer_wheel_multiple_runs_at_same_deadline() {
    let mut wheel = TimerWheel::new();
    let now = std::time::Instant::now();
    let deadline = now + std::time::Duration::from_millis(50);

    wheel.insert(RunId::new(1), deadline, PendingTimerKind::Wait).unwrap();
    wheel.insert(RunId::new(2), deadline, PendingTimerKind::Ask).unwrap();
    wheel.insert(RunId::new(3), deadline, PendingTimerKind::Wait).unwrap();

    assert_eq!(wheel.len(), 3, "wheel must have 3 timers");
    let fired = wheel.fire_expired(deadline);

    assert_eq!(fired.len(), 3, "all 3 timers must fire at deadline");
    assert!(wheel.is_empty(), "wheel must be empty after firing");
}

// ============================================================================
// GROUP: Timer Wheel Generation Behavior
// ============================================================================

/// Verifies generation increments on each replacement (validates freshness).
#[test]
fn timer_wheel_generation_increments_on_replace() {
    let mut wheel = TimerWheel::new();
    let now = std::time::Instant::now();

    wheel.insert(RunId::new(1), now, PendingTimerKind::Wait).unwrap();
    let entry1 = wheel.get_entry(RunId::new(1)).unwrap();
    assert_eq!(entry1.generation, 1, "first insert must have generation=1");

    let later = now + std::time::Duration::from_secs(1);
    wheel.insert(RunId::new(1), later, PendingTimerKind::Ask).unwrap();
    let entry2 = wheel.get_entry(RunId::new(1)).unwrap();
    assert_eq!(entry2.generation, 2, "replacement must increment generation");
    assert_eq!(entry2.kind, PendingTimerKind::Ask, "kind must be updated");
}

// ============================================================================
// GROUP: Timer Wheel Next Deadline Behavior
// ============================================================================

/// Verifies next_deadline returns earliest deadline.
#[test]
fn timer_wheel_next_deadline_returns_earliest() {
    let mut wheel = TimerWheel::new();
    let now = std::time::Instant::now();
    let early = now + std::time::Duration::from_millis(10);
    let late = now + std::time::Duration::from_millis(100);

    wheel.insert(RunId::new(1), late, PendingTimerKind::Wait).unwrap();
    wheel.insert(RunId::new(2), early, PendingTimerKind::Ask).unwrap();

    assert_eq!(wheel.next_deadline(), Some(early), "next_deadline must return earliest");
}

/// Verifies next_deadline returns None when wheel is empty.
#[test]
fn timer_wheel_next_deadline_none_when_empty() {
    let wheel = TimerWheel::new();
    assert!(wheel.next_deadline().is_none(), "empty wheel must have no next_deadline");
}

/// Verifies next_deadline updates after firing earliest timer.
#[test]
fn timer_wheel_next_deadline_updates_after_fire() {
    let mut wheel = TimerWheel::new();
    let now = std::time::Instant::now();
    let d1 = now - std::time::Duration::from_millis(50); // past, fires first
    let d2 = now + std::time::Duration::from_millis(100);

    wheel.insert(RunId::new(1), d1, PendingTimerKind::Wait).unwrap();
    wheel.insert(RunId::new(2), d2, PendingTimerKind::Ask).unwrap();

    assert_eq!(wheel.next_deadline(), Some(d1), "initial next_deadline is the past deadline");

    wheel.fire_expired(now);

    assert_eq!(wheel.next_deadline(), Some(d2), "next_deadline must be d2 after firing d1");
}

// ============================================================================
// GROUP: Timer Wheel Boundedness
// ============================================================================

/// Verifies len() tracks active timers accurately.
#[test]
fn timer_wheel_len_tracks_active_timers() {
    let mut wheel = TimerWheel::new();
    let now = std::time::Instant::now();

    assert_eq!(wheel.len(), 0, "new wheel must have len=0");

    wheel.insert(RunId::new(1), now, PendingTimerKind::Wait).unwrap();
    assert_eq!(wheel.len(), 1);

    wheel.insert(RunId::new(2), now, PendingTimerKind::Ask).unwrap();
    assert_eq!(wheel.len(), 2);

    wheel.cancel(RunId::new(1));
    assert_eq!(wheel.len(), 1);

    wheel.cancel(RunId::new(2));
    assert_eq!(wheel.len(), 0);
}

/// Verifies get_entry returns correct entry with generation.
#[test]
fn timer_wheel_get_entry_returns_correct_entry() {
    let mut wheel = TimerWheel::new();
    let now = std::time::Instant::now();

    wheel.insert(RunId::new(1), now, PendingTimerKind::Wait).unwrap();
    let entry = wheel.get_entry(RunId::new(1));

    assert!(entry.is_some(), "get_entry must return Some for existing run");
    let e = entry.unwrap();
    assert_eq!(e.run, RunId::new(1));
    assert_eq!(e.generation, 1);
    assert_eq!(e.kind, PendingTimerKind::Wait);
}

/// Verifies get_kind returns correct kind for existing run.
#[test]
fn timer_wheel_get_kind_returns_correct_kind() {
    let mut wheel = TimerWheel::new();
    let now = std::time::Instant::now();

    assert_eq!(wheel.get_kind(RunId::new(1)), None, "get_kind must return None for unknown run");

    wheel.insert(RunId::new(1), now, PendingTimerKind::Ask).unwrap();
    assert_eq!(wheel.get_kind(RunId::new(1)), Some(PendingTimerKind::Ask));
}

/// Verifies default timer wheel is empty.
#[test]
fn timer_wheel_default_is_empty() {
    let wheel = TimerWheel::default();
    assert!(wheel.is_empty());
    assert_eq!(wheel.len(), 0);
}

// ============================================================================
// GROUP: Timer Wheel Run-Indexed Lookup
// ============================================================================

/// Verifies timer can be looked up by run after insert.
#[test]
fn timer_wheel_run_lookup_after_insert() {
    let mut wheel = TimerWheel::new();
    let now = std::time::Instant::now();

    wheel.insert(RunId::new(42), now, PendingTimerKind::Wait).unwrap();

    let entry = wheel.get_entry(RunId::new(42));
    assert!(entry.is_some(), "must find timer by run id");
    assert_eq!(entry.unwrap().run, RunId::new(42));
}

/// Verifies canceling removes from both indexes.
#[test]
fn timer_wheel_cancel_removes_from_both_indexes() {
    let mut wheel = TimerWheel::new();
    let now = std::time::Instant::now();

    wheel.insert(RunId::new(1), now, PendingTimerKind::Wait).unwrap();
    assert!(wheel.get_entry(RunId::new(1)).is_some());

    wheel.cancel(RunId::new(1));

    assert!(wheel.get_entry(RunId::new(1)).is_none(), "run lookup must be None after cancel");
    assert!(wheel.is_empty(), "wheel must be empty after cancel");
}

/// Verifies is_empty returns true only when wheel has no timers.
#[test]
fn timer_wheel_is_empty_reflects_actual_state() {
    let mut wheel = TimerWheel::new();
    let now = std::time::Instant::now();

    assert!(wheel.is_empty(), "new wheel must be empty");

    wheel.insert(RunId::new(1), now, PendingTimerKind::Wait).unwrap();
    assert!(!wheel.is_empty(), "wheel with 1 timer must not be empty");

    wheel.cancel(RunId::new(1));
    assert!(wheel.is_empty(), "wheel after cancel must be empty");
}

/// Verifies fire_expired does not fire future timers.
#[test]
fn timer_wheel_fire_expired_ignores_future_timers() {
    let mut wheel = TimerWheel::new();
    let now = std::time::Instant::now();
    let future1 = now + std::time::Duration::from_secs(5);
    let future2 = now + std::time::Duration::from_secs(10);

    wheel.insert(RunId::new(1), future1, PendingTimerKind::Wait).unwrap();
    wheel.insert(RunId::new(2), future2, PendingTimerKind::Ask).unwrap();

    let fired = wheel.fire_expired(now);

    assert!(fired.is_empty(), "no timers must fire when none are expired");
    assert_eq!(wheel.len(), 2, "both timers must still be in wheel");
}

/// Verifies canceling one timer does not affect others.
#[test]
fn timer_wheel_cancel_one_does_not_affect_others() {
    let mut wheel = TimerWheel::new();
    let now = std::time::Instant::now();

    wheel.insert(RunId::new(1), now, PendingTimerKind::Wait).unwrap();
    wheel.insert(RunId::new(2), now, PendingTimerKind::Ask).unwrap();
    wheel.insert(RunId::new(3), now, PendingTimerKind::Wait).unwrap();

    wheel.cancel(RunId::new(2));

    assert_eq!(wheel.len(), 2, "wheel must have 2 timers after cancel");
    assert!(wheel.get_entry(RunId::new(1)).is_some(), "run 1 still present");
    assert!(wheel.get_entry(RunId::new(2)).is_none(), "run 2 removed");
    assert!(wheel.get_entry(RunId::new(3)).is_some(), "run 3 still present");
}
