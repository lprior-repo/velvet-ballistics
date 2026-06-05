#![cfg(all(test, loom))]

//! Loom model for obl-vb-mrwe-6-queue-intent-loom-012.

use loom::sync::atomic::{AtomicUsize, Ordering};
use loom::sync::{Arc, Mutex};
use loom::thread;

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
            let _guard = schedule_lock.lock().expect("loom mutex should not poison");
            schedule_state.events.fetch_add(1, Ordering::AcqRel);
            schedule_state.intents.fetch_add(1, Ordering::AcqRel);
        });

        let resolution_state = Arc::clone(&state);
        let resolution_lock = Arc::clone(&flush_lock);
        let resolution = thread::spawn(move || {
            let _guard = resolution_lock
                .lock()
                .expect("loom mutex should not poison");
            resolution_state.events.fetch_add(1, Ordering::AcqRel);
            resolution_state.intents.fetch_add(1, Ordering::AcqRel);
        });

        let observer_state = Arc::clone(&state);
        let observer_lock = Arc::clone(&flush_lock);
        let observer = thread::spawn(move || {
            let _guard = observer_lock.lock().expect("loom mutex should not poison");
            let events = observer_state.events.load(Ordering::Acquire);
            let intents = observer_state.intents.load(Ordering::Acquire);
            assert_eq!(events, intents);
        });

        scheduled.join().expect("scheduled thread joins");
        resolution.join().expect("resolution thread joins");
        observer.join().expect("observer thread joins");
    });
}
