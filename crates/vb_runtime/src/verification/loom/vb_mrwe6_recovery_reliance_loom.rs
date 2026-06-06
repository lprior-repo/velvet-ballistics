#![cfg(all(test, loom))]

//! Loom model for obl-vb-in8ib-recovery-loom.
//!
//! Source bridge: recovery observations are classified through
//! `vb_storage::mrwe6_seams::mrwe6_recovery_outcome_from_facts`, the same pure
//! production MRWE6 kernel surface used by storage recovery seams.

use loom::sync::atomic::{AtomicBool, Ordering};
use loom::sync::{Arc, Mutex};
use loom::thread;
use vb_storage::mrwe6_seams::{Mrwe6RecoveryOutcome, mrwe6_recovery_outcome_from_facts};

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
            let Ok(_guard) = writer_boundary.lock() else {
                return;
            };
            writer_event.store(true, Ordering::Release);
            writer_index.store(true, Ordering::Release);
        });

        let recovery_boundary = Arc::clone(&boundary);
        let recovery_event = Arc::clone(&event);
        let recovery_index = Arc::clone(&index);
        let recovery = thread::spawn(move || {
            let Ok(_guard) = recovery_boundary.lock() else {
                return;
            };
            let event_present = recovery_event.load(Ordering::Acquire);
            let marker_present = recovery_index.load(Ordering::Acquire);
            let outcome = mrwe6_recovery_outcome_from_facts(
                event_present,
                false,
                false,
                marker_present,
                false,
            );
            if outcome == Mrwe6RecoveryOutcome::PendingInventory {
                assert!(event_present);
                assert!(marker_present);
            }
        });

        assert!(writer.join().is_ok());
        assert!(recovery.join().is_ok());
    });
}
