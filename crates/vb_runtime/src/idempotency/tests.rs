//! Idempotency tracker tests.

use vb_core::action::{ActionTicket, Idempotency};
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

use super::IdempotencyTracker;

fn make_ticket(key: u128) -> ActionTicket {
    ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: key,
        capacity: 1,
        ..Default::default()
    }
}

#[test]
fn idempotency_tracker_new_is_empty() {
    let tracker = IdempotencyTracker::with_default_capacity();
    assert!(tracker.is_empty());
    assert_eq!(tracker.len(), 0);
}

#[test]
fn idempotency_tracker_record_completion_succeeds() {
    let mut tracker = IdempotencyTracker::with_default_capacity();
    let ticket = make_ticket(42);
    assert_eq!(tracker.mark_completed(&ticket), Ok(()));
    assert!(tracker.is_completed(&ticket));
    assert_eq!(tracker.len(), 1);
}

#[test]
fn idempotency_tracker_duplicate_completion_returns_error() {
    let mut tracker = IdempotencyTracker::with_default_capacity();
    let ticket = make_ticket(99);
    assert_eq!(tracker.mark_completed(&ticket), Ok(()));
    assert_eq!(
        tracker.mark_completed(&ticket),
        Err(vb_core::action::ActionError::CompletionAlreadyRecorded)
    );
}

#[test]
fn idempotency_tracker_different_keys_are_independent() {
    let mut tracker = IdempotencyTracker::with_default_capacity();
    let ticket_a = make_ticket(1);
    let ticket_b = make_ticket(2);
    let ticket_c = make_ticket(3);
    assert_eq!(tracker.mark_completed(&ticket_a), Ok(()));
    assert_eq!(tracker.mark_completed(&ticket_b), Ok(()));
    assert!(tracker.is_completed(&ticket_a));
    assert!(tracker.is_completed(&ticket_b));
    assert!(!tracker.is_completed(&ticket_c));
    assert_eq!(tracker.len(), 2);
}

#[test]
fn idempotency_tracker_default_matches_new() {
    let default = IdempotencyTracker::default();
    let new = IdempotencyTracker::with_default_capacity();
    assert_eq!(default.len(), new.len());
    assert_eq!(default.is_empty(), new.is_empty());
    assert_eq!(default.capacity, new.capacity);
}

#[test]
fn idempotency_tracker_mark_dispatched_new_is_true() {
    let tracker = IdempotencyTracker::with_default_capacity();
    let ticket = make_ticket(10);
    assert!(tracker.mark_dispatched(&ticket));
}

#[test]
fn idempotency_tracker_mark_dispatched_duplicate_is_false() {
    let mut tracker = IdempotencyTracker::with_default_capacity();
    let ticket = make_ticket(10);
    assert_eq!(tracker.mark_completed(&ticket), Ok(()));
    assert!(!tracker.mark_dispatched(&ticket));
}

#[test]
fn idempotency_tracker_is_duplicate_completion_true_after_record() {
    let mut tracker = IdempotencyTracker::with_default_capacity();
    let ticket = make_ticket(55);
    assert!(!tracker.is_duplicate_completion(&ticket));
    assert_eq!(tracker.mark_completed(&ticket), Ok(()));
    assert!(tracker.is_duplicate_completion(&ticket));
}

#[test]
fn idempotency_tracker_eviction_oldest_removed() {
    let mut tracker = IdempotencyTracker::with_capacity(2);
    let ticket_a = make_ticket(1);
    let ticket_b = make_ticket(2);
    let ticket_c = make_ticket(3);

    assert_eq!(tracker.mark_completed(&ticket_a), Ok(()));
    assert_eq!(tracker.mark_completed(&ticket_b), Ok(()));
    // At capacity. Adding ticket_c should evict ticket_a.
    assert_eq!(tracker.mark_completed(&ticket_c), Ok(()));
    assert!(!tracker.is_completed(&ticket_a));
    assert!(tracker.is_completed(&ticket_b));
    assert!(tracker.is_completed(&ticket_c));
    assert_eq!(tracker.len(), 2);
}

#[test]
fn idempotency_tracker_capacity_one_evicts_on_second_insert() {
    let mut tracker = IdempotencyTracker::with_capacity(1);
    let ticket_a = make_ticket(10);
    let ticket_b = make_ticket(20);
    assert_eq!(tracker.mark_completed(&ticket_a), Ok(()));
    assert_eq!(tracker.mark_completed(&ticket_b), Ok(()));
    assert!(!tracker.is_completed(&ticket_a));
    assert!(tracker.is_completed(&ticket_b));
    assert_eq!(tracker.len(), 1);
}

// =====================================================================
// Policy-aware tracking (VB-REPLAY-002)
// =====================================================================

#[test]
fn policy_aware_tracking_deterministic_pure_skips_tracking() {
    // DeterministicPure: track_for_policy always returns true (new),
    // but is_completed_for_policy always returns false (not tracked).
    let mut tracker = IdempotencyTracker::with_default_capacity();
    let key = 100u128;

    assert!(tracker.track_for_policy(Idempotency::DeterministicPure, key));
    // NOT tracked — is_completed_for_policy returns false for DeterministicPure
    assert!(!tracker.is_completed_for_policy(Idempotency::DeterministicPure, key));
    // is_completed (general) also returns false since nothing was tracked
    let ticket = make_ticket(key);
    assert!(!tracker.is_completed(&ticket));
}

#[test]
fn policy_aware_tracking_idempotent_external_skips_tracking() {
    // IdempotentExternal: same as DeterministicPure — skip tracking.
    let mut tracker = IdempotencyTracker::with_default_capacity();
    let key = 200u128;

    assert!(tracker.track_for_policy(Idempotency::IdempotentExternal, key));
    assert!(!tracker.is_completed_for_policy(Idempotency::IdempotentExternal, key));
    let ticket = make_ticket(key);
    assert!(!tracker.is_completed(&ticket));
}

#[test]
fn policy_aware_tracking_at_least_once_external_tracks() {
    // AtLeastOnceExternal: track_for_policy records and deduplicates.
    let mut tracker = IdempotencyTracker::with_default_capacity();
    let key = 300u128;

    // First dispatch: new — returns true
    assert!(tracker.track_for_policy(Idempotency::AtLeastOnceExternal, key));
    // Second dispatch: duplicate — returns false
    assert!(!tracker.track_for_policy(Idempotency::AtLeastOnceExternal, key));
    // is_completed_for_policy: true after tracking
    assert!(tracker.is_completed_for_policy(Idempotency::AtLeastOnceExternal, key));
}

#[test]
fn policy_aware_tracking_at_least_once_mark_completed_updates_set() {
    // mark_completed_for_policy with AtLeastOnceExternal adds to the set.
    let mut tracker = IdempotencyTracker::with_default_capacity();
    let key = 400u128;

    assert_eq!(
        tracker.mark_completed_for_policy(Idempotency::AtLeastOnceExternal, key),
        Ok(())
    );
    assert!(tracker.is_completed_for_policy(Idempotency::AtLeastOnceExternal, key));
    // Duplicate completion: rejected
    assert_eq!(
        tracker.mark_completed_for_policy(Idempotency::AtLeastOnceExternal, key),
        Err(vb_core::action::ActionError::CompletionAlreadyRecorded)
    );
}

#[test]
fn policy_aware_tracking_different_policies_independent() {
    // Keys are tracked independently per policy class.
    let mut tracker = IdempotencyTracker::with_default_capacity();
    let key = 500u128;

    // DeterministicPure: not tracked
    assert!(tracker.track_for_policy(Idempotency::DeterministicPure, key));
    assert!(!tracker.is_completed_for_policy(Idempotency::DeterministicPure, key));

    // Same key, AtLeastOnceExternal: tracked separately
    assert!(tracker.track_for_policy(Idempotency::AtLeastOnceExternal, key));
    assert!(tracker.is_completed_for_policy(Idempotency::AtLeastOnceExternal, key));
    // DeterministicPure still not tracked
    assert!(!tracker.is_completed_for_policy(Idempotency::DeterministicPure, key));
}
