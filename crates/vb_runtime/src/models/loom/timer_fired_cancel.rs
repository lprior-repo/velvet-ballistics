//! VB-CONC-003: Timer fired vs cancel/replace/terminal ordering.
//!
//! Obligations: PO-007.
//! Verifier: loom.
//! Command: `cargo xtask loom --model timer_fired_cancel`.
//!
//! This is a verification-only model of captured `TimerFired` delivery.  It is
//! intentionally smaller than the production shard, but it models the authority
//! tuple that matters for stale-fire rejection: `(run, generation, deadline,
//! kind)` plus terminal lifecycle state.

#[cfg(test)]
use loom::sync::{Arc, Mutex, MutexGuard};

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TimerKind {
    Wait,
    Ask,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PendingTimer {
    generation: usize,
    deadline: usize,
    kind: TimerKind,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CapturedFire {
    generation: usize,
    deadline: usize,
    kind: TimerKind,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FireOutcome {
    ValidDelivered,
    StaleAfterCancel,
    StaleAfterReplace,
    TerminalRejected,
}

#[cfg(test)]
#[derive(Debug)]
struct ModelState {
    pending: Option<PendingTimer>,
    terminal: bool,
    delivered: usize,
    invalid: usize,
    valid: usize,
}

#[cfg(test)]
fn lock_state(state: &Mutex<ModelState>) -> MutexGuard<'_, ModelState> {
    match state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
fn deliver(state: &mut ModelState, event: CapturedFire) -> FireOutcome {
    if state.terminal {
        state.invalid += 1;
        return FireOutcome::TerminalRejected;
    }

    match state.pending {
        Some(timer)
            if timer.generation == event.generation
                && timer.deadline == event.deadline
                && timer.kind == event.kind =>
        {
            state.pending = None;
            state.delivered += 1;
            state.valid += 1;
            FireOutcome::ValidDelivered
        }
        Some(_) => {
            state.invalid += 1;
            FireOutcome::StaleAfterReplace
        }
        None => {
            state.invalid += 1;
            FireOutcome::StaleAfterCancel
        }
    }
}

#[cfg(test)]
fn initial_state() -> ModelState {
    ModelState {
        pending: Some(PendingTimer {
            generation: 1,
            deadline: 10,
            kind: TimerKind::Wait,
        }),
        terminal: false,
        delivered: 0,
        invalid: 0,
        valid: 0,
    }
}

#[cfg(test)]
fn captured_fire() -> CapturedFire {
    CapturedFire {
        generation: 1,
        deadline: 10,
        kind: TimerKind::Wait,
    }
}

#[cfg(test)]
fn assert_lattice(state: &ModelState, outcome: FireOutcome) {
    assert!(
        state.valid <= 1,
        "captured timer may be valid-delivered at most once"
    );
    assert!(
        state.delivered <= 1,
        "delivery mutation must be single-shot"
    );
    assert!(
        state.invalid <= 1,
        "one captured event yields one rejection at most"
    );
    assert!(
        state.valid + state.invalid == 1,
        "captured event must resolve to exactly one lattice outcome"
    );

    match outcome {
        FireOutcome::ValidDelivered => {
            assert_ne!(
                state.pending,
                Some(PendingTimer {
                    generation: 1,
                    deadline: 10,
                    kind: TimerKind::Wait,
                }),
                "valid delivery must consume the captured timer authority"
            );
            assert_eq!(state.valid, 1, "valid delivery records valid branch");
        }
        FireOutcome::StaleAfterCancel => {
            assert_eq!(state.pending, None, "cancelled timer stays absent");
            assert_eq!(state.valid, 0, "stale cancel cannot valid-deliver");
        }
        FireOutcome::StaleAfterReplace => {
            assert_eq!(
                state.pending,
                Some(PendingTimer {
                    generation: 2,
                    deadline: 11,
                    kind: TimerKind::Ask,
                }),
                "replacement timer must remain authoritative"
            );
            assert_eq!(state.valid, 0, "stale replacement cannot valid-deliver");
        }
        FireOutcome::TerminalRejected => {
            assert!(state.terminal, "terminal branch requires terminal state");
            assert_eq!(
                state.pending, None,
                "terminal state cannot resurrect a timer"
            );
            assert_eq!(state.valid, 0, "terminal stale fire cannot valid-deliver");
        }
    }
}

/// Captured fired event versus cancel: either delivery wins and consumes the
/// timer, or cancel wins and delivery is explicitly stale-after-cancel.
#[test]
fn timer_fired_cancel_ordering() {
    loom::model::Builder::new()
        .max_preemptions(3)
        .max_branches(1000)
        .check(|| {
        let state = Arc::new(Mutex::new(initial_state()));
        let event = captured_fire();

        let cancel_state = state.clone();
        let cancel = loom::thread::spawn(move || {
            let mut locked = lock_state(&cancel_state);
            locked.pending = None;
        });

        let deliver_state = state.clone();
        let delivery = loom::thread::spawn(move || {
            let mut locked = lock_state(&deliver_state);
            deliver(&mut locked, event)
        });

        assert!(cancel.join().is_ok(), "cancel thread should complete");
        let delivery_result = delivery.join();
        assert!(delivery_result.is_ok(), "delivery thread should complete");
        let Ok(outcome) = delivery_result else {
            return;
        };

        let locked = lock_state(&state);
        assert!(
            matches!(
                outcome,
                FireOutcome::ValidDelivered | FireOutcome::StaleAfterCancel
            ),
            "cancel race must resolve only to valid delivery or stale-after-cancel"
        );
        assert_lattice(&locked, outcome);
    });
}

/// Captured fired event versus replacement: either the captured event wins
/// before replacement, or the replacement wins and stale metadata is rejected.
#[test]
fn timer_fired_replace_ordering() {
    loom::model::Builder::new()
        .max_preemptions(3)
        .max_branches(1000)
        .check(|| {
        let state = Arc::new(Mutex::new(initial_state()));
        let event = captured_fire();

        let replace_state = state.clone();
        let replace = loom::thread::spawn(move || {
            let mut locked = lock_state(&replace_state);
            locked.pending = Some(PendingTimer {
                generation: 2,
                deadline: 11,
                kind: TimerKind::Ask,
            });
        });

        let deliver_state = state.clone();
        let delivery = loom::thread::spawn(move || {
            let mut locked = lock_state(&deliver_state);
            deliver(&mut locked, event)
        });

        assert!(replace.join().is_ok(), "replace thread should complete");
        let delivery_result = delivery.join();
        assert!(delivery_result.is_ok(), "delivery thread should complete");
        let Ok(outcome) = delivery_result else {
            return;
        };

        let locked = lock_state(&state);
        assert!(
            matches!(
                outcome,
                FireOutcome::ValidDelivered | FireOutcome::StaleAfterReplace
            ),
            "replace race must resolve only to valid delivery or stale-after-replace"
        );
        assert_lattice(&locked, outcome);
    });
}

/// Captured fired event versus terminal lifecycle transition: either delivery
/// wins before terminalization, or terminalization wins and stale delivery is
/// explicitly rejected with no resurrection.
#[test]
fn timer_fired_terminal_ordering() {
    loom::model::Builder::new()
        .max_preemptions(3)
        .max_branches(1000)
        .check(|| {
        let state = Arc::new(Mutex::new(initial_state()));
        let event = captured_fire();

        let terminal_state = state.clone();
        let terminal = loom::thread::spawn(move || {
            let mut locked = lock_state(&terminal_state);
            locked.terminal = true;
            locked.pending = None;
        });

        let deliver_state = state.clone();
        let delivery = loom::thread::spawn(move || {
            let mut locked = lock_state(&deliver_state);
            deliver(&mut locked, event)
        });

        assert!(terminal.join().is_ok(), "terminal thread should complete");
        let delivery_result = delivery.join();
        assert!(delivery_result.is_ok(), "delivery thread should complete");
        let Ok(outcome) = delivery_result else {
            return;
        };

        let locked = lock_state(&state);
        assert!(
            matches!(
                outcome,
                FireOutcome::ValidDelivered | FireOutcome::TerminalRejected
            ),
            "terminal race must resolve only to valid delivery or terminal rejection"
        );
        assert_lattice(&locked, outcome);
    });
}
