//! Verus proof lemmas for the step state machine.
//!
//! Local-only model checks — not production deductive evidence.

#[cfg(verus_keep_ghost)]
verus! {
    use crate::step_state::spec::{spec_is_terminal, spec_valid_transition};

    // ── Lemma: terminal states have no non-terminal successors ─────────
    proof fn lemma_terminal_has_no_non_terminal_successor(terminal: StepState)
        requires
            spec_is_terminal(terminal),
        ensures
            !spec_valid_transition(terminal, StepState::Pending)
                && !spec_valid_transition(terminal, StepState::Running)
                && !spec_valid_transition(terminal, StepState::Waiting)
                && !spec_valid_transition(terminal, StepState::Asking),
    {
        // Only Succeeded, Failed, Cancelled, Skipped are terminal.
        // Each has only self-transition; none transition to Pending/Running/Waiting/Asking.
        assert(!spec_valid_transition(terminal, StepState::Pending));
        assert(!spec_valid_transition(terminal, StepState::Running));
        assert(!spec_valid_transition(terminal, StepState::Waiting));
        assert(!spec_valid_transition(terminal, StepState::Asking));
    }

    // ── Lemma: pending is non-terminal ─────────────────────────────────
    proof fn lemma_pending_is_non_terminal()
        ensures
            !spec_is_terminal(StepState::Pending),
    {
        assert(!spec_is_terminal(StepState::Pending));
    }

    // ── Lemma: running is non-terminal ─────────────────────────────────
    proof fn lemma_running_is_non_terminal()
        ensures
            !spec_is_terminal(StepState::Running),
    {
        assert(!spec_is_terminal(StepState::Running));
    }

    // ── Lemma: waiting is non-terminal ─────────────────────────────────
    proof fn lemma_waiting_is_non_terminal()
        ensures
            !spec_is_terminal(StepState::Waiting),
    {
        assert(!spec_is_terminal(StepState::Waiting));
    }

    // ── Lemma: asking is non-terminal ──────────────────────────────────
    proof fn lemma_asking_is_non_terminal()
        ensures
            !spec_is_terminal(StepState::Asking),
    {
        assert(!spec_is_terminal(StepState::Asking));
    }

    // ── Lemma: succeeded is terminal ───────────────────────────────────
    proof fn lemma_succeeded_is_terminal()
        ensures
            spec_is_terminal(StepState::Succeeded),
    {
        assert(spec_is_terminal(StepState::Succeeded));
    }

    // ── Lemma: failed is terminal ──────────────────────────────────────
    proof fn lemma_failed_is_terminal()
        ensures
            spec_is_terminal(StepState::Failed),
    {
        assert(spec_is_terminal(StepState::Failed));
    }

    // ── Lemma: cancelled is terminal ───────────────────────────────────
    proof fn lemma_cancelled_is_terminal()
        ensures
            spec_is_terminal(StepState::Cancelled),
    {
        assert(spec_is_terminal(StepState::Cancelled));
    }

    // ── Lemma: skipped is terminal ─────────────────────────────────────
    proof fn lemma_skipped_is_terminal()
        ensures
            spec_is_terminal(StepState::Skipped),
    {
        assert(spec_is_terminal(StepState::Skipped));
    }

    // ── Lemma: terminal self-transitions are always valid ──────────────
    proof fn lemma_terminal_self_transition_valid(terminal: StepState)
        requires
            spec_is_terminal(terminal),
        ensures
            spec_valid_transition(terminal, terminal),
    {
        // Succeeded->Succeeded, Failed->Failed, Cancelled->Cancelled,
        // Skipped->Skipped are all listed in spec_valid_transition.
        assert(spec_valid_transition(terminal, terminal));
    }
}

#[cfg(verus_keep_ghost)]
pub use vstd::prelude::*;
