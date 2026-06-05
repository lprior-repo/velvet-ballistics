#![cfg(all(test, loom))]

//! Loom model for obl-vb-mrwe-6-completion-policy-loom-024.

use loom::sync::atomic::{AtomicBool, Ordering};
use loom::sync::{Arc, Mutex};
use loom::thread;

#[test]
fn vb_mrwe6_resolution_removes_pending_under_interleavings() {
    loom::model(|| {
        let boundary = Arc::new(Mutex::new(()));
        let pending = Arc::new(AtomicBool::new(true));
        let resolution = Arc::new(AtomicBool::new(false));

        let resolver_boundary = Arc::clone(&boundary);
        let resolver_pending = Arc::clone(&pending);
        let resolver_resolution = Arc::clone(&resolution);
        let resolver = thread::spawn(move || {
            let _guard = resolver_boundary
                .lock()
                .expect("loom mutex should not poison");
            resolver_resolution.store(true, Ordering::Release);
            resolver_pending.store(false, Ordering::Release);
        });

        let observer_boundary = Arc::clone(&boundary);
        let observer_pending = Arc::clone(&pending);
        let observer_resolution = Arc::clone(&resolution);
        let observer = thread::spawn(move || {
            let _guard = observer_boundary
                .lock()
                .expect("loom mutex should not poison");
            let seen_resolution = observer_resolution.load(Ordering::Acquire);
            let seen_pending = observer_pending.load(Ordering::Acquire);
            assert!(!seen_resolution || !seen_pending);
        });

        resolver.join().expect("resolver joins");
        observer.join().expect("observer joins");
    });
}
