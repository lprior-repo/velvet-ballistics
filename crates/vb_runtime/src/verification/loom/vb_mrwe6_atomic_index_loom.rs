#![cfg(all(test, loom))]

//! Loom model for obl-vb-mrwe-6-atomic-index-loom-006.

use loom::sync::atomic::{AtomicBool, Ordering};
use loom::sync::{Arc, Mutex};
use loom::thread;

#[derive(Default)]
struct AtomState {
    event: AtomicBool,
    index: AtomicBool,
}

#[test]
fn vb_mrwe6_atomic_index_no_partial_commit_under_interleavings() {
    loom::model(|| {
        let state = Arc::new(AtomState::default());
        let boundary = Arc::new(Mutex::new(()));

        let writer_state = Arc::clone(&state);
        let writer_boundary = Arc::clone(&boundary);
        let writer = thread::spawn(move || {
            let _guard = writer_boundary
                .lock()
                .expect("loom mutex should not poison");
            writer_state.event.store(true, Ordering::Release);
            writer_state.index.store(true, Ordering::Release);
        });

        let observer_state = Arc::clone(&state);
        let observer_boundary = Arc::clone(&boundary);
        let observer = thread::spawn(move || {
            let _guard = observer_boundary
                .lock()
                .expect("loom mutex should not poison");
            let event = observer_state.event.load(Ordering::Acquire);
            let index = observer_state.index.load(Ordering::Acquire);
            assert_eq!(event, index);
        });

        writer.join().expect("writer thread joins");
        observer.join().expect("observer thread joins");
    });
}
