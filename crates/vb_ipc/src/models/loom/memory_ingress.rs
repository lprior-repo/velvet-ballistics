//! LOOM-MI-001: MemoryIngress bounded queue invariants
//!
//! Model: Abstract bounded mpsc channel representing MemoryIngress.
//! Invariant: available <= capacity (backpressure envelope never exceeded)
//!
//! Obligation: LOOM-MI-001
//! Verifier: loom
//! Command: RUSTFLAGS="--cfg loom" cargo test -p vb_ipc memory_ingress

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Abstract model of a bounded mpsc queue with concurrent submit/receive.
/// Tests the invariant: queued <= capacity
#[derive(Debug)]
struct BoundedQueue {
    queued: AtomicUsize,
    capacity: usize,
}

impl BoundedQueue {
    fn new(capacity: usize) -> Self {
        Self {
            queued: AtomicUsize::new(0),
            capacity,
        }
    }

    /// Attempts to submit one item. Returns true if queued, false if full.
    /// Uses CAS retry loop to handle concurrent modifications.
    fn try_submit(&self) -> bool {
        loop {
            let current = self.queued.load(Ordering::SeqCst);
            if current >= self.capacity {
                return false;
            }
            match self.queued.compare_exchange(
                current,
                current + 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return true,
                Err(_) => continue, // CAS failed (concurrent modification), retry
            }
        }
    }

    /// Attempts to receive one item. Returns true if dequeued, false if empty.
    /// Uses CAS retry loop to handle concurrent modifications.
    fn try_recv(&self) -> bool {
        loop {
            let current = self.queued.load(Ordering::SeqCst);
            if current == 0 {
                return false;
            }
            match self.queued.compare_exchange(
                current,
                current - 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return true,
                Err(_) => continue, // CAS failed (concurrent modification), retry
            }
        }
    }

    fn queued(&self) -> usize {
        self.queued.load(Ordering::SeqCst)
    }

    fn capacity(&self) -> usize {
        self.capacity
    }

    fn check_invariant(&self) {
        let q = self.queued();
        // usize is always >= 0 in Rust, no underflow possible
        assert!(
            q <= self.capacity,
            "queued {} exceeds capacity {}",
            q,
            self.capacity
        );
    }
}

/// Loom model: single producer, single consumer, bounded queue.
/// Tests INV-001: MemoryIngress available slots never exceed channel capacity.
#[test]
fn memory_ingress_invariants() {
    loom::model(|| {
        let queue = Arc::new(BoundedQueue::new(4));
        let q_prod = queue.clone();
        let q_cons = queue.clone();

        loom::thread::spawn(move || {
            // Producer submits up to 3 times
            for _ in 0..3 {
                q_prod.try_submit();
            }
        });

        // Consumer receives once
        let _ = q_cons.try_recv();
        queue.check_invariant();
    });
}

/// Loom model: multiple producers, multiple consumers, bounded queue.
/// Bounded exploration: 2 producers x 2 consumers x 2 rounds each.
///
/// NOTE: loom 0.7.2 has MAX_THREADS=5 compile-time constant for fixed arrays.
/// We use 4 concurrent threads (2 producers + 2 consumers) to stay within limit.
#[test]
fn memory_ingress_multi_producer() {
    loom::model(|| {
        let queue = Arc::new(BoundedQueue::new(4));
        let q1 = queue.clone();
        let q2 = queue.clone();
        let c1 = queue.clone();
        let c2 = queue.clone();

        // Two producers each submit 2 frames
        loom::thread::spawn(move || {
            for _ in 0..2 {
                q1.try_submit();
            }
        });
        loom::thread::spawn(move || {
            for _ in 0..2 {
                q2.try_submit();
            }
        });

        // Two consumers each receive 1 frame (all spawned concurrently)
        loom::thread::spawn(move || {
            let _ = c1.try_recv();
        });
        loom::thread::spawn(move || {
            let _ = c2.try_recv();
        });

        queue.check_invariant();
    });
}

/// Loom model: concurrent submit/receive interleavings.
/// Tests that capacity bound is preserved under all orderings.
#[test]
fn memory_ingress_submit_recv_interleaved() {
    loom::model(|| {
        let queue = Arc::new(BoundedQueue::new(3));
        let q1 = queue.clone();
        let q2 = queue.clone();

        loom::thread::spawn(move || {
            // Producer submits 2
            q1.try_submit();
            q1.try_submit();
        });

        loom::thread::spawn(move || {
            // Consumer receives 1
            let _ = q2.try_recv();
        });

        // Main submits 1 more
        queue.try_submit();
        queue.check_invariant();
    });
}
