use vb_core::action::{ActionError, ActionTicket, Idempotency};
use vb_core::{ActionId, RunId, SeqNo, StepIdx};
use vb_runtime::idempotency::IdempotencyTracker;

fn ticket(key: u128) -> ActionTicket {
    ActionTicket {
        run: RunId::new(1),
        step: StepIdx::ZERO,
        seq: SeqNo::new(1),
        action: ActionId::new(1),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: key,
        capacity: 1,
        fencing_token: 0,
    }
}

#[test]
fn duplicate_completion_does_not_refresh_fifo_or_evict() {
    let mut tracker = IdempotencyTracker::new(2);
    assert_eq!(tracker.mark_completed(&ticket(1)), Ok(()));
    assert_eq!(tracker.mark_completed(&ticket(2)), Ok(()));
    assert_eq!(
        tracker.mark_completed(&ticket(1)),
        Err(ActionError::CompletionAlreadyRecorded)
    );
    assert_eq!(tracker.len(), 2);
    assert!(tracker.is_completed(&ticket(2)));
    assert_eq!(tracker.mark_completed(&ticket(3)), Ok(()));
    assert!(!tracker.is_completed(&ticket(1)));
    assert!(tracker.is_completed(&ticket(2)));
    assert!(tracker.is_completed(&ticket(3)));
    assert_eq!(tracker.mark_completed(&ticket(1)), Ok(()));
    assert!(!tracker.is_completed(&ticket(2)));
    assert!(tracker.is_completed(&ticket(3)));
    assert!(tracker.is_completed(&ticket(1)));
}

#[test]
fn zero_capacity_retains_one_and_fresh_construction_resets_state() {
    let mut tracker = IdempotencyTracker::with_capacity(0);
    assert_eq!(tracker.mark_completed(&ticket(1)), Ok(()));
    assert_eq!(tracker.mark_completed(&ticket(2)), Ok(()));
    assert_eq!(tracker.len(), 1);
    assert!(!tracker.is_completed(&ticket(1)));
    assert!(tracker.is_completed(&ticket(2)));
    tracker = IdempotencyTracker::with_capacity(0);
    assert!(tracker.is_empty());
    assert!(tracker.mark_dispatched(&ticket(2)));
}

#[test]
fn maximum_capacity_construction_stays_lazy_and_usable() {
    let mut tracker = IdempotencyTracker::new(usize::MAX);
    assert!(tracker.is_empty());
    assert_eq!(tracker.mark_completed(&ticket(1)), Ok(()));
    assert_eq!(tracker.len(), 1);
}

#[test]
fn ticket_eviction_keeps_independent_policy_deduplication() {
    let mut tracker = IdempotencyTracker::new(1);
    let policy = Idempotency::AtLeastOnceExternal;
    assert!(tracker.track_for_policy(policy, 1));
    assert_eq!(tracker.mark_completed(&ticket(1)), Ok(()));
    assert_eq!(tracker.mark_completed(&ticket(2)), Ok(()));
    assert!(!tracker.is_completed(&ticket(1)));
    assert!(tracker.is_completed_for_policy(policy, 1));
    assert!(!tracker.track_for_policy(policy, 1));
    assert_eq!(
        tracker.mark_completed_for_policy(policy, 1),
        Err(ActionError::CompletionAlreadyRecorded)
    );
    for pure in [
        Idempotency::DeterministicPure,
        Idempotency::IdempotentExternal,
    ] {
        assert!(tracker.track_for_policy(pure, 1));
        assert_eq!(tracker.mark_completed_for_policy(pure, 1), Ok(()));
        assert!(!tracker.is_completed_for_policy(pure, 1));
    }
}
