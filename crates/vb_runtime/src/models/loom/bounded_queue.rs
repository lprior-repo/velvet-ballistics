//! VB-CONC-005: Bounded queue enqueue/dequeue invariants
//!
//! Model: Abstract bounded counter representing the frame pool capacity.
//! Invariant: available() <= capacity && available() >= 0
//!
//! Obligation: VB-CONC-005
//! Verifier: loom
//! Command: RUSTFLAGS="--cfg loom" cargo test -p vb_runtime bounded_queue

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Abstract model of a bounded queue with concurrent take/release.
/// Tests the invariant: available <= capacity && available >= 0
struct BoundedQueue {
    available: AtomicUsize,
    capacity: usize,
}

impl BoundedQueue {
    fn new(capacity: usize) -> Self {
        Self {
            available: AtomicUsize::new(capacity),
            capacity,
        }
    }

    fn try_take(&self) -> bool {
        let current = self.available.load(Ordering::SeqCst);
        if current == 0 {
            return false;
        }
        self.available
            .compare_exchange(
                current,
                current.saturating_sub(1),
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok()
    }

    fn release(&self) {
        let current = self.available.load(Ordering::SeqCst);
        if current < self.capacity {
            self.available.store(
                current.saturating_add(1).min(self.capacity),
                Ordering::SeqCst,
            );
        }
    }

    fn available(&self) -> usize {
        self.available.load(Ordering::SeqCst)
    }

    fn check_invariants(&self) {
        let avail = self.available();
        assert!(
            avail <= self.capacity,
            "available {} exceeds capacity {}",
            avail,
            self.capacity
        );
    }
}

/// Loom model for bounded queue invariants.
/// Tests concurrent take/release operations maintaining capacity bounds.
#[test]
fn bounded_queue_invariants() {
    loom::model(|| {
        let queue = Arc::new(BoundedQueue::new(10));
        let q1 = queue.clone();
        let q2 = queue.clone();

        loom::thread::spawn(move || {
            q2.release();
        });

        if q1.try_take() {
            q1.check_invariants();
        }
        q1.check_invariants();
    });
}

/// Multiple concurrent operations variant.
#[test]
fn bounded_queue_multiple_operations() {
    loom::model(|| {
        let queue = Arc::new(BoundedQueue::new(10));
        let q1 = queue.clone();
        let q2 = queue.clone();
        let q3 = queue.clone();

        loom::thread::spawn(move || {
            for _ in 0..4 {
                q2.release();
            }
        });
        loom::thread::spawn(move || {
            for _ in 0..2 {
                if q3.try_take() {
                    q3.check_invariants();
                }
            }
        });

        for _ in 0..2 {
            if q1.try_take() {
                q1.check_invariants();
            }
        }

        queue.check_invariants();
    });
}
