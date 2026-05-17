//! VB-CONC-002: Action completion vs cancel race
//!
//! Model: ActionTicket and Action completion arriving while cancel is pending.
//! Invariant: exactly one of (completed, cancelled, pending) is true at any time.
//!
//! Obligation: VB-CONC-002
//! Verifier: loom
//! Command: RUSTFLAGS="--cfg loom" cargo test -p vb_runtime action_completion_cancel

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

enum ActionState {
    Pending,
    Completed,
    Cancelled,
}

struct ActionTicket {
    completed: AtomicBool,
    cancelled: AtomicBool,
}

impl ActionTicket {
    fn new() -> Self {
        Self {
            completed: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
        }
    }

    fn try_complete(&self) -> bool {
        !self.cancelled.load(Ordering::SeqCst) && !self.completed.swap(true, Ordering::SeqCst)
    }

    fn try_cancel(&self) -> bool {
        !self.completed.load(Ordering::SeqCst) && !self.cancelled.swap(true, Ordering::SeqCst)
    }

    fn is_completed(&self) -> bool {
        self.completed.load(Ordering::SeqCst)
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    fn check_invariants(&self) {
        let completed = self.is_completed();
        let cancelled = self.is_cancelled();
        assert!(
            !(completed && cancelled),
            "cannot be both completed and cancelled"
        );
    }
}

#[test]
fn action_completion_cancel_race() {
    loom::model(|| {
        let ticket = Arc::new(ActionTicket::new());
        let t1 = ticket.clone();
        let t2 = ticket.clone();

        loom::thread::spawn(move || {
            let _ = t1.try_complete();
        });

        let _ = t2.try_cancel();
        ticket.check_invariants();
    });
}

#[test]
fn action_completion_cancel_concurrent() {
    loom::model(|| {
        let ticket = Arc::new(ActionTicket::new());
        let t1 = ticket.clone();
        let t2 = ticket.clone();
        let t3 = ticket.clone();

        loom::thread::spawn(move || {
            let _ = t1.try_complete();
        });
        loom::thread::spawn(move || {
            let _ = t2.try_cancel();
        });
        loom::thread::spawn(move || {
            let _ = t3.try_complete();
        });

        ticket.check_invariants();
    });
}
