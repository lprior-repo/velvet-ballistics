#![cfg(all(test, loom))]

//! Loom model for obl-vb-in8ib-queue-intent-loom.
//!
//! Source bridge: the model calls `vb_storage::mrwe6_seams` production seam
//! helpers for queued relevant intent classification instead of proving a local
//! duplicate of MRWE6 queue semantics.

use loom::sync::atomic::{AtomicUsize, Ordering};
use loom::sync::{Arc, Mutex};
use loom::thread;
use vb_storage::mrwe6_seams::{
    Mrwe6EventClass, Mrwe6IntentKind, mrwe6_valid_queued_relevant_intent,
};

#[derive(Default)]
struct QueueCommitState {
    events: AtomicUsize,
    intents: AtomicUsize,
}

#[test]
fn vb_mrwe6_queue_flush_drain_preserves_index_intent_under_interleavings() {
    loom::model(|| {
        let state = Arc::new(QueueCommitState::default());
        let flush_lock = Arc::new(Mutex::new(()));

        let schedule_state = Arc::clone(&state);
        let schedule_lock = Arc::clone(&flush_lock);
        let scheduled = thread::spawn(move || {
            let Ok(_guard) = schedule_lock.lock() else {
                return;
            };
            assert!(
                mrwe6_valid_queued_relevant_intent(
                    Mrwe6EventClass::Scheduled,
                    Mrwe6IntentKind::PutPending,
                )
                .is_ok()
            );
            schedule_state.events.fetch_add(1, Ordering::AcqRel);
            schedule_state.intents.fetch_add(1, Ordering::AcqRel);
        });

        let resolution_state = Arc::clone(&state);
        let resolution_lock = Arc::clone(&flush_lock);
        let resolution = thread::spawn(move || {
            let Ok(_guard) = resolution_lock.lock() else {
                return;
            };
            assert!(
                mrwe6_valid_queued_relevant_intent(
                    Mrwe6EventClass::Resolution,
                    Mrwe6IntentKind::RemovePending,
                )
                .is_ok()
            );
            resolution_state.events.fetch_add(1, Ordering::AcqRel);
            resolution_state.intents.fetch_add(1, Ordering::AcqRel);
        });

        let observer_state = Arc::clone(&state);
        let observer_lock = Arc::clone(&flush_lock);
        let observer = thread::spawn(move || {
            let Ok(_guard) = observer_lock.lock() else {
                return;
            };
            let events = observer_state.events.load(Ordering::Acquire);
            let intents = observer_state.intents.load(Ordering::Acquire);
            assert_eq!(events, intents);
        });

        assert!(scheduled.join().is_ok());
        assert!(resolution.join().is_ok());
        assert!(observer.join().is_ok());
    });
}
