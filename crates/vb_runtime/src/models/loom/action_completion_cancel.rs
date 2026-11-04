//! VB-CONC-002: Action completion vs cancel race
//!
//! Model: ActionTicket and Action completion arriving while cancel is pending.
//! Invariant: exactly one of (completed, cancelled, pending) is true at any time.
//!
//! Obligation: VB-CONC-002
//! Verifier: loom
//! Command: RUSTFLAGS="--cfg loom" cargo test -p vb_runtime action_completion_cancel

use crate::models::sync::sync::{Arc, AtomicUsize, Ordering};

/// Single-word ticket state so completion and cancellation resolve through
/// one atomic transition. The two-flag check-then-act form allowed both
/// `try_complete` and `try_cancel` to succeed under interleaving; loom
/// exposes that race once the model uses loom atomics.
const TICKET_PENDING: usize = 0;
const TICKET_COMPLETED: usize = 1;
const TICKET_CANCELLED: usize = 2;

struct ActionTicket {
    state: AtomicUsize,
}

impl ActionTicket {
    fn new() -> Self {
        Self {
            state: AtomicUsize::new(TICKET_PENDING),
        }
    }

    fn try_complete(&self) -> bool {
        self.state
            .compare_exchange(
                TICKET_PENDING,
                TICKET_COMPLETED,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok()
    }

    fn try_cancel(&self) -> bool {
        self.state
            .compare_exchange(
                TICKET_PENDING,
                TICKET_CANCELLED,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok()
    }

    fn is_completed(&self) -> bool {
        self.state.load(Ordering::SeqCst) == TICKET_COMPLETED
    }

    fn is_cancelled(&self) -> bool {
        self.state.load(Ordering::SeqCst) == TICKET_CANCELLED
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
            if t1.try_complete() {
                t1.check_invariants();
            }
        });

        if t2.try_cancel() {
            t2.check_invariants();
        }
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
            if t1.try_complete() {
                t1.check_invariants();
            }
        });
        loom::thread::spawn(move || {
            if t2.try_cancel() {
                t2.check_invariants();
            }
        });
        loom::thread::spawn(move || {
            if t3.try_complete() {
                t3.check_invariants();
            }
        });

        ticket.check_invariants();
    });
}
