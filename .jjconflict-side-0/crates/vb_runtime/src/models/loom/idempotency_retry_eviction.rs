//! vb-ko29.7 LOOM-IDEMPOTENCY-001/002: idempotency retry collision and
//! eviction/conflict interleavings.
//!
//! Verifier: loom.
//! Command: `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime --lib models::loom::idempotency_retry_eviction -- --nocapture`.
//!
//! This model intentionally uses local sync indirection (`crate::models::sync`)
//! around the non-concurrent `IdempotencyTracker`.  The production tracker is a
//! synchronous data structure; callers must serialize access.  The model checks
//! that every serialized interleaving of retry admission, completion, duplicate
//! completion, and FIFO eviction resolves to an explicit local lattice outcome.

#[cfg(test)]
use crate::idempotency::IdempotencyTracker;
#[cfg(test)]
use crate::models::sync::sync::{Arc, Mutex, MutexGuard, Ordering, thread};
#[cfg(test)]
use loom::sync::atomic::AtomicUsize;
#[cfg(test)]
use vb_core::action::{ActionError, ActionTicket, Idempotency};
#[cfg(test)]
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

#[cfg(test)]
fn lock_tracker(tracker: &Mutex<IdempotencyTracker>) -> MutexGuard<'_, IdempotencyTracker> {
    match tracker.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
fn ticket(run: u64, seq: u64, key: u128) -> ActionTicket {
    ActionTicket {
        run: RunId::new(run),
        step: StepIdx::new(0),
        seq: SeqNo::new(seq),
        action: ActionId::new(7),
        attempt: 1,
        idempotency_key: key,
        capacity: 1,
    }
}

#[cfg(test)]
fn increment(counter: &AtomicUsize) {
    let _previous = counter.fetch_add(1, Ordering::SeqCst);
}

#[cfg(test)]
fn check_bounded_model(model: impl Fn() + Sync + Send + 'static) {
    let mut builder = loom::model::Builder::new();
    builder.max_branches = 1000;
    builder.preemption_bound = Some(3);
    builder.check(model);
}

/// LOOM-IDEMPOTENCY-001: two same-scope retry admissions race with an
/// independently serialized completion for the same idempotency key.
#[test]
fn same_scope_retry_admission_completion_collision_is_single_winner() {
    check_bounded_model(|| {
        let tracker = Arc::new(Mutex::new(IdempotencyTracker::with_capacity(2)));
        let admitted = Arc::new(AtomicUsize::new(0));
        let denied = Arc::new(AtomicUsize::new(0));
        let completion_ok = Arc::new(AtomicUsize::new(0));
        let completion_duplicate = Arc::new(AtomicUsize::new(0));
        let key = 0xA11CEu128;

        let mut handles = Vec::new();
        for seq in 1..=2 {
            let retry_tracker = tracker.clone();
            let retry_admitted = admitted.clone();
            let retry_denied = denied.clone();
            let retry_completion_ok = completion_ok.clone();
            let retry_completion_duplicate = completion_duplicate.clone();
            handles.push(thread::spawn(move || {
                let mut locked = lock_tracker(&retry_tracker);
                if locked.track_for_policy(Idempotency::AtLeastOnceExternal, key) {
                    increment(&retry_admitted);
                    match locked.mark_completed(&ticket(1, seq, key)) {
                        Ok(()) => increment(&retry_completion_ok),
                        Err(ActionError::CompletionAlreadyRecorded) => {
                            increment(&retry_completion_duplicate);
                        }
                        Err(_) => increment(&retry_completion_duplicate),
                    }
                } else {
                    increment(&retry_denied);
                }
            }));
        }

        let completion_tracker = tracker.clone();
        let independent_completion_ok = completion_ok.clone();
        let independent_completion_duplicate = completion_duplicate.clone();
        handles.push(thread::spawn(move || {
            let mut locked = lock_tracker(&completion_tracker);
            match locked.mark_completed(&ticket(1, 99, key)) {
                Ok(()) => increment(&independent_completion_ok),
                Err(ActionError::CompletionAlreadyRecorded) => {
                    increment(&independent_completion_duplicate);
                }
                Err(_) => increment(&independent_completion_duplicate),
            }
        }));

        for handle in handles {
            assert!(handle.join().is_ok(), "loom worker should complete");
        }

        let locked = lock_tracker(&tracker);
        let admitted_count = admitted.load(Ordering::SeqCst);
        let denied_count = denied.load(Ordering::SeqCst);
        let completion_ok_count = completion_ok.load(Ordering::SeqCst);
        let duplicate_count = completion_duplicate.load(Ordering::SeqCst);

        assert!(
            admitted_count <= 1,
            "same-scope retry admits at most one caller"
        );
        assert_eq!(
            admitted_count + denied_count,
            2,
            "both retry attempts resolve"
        );
        assert!(
            completion_ok_count <= 1,
            "completed map records at most one winner"
        );
        assert_eq!(
            locked.len(),
            completion_ok_count,
            "tracker length mirrors completion winner count"
        );
        assert_eq!(
            completion_ok_count + duplicate_count,
            admitted_count + 1,
            "every completion attempt resolves"
        );
        assert!(
            locked.is_completed(&ticket(1, 0, key)),
            "winning completion is visible by key"
        );
    });
}

/// LOOM-IDEMPOTENCY-002: a duplicate completion for an already tracked key races
/// a capacity-one FIFO eviction caused by a different key.
#[test]
fn eviction_conflict_interleaving_preserves_capacity_and_explicit_outcome() {
    check_bounded_model(|| {
        let old_key = 0xE71C7u128;
        let new_key = 0xC0A11DEu128;
        let tracker = Arc::new(Mutex::new(IdempotencyTracker::with_capacity(1)));
        {
            let mut locked = lock_tracker(&tracker);
            assert_eq!(locked.mark_completed(&ticket(1, 1, old_key)), Ok(()));
        }

        let evictions = Arc::new(AtomicUsize::new(0));
        let conflict_rejected = Arc::new(AtomicUsize::new(0));
        let conflict_accepted_after_eviction = Arc::new(AtomicUsize::new(0));

        let evict_tracker = tracker.clone();
        let evict_count = evictions.clone();
        let evict = thread::spawn(move || {
            let mut locked = lock_tracker(&evict_tracker);
            if locked.mark_completed(&ticket(1, 2, new_key)).is_ok() {
                increment(&evict_count);
            }
        });

        let conflict_tracker = tracker.clone();
        let rejected = conflict_rejected.clone();
        let accepted_after_eviction = conflict_accepted_after_eviction.clone();
        let conflict = thread::spawn(move || {
            let mut locked = lock_tracker(&conflict_tracker);
            match locked.mark_completed(&ticket(1, 3, old_key)) {
                Ok(()) => increment(&accepted_after_eviction),
                Err(ActionError::CompletionAlreadyRecorded) => increment(&rejected),
                Err(_) => increment(&rejected),
            }
        });

        assert!(evict.join().is_ok(), "eviction worker should complete");
        assert!(conflict.join().is_ok(), "conflict worker should complete");

        let locked = lock_tracker(&tracker);
        let eviction_count = evictions.load(Ordering::SeqCst);
        let rejected_count = conflict_rejected.load(Ordering::SeqCst);
        let accepted_count = conflict_accepted_after_eviction.load(Ordering::SeqCst);
        let old_present = locked.is_completed(&ticket(1, 0, old_key));
        let new_present = locked.is_completed(&ticket(1, 0, new_key));

        assert_eq!(eviction_count, 1, "new key insertion must complete once");
        assert_eq!(
            rejected_count + accepted_count,
            1,
            "old-key conflict resolves once"
        );
        assert!(
            locked.len() <= 1,
            "capacity-one tracker never exceeds capacity"
        );
        assert_ne!(
            old_present, new_present,
            "capacity-one tracker has exactly one visible key"
        );
        if accepted_count == 1 {
            assert!(
                old_present,
                "accepted stale key must be the visible survivor"
            );
            assert!(
                !new_present,
                "accepted stale key evicts the newer key locally"
            );
        } else {
            assert!(new_present, "rejected conflict leaves newer key visible");
            assert!(
                !old_present,
                "rejected old key remains absent after eviction"
            );
        }
    });
}
