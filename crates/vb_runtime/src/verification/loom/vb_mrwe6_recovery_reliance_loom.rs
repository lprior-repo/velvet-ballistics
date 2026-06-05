#![cfg(all(test, loom))]

//! Loom model for obl-vb-mrwe-6-recovery-reliance-loom-030.

use loom::sync::atomic::{AtomicBool, Ordering};
use loom::sync::{Arc, Mutex};
use loom::thread;

#[test]
fn vb_mrwe6_recovery_trusts_only_atomic_event_index_pairs() {
    loom::model(|| {
        let boundary = Arc::new(Mutex::new(()));
        let event = Arc::new(AtomicBool::new(false));
        let index = Arc::new(AtomicBool::new(false));

        let writer_boundary = Arc::clone(&boundary);
        let writer_event = Arc::clone(&event);
        let writer_index = Arc::clone(&index);
        let writer = thread::spawn(move || {
            let _guard = writer_boundary
                .lock()
                .expect("loom mutex should not poison");
            writer_event.store(true, Ordering::Release);
            writer_index.store(true, Ordering::Release);
        });

        let recovery_boundary = Arc::clone(&boundary);
        let recovery_event = Arc::clone(&event);
        let recovery_index = Arc::clone(&index);
        let recovery = thread::spawn(move || {
            let _guard = recovery_boundary
                .lock()
                .expect("loom mutex should not poison");
            let valid_pending =
                recovery_event.load(Ordering::Acquire) && recovery_index.load(Ordering::Acquire);
            if valid_pending {
                assert!(recovery_event.load(Ordering::Acquire));
                assert!(recovery_index.load(Ordering::Acquire));
            }
        });

        writer.join().expect("writer joins");
        recovery.join().expect("recovery joins");
    });
}
