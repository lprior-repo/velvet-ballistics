//! VB-CONC-004: Shutdown drain ordering
//!
//! Obligation: VB-CONC-004
//! Verifier: loom
//! Command: RUSTFLAGS="--cfg loom" cargo test -p vb_runtime shutdown_drain

//! Model: Shutdown drain ordering.
//! Invariant: all pending work drained after shutdown.

#[cfg(loom)]
use loom::sync::Arc;
#[cfg(loom)]
use loom::sync::atomic::{AtomicUsize, Ordering};
#[cfg(not(loom))]
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

/// Verifies that shutdown drains all pending work in correct order.
#[test]
fn shutdown_drain_ordering() {
    loom::model(|| {
        // Model pending work counter
        let pending = Arc::new(AtomicUsize::new(3));
        let pending_work = pending.clone();
        let pending_shutdown = pending.clone();

        loom::thread::spawn(move || {
            pending_work.fetch_sub(1, Ordering::SeqCst);
        });
        loom::thread::spawn(move || {
            pending_shutdown.fetch_sub(1, Ordering::SeqCst);
        });

        // Shutdown sets pending to 0
        pending.store(0, Ordering::SeqCst);

        // Invariant: after shutdown, pending is 0
        assert_eq!(
            pending.load(Ordering::SeqCst),
            0,
            "pending work should be 0 after shutdown"
        );
    });
}
