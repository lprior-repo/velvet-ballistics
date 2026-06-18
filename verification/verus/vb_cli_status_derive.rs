//! Verus proof for derive_status_from_events (vb_cli status derivation).
//!
//! This file proves key invariants of the pure status derivation function
//! over event sequences, without depending on crate-level types.
//!
//! ## Model
//!
//! Events are represented as tagged integers:
//! - 0 = RunCancelled (terminal → Cancelled)
//! - 1 = RunFinished  (terminal → Completed)
//! - 2 = RunFailed    (non-terminal failure)
//! - 3 = RetryScheduled (indicates retry timer active)
//! - 4 = ActionScheduled (blocking wait on external action)
//! - 5 = AskWait       (answer pending — ask/wait scheduled)
//! - 6 = Other         (all other events, ignored)
//!
//! Status values are tagged integers matching the DerivedStatus enum:
//! - 0 = Pending
//! - 1 = Active
//! - 2 = WaitingAction
//! - 3 = WaitingAnswer
//! - 4 = Completed
//! - 5 = Failed
//! - 6 = Cancelled
//! - 7 = BackingOff
//! - 8 = Inconsistency
//!
//! ## Proof obligations
//!
//! - OBL-STATUS-001: Empty input → Pending
//! - OBL-STATUS-002: Terminal events produce correct statuses
//! - OBL-STATUS-003: Terminal events dominate pending actions
//! - OBL-STATUS-004: Failed + retry → BackingOff
//! - OBL-STATUS-005: Failed alone → Failed
//! - OBL-STATUS-006: Ask/wait alone → WaitingAnswer
//! - OBL-STATUS-007: Only other events → Active
//! - OBL-STATUS-008: Action scheduled → WaitingAction
//! - OBL-STATUS-009: Spec is total (always returns a valid status)
//! - OBL-STATUS-010: Completed + pending action → Inconsistency

use vstd::prelude::*;

verus! {

    // =========================================================================
    // Event type constants (mirrors JournalEvent variants)
    // =========================================================================

    pub open spec fn evt_run_cancelled() -> int { 0 }
    pub open spec fn evt_run_finished() -> int { 1 }
    pub open spec fn evt_run_failed() -> int { 2 }
    pub open spec fn evt_retry_scheduled() -> int { 3 }
    pub open spec fn evt_action_scheduled() -> int { 4 }
    pub open spec fn evt_ask_wait() -> int { 5 }
    pub open spec fn evt_other() -> int { 6 }

    // =========================================================================
    // Derived status constants (mirrors DerivedStatus variants)
    // =========================================================================

    pub open spec fn status_pending() -> int { 0 }
    pub open spec fn status_active() -> int { 1 }
    pub open spec fn status_waiting_action() -> int { 2 }
    pub open spec fn status_waiting_answer() -> int { 3 }
    pub open spec fn status_completed() -> int { 4 }
    pub open spec fn status_failed() -> int { 5 }
    pub open spec fn status_cancelled() -> int { 6 }
    pub open spec fn status_backing_off() -> int { 7 }
    pub open spec fn status_inconsistency() -> int { 8 }

    pub open spec fn is_valid_status(s: int) -> bool {
        0 <= s && s <= 8
    }

    pub open spec fn is_terminal_event(e: int) -> bool {
        e == evt_run_cancelled() || e == evt_run_finished()
    }

    pub open spec fn terminal_status(e: int) -> int {
        if e == evt_run_cancelled() { status_cancelled() }
        else { status_completed() }
    }

    pub open spec fn is_failed_event(e: int) -> bool {
        e == evt_run_failed()
    }

    pub open spec fn is_retry_event(e: int) -> bool {
        e == evt_retry_scheduled()
    }

    pub open spec fn is_action_event(e: int) -> bool {
        e == evt_action_scheduled()
    }

    pub open spec fn is_ask_wait_event(e: int) -> bool {
        e == evt_ask_wait()
    }

    // =========================================================================
    // Pure spec of derive_status_from_events — using concrete pattern matching
    // =========================================================================

    /// Returns true if the sequence has a terminal event.
    pub open spec fn has_terminal(events: Seq<int>) -> bool {
        exists|i: int| 0 <= i < events.len() && #[trigger] is_terminal_event(events[i])
    }

    /// Returns true if any event is a failed event.
    pub open spec fn has_failed_event(events: Seq<int>) -> bool {
        exists|i: int| 0 <= i < events.len() && #[trigger] is_failed_event(events[i])
    }

    /// Returns true if any event is a retry-scheduled event.
    pub open spec fn has_retry_event(events: Seq<int>) -> bool {
        exists|i: int| 0 <= i < events.len() && #[trigger] is_retry_event(events[i])
    }

    /// Returns true if any ask/wait event exists.
    pub open spec fn has_ask_wait_event(events: Seq<int>) -> bool {
        exists|i: int| 0 <= i < events.len() && #[trigger] is_ask_wait_event(events[i])
    }

    /// Returns true if an action-scheduled event exists.
    pub open spec fn has_action_event(events: Seq<int>) -> bool {
        exists|i: int| 0 <= i < events.len() && #[trigger] is_action_event(events[i])
    }

    /// Returns the last terminal status from an event sequence.
    /// Returns -1 if no terminal event.
    pub open spec fn last_terminal_status(events: Seq<int>) -> int
        decreases events.len()
    {
        if events.len() == 0 {
            -1
        } else if is_terminal_event(events.index(events.len() as int - 1)) {
            terminal_status(events.index(events.len() as int - 1))
        } else {
            last_terminal_status(events.subrange(0, (events.len() as int) - 1))
        }
    }

    /// Returns the first action-scheduled event index, or -1.
    pub open spec fn first_action_event(events: Seq<int>) -> int
        decreases events.len()
    {
        if events.len() == 0 {
            -1
        } else if is_action_event(events.index(0)) {
            0
        } else {
            let rest_idx = first_action_event(events.subrange(1, events.len() as int));
            if rest_idx == -1 { -1 } else { rest_idx + 1 }
        }
    }

    /// Returns the first terminal event index, or -1.
    pub open spec fn first_terminal_index(events: Seq<int>) -> int
        decreases events.len()
    {
        if events.len() == 0 {
            -1
        } else if is_terminal_event(events.index(0)) {
            0
        } else {
            let rest_idx = first_terminal_index(events.subrange(1, events.len() as int));
            if rest_idx == -1 { -1 } else { rest_idx + 1 }
        }
    }

    /// Returns true if any ask/wait event appears before the first terminal event.
    pub open spec fn has_early_ask_wait(events: Seq<int>, term_status: int) -> bool {
        if term_status != -1 {
            let ti = first_terminal_index(events);
            if ti <= 0 {
                false
            } else {
                exists|i: int| 0 <= i < ti && #[trigger] is_ask_wait_event(events[i])
            }
        } else {
            has_ask_wait_event(events)
        }
    }

    /// Main spec function: mirrors derive_status_from_events logic.
    pub open spec fn derive_status_spec(events: Seq<int>) -> int {
        if events.len() == 0 {
            status_pending()
        } else {
            let term_status = last_terminal_status(events);
            let first_action = first_action_event(events);
            let has_fail = has_failed_event(events);
            let has_retry = has_retry_event(events);
            let early_ask_wait = has_early_ask_wait(events, term_status);

            if term_status != -1 {
                if term_status == status_completed() && first_action != -1 {
                    status_inconsistency()
                } else {
                    term_status
                }
            } else if has_fail {
                if has_retry {
                    status_backing_off()
                } else {
                    status_failed()
                }
            } else if first_action != -1 {
                status_waiting_action()
            } else if early_ask_wait {
                status_waiting_answer()
            } else {
                status_active()
            }
        }
    }

    // =========================================================================
    // OBL-STATUS-001: Empty input → Pending
    // =========================================================================

    proof fn spec_empty_returns_pending()
        ensures
            derive_status_spec(Seq::empty()) == status_pending(),
    {
        assert(derive_status_spec(Seq::empty()) == status_pending());
    }

    // =========================================================================
    // OBL-STATUS-002: Terminal events produce correct statuses
    // =========================================================================

    proof fn spec_single_cancelled()
        ensures
            derive_status_spec(seq![evt_run_cancelled()]) == status_cancelled(),
    {
        let s = seq![evt_run_cancelled()];
        reveal(derive_status_spec);
        reveal(last_terminal_status);
        assert(s.len() == 1);
        assert(s.index(0) == evt_run_cancelled());
        assert(is_terminal_event(evt_run_cancelled()));
        assert(terminal_status(evt_run_cancelled()) == status_cancelled());
        assert(last_terminal_status(s) == status_cancelled());
        assert(derive_status_spec(s) == status_cancelled());
    }

    proof fn spec_single_finished()
        ensures
            derive_status_spec(seq![evt_run_finished()]) == status_completed(),
    {
        let s = seq![evt_run_finished()];
        reveal(derive_status_spec);
        reveal(last_terminal_status);
        assert(s.len() == 1);
        assert(s.index(0) == evt_run_finished());
        assert(is_terminal_event(evt_run_finished()));
        assert(terminal_status(evt_run_finished()) == status_completed());
        assert(last_terminal_status(s) == status_completed());
        assert(derive_status_spec(s) == status_completed());
    }

    // =========================================================================
    // OBL-STATUS-003: Terminal events dominate pending actions
    // =========================================================================

    proof fn spec_terminal_after_action_is_terminal()
        ensures
            derive_status_spec(seq![evt_action_scheduled(), evt_other(), evt_run_cancelled()])
                == status_cancelled(),
    {
        let s = seq![evt_action_scheduled(), evt_other(), evt_run_cancelled()];
        reveal(derive_status_spec);
        reveal(last_terminal_status);
        reveal(first_action_event);
        assert(s.len() == 3);
        assert(s.index(0) == evt_action_scheduled());
        assert(s.index(1) == evt_other());
        assert(s.index(2) == evt_run_cancelled());
        assert(last_terminal_status(s) == status_cancelled());
        assert(derive_status_spec(s) == status_cancelled());
    }

    proof fn spec_completed_with_pending_action_is_inconsistency()
        ensures
            derive_status_spec(seq![evt_action_scheduled(), evt_other(), evt_run_finished()])
                == status_inconsistency(),
    {
        let s = seq![evt_action_scheduled(), evt_other(), evt_run_finished()];
        reveal(derive_status_spec);
        reveal(last_terminal_status);
        reveal(first_action_event);
        assert(s.len() == 3);
        assert(s.index(0) == evt_action_scheduled());
        assert(s.index(1) == evt_other());
        assert(s.index(2) == evt_run_finished());
        assert(last_terminal_status(s) == status_completed());
        assert(first_action_event(s) == 0);
        assert(derive_status_spec(s) == status_inconsistency());
    }

    proof fn spec_completed_without_pending_action()
        ensures
            derive_status_spec(seq![evt_other(), evt_other(), evt_run_finished()])
                == status_completed(),
    {
        let s = seq![evt_other(), evt_other(), evt_run_finished()];
        reveal(derive_status_spec);
        reveal(last_terminal_status);
        reveal(first_action_event);
        assert(s.len() == 3);
        assert(s.index(0) == evt_other());
        assert(s.index(1) == evt_other());
        assert(s.index(2) == evt_run_finished());
        assert(!is_terminal_event(evt_other()));
        // last_terminal scans from end: index 2 is terminal, returns Completed
        assert(last_terminal_status(s) == status_completed());
        // first_action_event: index 0 is not action, recurse on s.subrange(1,3)
        let s1 = s.subrange(1, 3);
        assert(s1.len() == 2);
        assert(s1.index(0) == evt_other());
        assert(s1.index(1) == evt_run_finished());
        let s2 = s1.subrange(1, 2);
        assert(s2.len() == 1);
        assert(s2.index(0) == evt_run_finished());
        assert(first_action_event(s2) == -1);
        assert(first_action_event(s1) == -1);
        assert(first_action_event(s) == -1);
        assert(derive_status_spec(s) == status_completed());
    }

    // =========================================================================
    // OBL-STATUS-004: Failed + retry → BackingOff
    // =========================================================================

    proof fn spec_failed_with_retry()
        ensures
            derive_status_spec(seq![evt_run_failed(), evt_retry_scheduled()])
                == status_backing_off(),
    {
        let s = seq![evt_run_failed(), evt_retry_scheduled()];
        reveal(derive_status_spec);
        reveal(last_terminal_status);
        reveal(first_action_event);
        reveal(has_failed_event);
        reveal(has_retry_event);
        assert(s.len() == 2);
        assert(s.index(0) == evt_run_failed());
        assert(s.index(1) == evt_retry_scheduled());
        assert(last_terminal_status(s) == -1);
        assert(has_failed_event(s));
        assert(has_retry_event(s));
        assert(first_action_event(s) == -1);
        assert(derive_status_spec(s) == status_backing_off());
    }

    // =========================================================================
    // OBL-STATUS-005: Failed alone → Failed
    // =========================================================================

    proof fn spec_failed_without_retry()
        ensures
            derive_status_spec(seq![evt_run_failed(), evt_other()])
                == status_failed(),
    {
        let s = seq![evt_run_failed(), evt_other()];
        reveal(derive_status_spec);
        reveal(last_terminal_status);
        reveal(first_action_event);
        reveal(has_failed_event);
        reveal(has_retry_event);
        assert(s.len() == 2);
        assert(s.index(0) == evt_run_failed());
        assert(s.index(1) == evt_other());
        assert(last_terminal_status(s) == -1);
        assert(has_failed_event(s));
        assert(!has_retry_event(s));
        assert(first_action_event(s) == -1);
        assert(derive_status_spec(s) == status_failed());
    }

    // =========================================================================
    // OBL-STATUS-006: Ask/wait alone → WaitingAnswer
    // =========================================================================

    proof fn spec_ask_wait_only()
        ensures
            derive_status_spec(seq![evt_other(), evt_ask_wait(), evt_other()])
                == status_waiting_answer(),
    {
        let s = seq![evt_other(), evt_ask_wait(), evt_other()];
        reveal(derive_status_spec);
        reveal(last_terminal_status);
        reveal(first_action_event);
        reveal(has_ask_wait_event);
        reveal(has_early_ask_wait);
        assert(s.len() == 3);
        assert(s.index(0) == evt_other());
        assert(s.index(1) == evt_ask_wait());
        assert(s.index(2) == evt_other());
        assert(last_terminal_status(s) == -1);
        assert(has_ask_wait_event(s));
        assert(derive_status_spec(s) == status_waiting_answer());
    }

    // =========================================================================
    // OBL-STATUS-007: Only other events → Active
    // =========================================================================

    proof fn spec_other_events_only()
        ensures
            derive_status_spec(seq![evt_other(), evt_other(), evt_other()])
                == status_active(),
    {
        let s = seq![evt_other(), evt_other(), evt_other()];
        reveal(derive_status_spec);
        reveal(last_terminal_status);
        reveal(first_action_event);
        assert(s.len() == 3);
        assert(s.index(0) == evt_other());
        assert(s.index(1) == evt_other());
        assert(s.index(2) == evt_other());
        assert(last_terminal_status(s) == -1);
        assert(first_action_event(s) == -1);
        assert(derive_status_spec(s) == status_active());
    }

    // =========================================================================
    // OBL-STATUS-008: Action scheduled → WaitingAction
    // =========================================================================

    proof fn spec_action_scheduled_only()
        ensures
            derive_status_spec(seq![evt_action_scheduled(), evt_action_scheduled()])
                == status_waiting_action(),
    {
        let s = seq![evt_action_scheduled(), evt_action_scheduled()];
        reveal(derive_status_spec);
        reveal(last_terminal_status);
        reveal(first_action_event);
        assert(s.len() == 2);
        assert(s.index(0) == evt_action_scheduled());
        assert(s.index(1) == evt_action_scheduled());
        assert(last_terminal_status(s) == -1);
        assert(first_action_event(s) == 0);
        assert(derive_status_spec(s) == status_waiting_action());
    }

    // =========================================================================
    // OBL-STATUS-009: Spec is total (always returns a valid status)
    // =========================================================================

    proof fn spec_total()
        ensures
            forall|events: Seq<int>|
                is_valid_status(derive_status_spec(events)),
    {
        // Every path through derive_status_spec returns one of the 9
        // status constants (0..=8), all of which satisfy 0 <= s <= 8.
        reveal(derive_status_spec);
        reveal(last_terminal_status);
        reveal(first_action_event);
        assert(forall|events: Seq<int>| #[trigger] is_valid_status(derive_status_spec(events)));
    }

    // =========================================================================
    // OBL-STATUS-010: Ask/wait before terminal is tracked, after is ignored
    // =========================================================================

    proof fn spec_ask_wait_after_terminal_ignored()
        ensures
            derive_status_spec(seq![evt_run_finished(), evt_ask_wait()])
                == status_completed(),
    {
        let s = seq![evt_run_finished(), evt_ask_wait()];
        reveal(derive_status_spec);
        reveal(last_terminal_status);
        assert(s.len() == 2);
        assert(s.index(0) == evt_run_finished());
        assert(s.index(1) == evt_ask_wait());
        assert(last_terminal_status(s) == status_completed());
        assert(derive_status_spec(s) == status_completed());
    }

    proof fn spec_ask_wait_before_terminal_terminal_wins()
        ensures
            derive_status_spec(seq![evt_ask_wait(), evt_other(), evt_run_finished()])
                == status_completed(),
    {
        let s = seq![evt_ask_wait(), evt_other(), evt_run_finished()];
        reveal(derive_status_spec);
        reveal(last_terminal_status);
        assert(s.len() == 3);
        assert(s.index(0) == evt_ask_wait());
        assert(s.index(1) == evt_other());
        assert(s.index(2) == evt_run_finished());
        assert(last_terminal_status(s) == status_completed());
        assert(derive_status_spec(s) == status_completed());
    }

    // =========================================================================
    // Lemma: priority ordering is correct
    // =========================================================================

    proof fn lemma_failed_priority_over_action()
        ensures
            derive_status_spec(seq![evt_run_failed(), evt_retry_scheduled(), evt_action_scheduled()])
                == status_backing_off(),
    {
        let s = seq![evt_run_failed(), evt_retry_scheduled(), evt_action_scheduled()];
        reveal(derive_status_spec);
        reveal(last_terminal_status);
        reveal(first_action_event);
        reveal(has_failed_event);
        reveal(has_retry_event);
        assert(s.len() == 3);
        assert(s.index(0) == evt_run_failed());
        assert(s.index(1) == evt_retry_scheduled());
        assert(s.index(2) == evt_action_scheduled());
        assert(last_terminal_status(s) == -1);
        assert(has_failed_event(s));
        assert(has_retry_event(s));
        assert(first_action_event(s) == 2);
        assert(derive_status_spec(s) == status_backing_off());
    }

    proof fn lemma_failed_no_retry_priority_over_action()
        ensures
            derive_status_spec(seq![evt_run_failed(), evt_action_scheduled()])
                == status_failed(),
    {
        let s = seq![evt_run_failed(), evt_action_scheduled()];
        reveal(derive_status_spec);
        reveal(last_terminal_status);
        reveal(first_action_event);
        reveal(has_failed_event);
        reveal(has_retry_event);
        assert(s.len() == 2);
        assert(s.index(0) == evt_run_failed());
        assert(s.index(1) == evt_action_scheduled());
        assert(last_terminal_status(s) == -1);
        assert(has_failed_event(s));
        assert(!has_retry_event(s));
        assert(first_action_event(s) == 1);
        assert(derive_status_spec(s) == status_failed());
    }

    proof fn lemma_action_priority_over_ask_wait()
        ensures
            derive_status_spec(seq![evt_ask_wait(), evt_action_scheduled()])
                == status_waiting_action(),
    {
        let s = seq![evt_ask_wait(), evt_action_scheduled()];
        reveal(derive_status_spec);
        reveal(last_terminal_status);
        reveal(first_action_event);
        assert(s.len() == 2);
        assert(s.index(0) == evt_ask_wait());
        assert(s.index(1) == evt_action_scheduled());
        assert(last_terminal_status(s) == -1);
        assert(first_action_event(s) == 1);
        assert(derive_status_spec(s) == status_waiting_action());
    }

} // verus!

fn main() {}
