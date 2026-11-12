//! VB-CONC-001: Journal writer queue concurrent append/drain
//!
//! Model: JournalWriterQueue with concurrent append from action threads
//! and drain from journal writer thread.
//! Invariant: queue length never exceeds configured capacity.
//!
//! Obligation: VB-CONC-001
//! Verifier: loom
//! Command: RUSTFLAGS="--cfg loom" cargo test -p vb_runtime journal_writer_queue

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct JournalWriterQueue {
    pending: AtomicUsize,
    capacity: usize,
}

impl JournalWriterQueue {
    fn new(capacity: usize) -> Self {
        Self {
            pending: AtomicUsize::new(0),
            capacity,
        }
    }

    fn try_append(&self) -> bool {
        let current = self.pending.load(Ordering::SeqCst);
        if current >= self.capacity {
            return false;
        }
        self.pending
            .compare_exchange(current, current + 1, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    fn drain(&self, count: usize) {
        let current = self.pending.load(Ordering::SeqCst);
        self.pending
            .store(current.saturating_sub(count), Ordering::SeqCst);
    }

    fn pending(&self) -> usize {
        self.pending.load(Ordering::SeqCst)
    }

    fn check_invariants(&self) {
        let pending = self.pending();
        assert!(
            pending <= self.capacity,
            "pending {} exceeds capacity {}",
            pending,
            self.capacity
        );
    }
}

#[test]
fn journal_writer_queue_append_drain() {
    loom::model(|| {
        let queue = Arc::new(JournalWriterQueue::new(100));
        let q1 = queue.clone();
        let q2 = queue.clone();

        loom::thread::spawn(move || {
            q2.drain(1);
        });

        if q1.try_append() {
            q1.check_invariants();
        }
        q1.check_invariants();
    });
}

#[test]
fn journal_writer_queue_concurrent_append() {
    loom::model(|| {
        let queue = Arc::new(JournalWriterQueue::new(10));
        let q1 = queue.clone();
        let q2 = queue.clone();
        let q3 = queue.clone();

        loom::thread::spawn(move || {
            for _ in 0..3 {
                if q1.try_append() {
                    q1.check_invariants();
                }
            }
        });
        loom::thread::spawn(move || {
            for _ in 0..2 {
                if q2.try_append() {
                    q2.check_invariants();
                }
            }
        });

        for _ in 0..5 {
            if q3.try_append() {
                q3.check_invariants();
            }
        }

        queue.check_invariants();
    });
}

#[test]
fn journal_writer_queue_at_capacity() {
    loom::model(|| {
        let queue = Arc::new(JournalWriterQueue::new(2));
        let q1 = queue.clone();
        let q2 = queue.clone();

        loom::thread::spawn(move || {
            for _ in 0..2 {
                if q1.try_append() {
                    q1.check_invariants();
                }
            }
        });
        loom::thread::spawn(move || {
            if q2.try_append() {
                q2.check_invariants();
            }
            if q2.try_append() {
                q2.check_invariants();
            }
        });

        queue.check_invariants();
    });
}
