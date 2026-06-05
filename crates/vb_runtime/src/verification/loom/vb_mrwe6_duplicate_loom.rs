#![cfg(all(test, loom))]

//! Loom model for obl-vb-mrwe-6-duplicate-loom-018.

use loom::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use loom::sync::{Arc, Mutex};
use loom::thread;

#[test]
fn vb_mrwe6_duplicate_retries_preserve_index_under_interleavings() {
    loom::model(|| {
        let boundary = Arc::new(Mutex::new(()));
        let marker = Arc::new(AtomicBool::new(true));
        let conflicts = Arc::new(AtomicUsize::new(0));

        let equal_boundary = Arc::clone(&boundary);
        let equal_marker = Arc::clone(&marker);
        let equal = thread::spawn(move || {
            let _guard = equal_boundary.lock().expect("loom mutex should not poison");
            assert!(equal_marker.load(Ordering::Acquire));
        });

        let divergent_boundary = Arc::clone(&boundary);
        let divergent_marker = Arc::clone(&marker);
        let divergent_conflicts = Arc::clone(&conflicts);
        let divergent = thread::spawn(move || {
            let _guard = divergent_boundary
                .lock()
                .expect("loom mutex should not poison");
            if divergent_marker.load(Ordering::Acquire) {
                divergent_conflicts.fetch_add(1, Ordering::AcqRel);
            }
        });

        equal.join().expect("equal retry joins");
        divergent.join().expect("divergent retry joins");
        assert!(marker.load(Ordering::Acquire));
        assert!(conflicts.load(Ordering::Acquire) <= 1);
    });
}
