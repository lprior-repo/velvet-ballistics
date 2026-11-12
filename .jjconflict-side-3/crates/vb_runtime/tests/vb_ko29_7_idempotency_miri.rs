//! vb-ko29.7 MIRI-IDEMPOTENCY-001: representative Miri exercise for safe
//! idempotency tracker data-structure paths.
//!
//! Verifier: Miri.
//! Command: `cargo +nightly miri test -p vb_runtime --test vb_ko29_7_idempotency_miri -- --nocapture`.

#![cfg(miri)]

use vb_core::action::{ActionError, ActionTicket, Idempotency};
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
use vb_runtime::idempotency::IdempotencyTracker;

fn ticket(seq: u64, key: u128) -> ActionTicket {
    ActionTicket {
        run: RunId::new(29),
        step: StepIdx::new(7),
        seq: SeqNo::new(seq),
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: key,
        capacity: 1,
    }
}

#[test]
fn miri_idempotency_tracker_retry_collision_and_eviction_paths_no_ub() {
    let mut tracker = IdempotencyTracker::with_capacity(1);
    let key_a = 0xAAu128;
    let key_b = 0xBBu128;

    assert!(tracker.track_for_policy(Idempotency::AtLeastOnceExternal, key_a));
    assert!(!tracker.track_for_policy(Idempotency::AtLeastOnceExternal, key_a));
    assert!(tracker.is_completed_for_policy(Idempotency::AtLeastOnceExternal, key_a));

    assert_eq!(tracker.mark_completed(&ticket(1, key_a)), Ok(()));
    assert_eq!(
        tracker.mark_completed(&ticket(2, key_a)),
        Err(ActionError::CompletionAlreadyRecorded)
    );

    assert_eq!(tracker.mark_completed(&ticket(3, key_b)), Ok(()));
    assert!(!tracker.is_completed(&ticket(4, key_a)));
    assert!(tracker.is_completed(&ticket(5, key_b)));
    assert_eq!(tracker.len(), 1);
}
