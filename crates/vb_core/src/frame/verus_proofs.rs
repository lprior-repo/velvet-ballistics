//! Verus formal verification proofs for frame state machine.
//!
//! This module is compiled only under the Verus toolchain (`#[cfg(verus)]`).
//! It contains spec functions and proof lemmas that bind production code
//! invariants to mathematical models.
//!
//! See `frame.rs` for the production types these proofs reference.

verus! {
    use vstd::prelude::*;

    use crate::frame::{StepState, is_valid_step_state_transition, RunFrame, CoreError, CoreResult};

    /// Spec: is_valid_step_state_transition — mathematical model.
    /// This spec defines the ground-truth transition relation that the
    /// production implementation must satisfy.
    pub closed spec fn spec_is_valid_step_state_transition(current: StepState, new: StepState) -> bool {
        if current == new {
            true
        } else {
            matches!(
                (current, new),
                (StepState::Pending, StepState::Running)
                    | (StepState::Pending, StepState::Succeeded)
                    | (StepState::Pending, StepState::Failed)
                    | (StepState::Pending, StepState::Cancelled)
                    | (StepState::Pending, StepState::Skipped)
                    | (StepState::Running, StepState::Succeeded)
                    | (StepState::Running, StepState::Failed)
                    | (StepState::Running, StepState::Waiting)
                    | (StepState::Running, StepState::Asking)
                    | (StepState::Running, StepState::Cancelled)
                    | (StepState::Running, StepState::Skipped)
                    | (StepState::Waiting, StepState::Running)
                    | (StepState::Asking, StepState::Running)
            )
        }
    }

    /// Spec: terminal state predicate.
    /// Terminal states: Succeeded, Failed, Skipped, Cancelled.
    pub closed spec fn spec_is_terminal_state(s: StepState) -> bool {
        matches!(s, StepState::Succeeded | StepState::Failed | StepState::Skipped | StepState::Cancelled)
    }

    /// Spec: terminal states have no outgoing transitions except self.
    pub closed spec fn spec_terminal_states_absorbing() -> bool {
        forall(|t: StepState, s: StepState| {
            spec_is_terminal_state(t) && t != s ==> !spec_is_valid_step_state_transition(t, s)
        })
    }

    /// Spec: self-transition is always valid for any state.
    pub closed spec fn spec_self_transition_always_valid() -> bool {
        forall(|s: StepState| spec_is_valid_step_state_transition(s, s))
    }

    /// Proof: production is_valid_step_state_transition equals the spec.
    pub proof fn lemma_is_valid_step_state_transition_matches_spec(current: StepState, new: StepState)
        ensures
            spec_is_valid_step_state_transition(current, new)
                == is_valid_step_state_transition(current, new),
    {
        // Reveal both definitions and let compute discharge.
        reveal_with_fuel(is_valid_step_state_transition, 1);
        reveal(spec_is_valid_step_state_transition);
        assert(spec_is_valid_step_state_transition(current, new)
            == is_valid_step_state_transition(current, new));
    }

    /// Proof: terminal states block all non-self transitions.
    pub proof fn lemma_terminal_states_absorbing()
        ensures
            forall|t: StepState, s: StepState|
                spec_is_terminal_state(t) && t != s
                    ==> !spec_is_valid_step_state_transition(t, s),
    {
        assert forall|t: StepState, s: StepState|
            spec_is_terminal_state(t) && t != s
                ==> !spec_is_valid_step_state_transition(t, s) by {
            if spec_is_terminal_state(t) && t != s {
                reveal(spec_is_terminal_state);
                reveal(spec_is_valid_step_state_transition);
                assert(!spec_is_valid_step_state_transition(t, s));
            }
        };
    }

    /// Proof: self-transitions are always valid.
    pub proof fn lemma_self_transitions_always_valid()
        ensures
            forall|s: StepState| spec_is_valid_step_state_transition(s, s),
    {
        assert forall|s: StepState| spec_is_valid_step_state_transition(s, s) by {
            reveal(spec_is_valid_step_state_transition);
            // Self-equality short-circuits to true.
        };
    }

    // ── RunFrame::new specs ──

    /// Spec: valid inputs for RunFrame::new.
    pub closed spec fn spec_run_frame_new_accepts(step_count: u16, first_step: u16) -> bool {
        step_count > 0 && first_step < step_count
    }

    /// Spec: valid slot count.
    pub closed spec fn spec_run_frame_new_accepts_slot_count(slot_count: u16) -> bool {
        slot_count > 0
    }

    /// Proof: RunFrame::new returns Err when step_count == 0.
    pub proof fn lemma_run_frame_new_step_count_zero_returns_err(run: RunId, first_step: StepIdx, slot_count: u16)
        ensures
            RunFrame::new(run, first_step, 0, slot_count).is_err(),
    {
        // Production checks step_count == 0 and returns Err.
        assert(RunFrame::new(run, first_step, 0, slot_count).is_err());
    }

    /// Proof: RunFrame::new returns Err when first_step >= step_count.
    pub proof fn lemma_run_frame_new_first_step_out_of_bounds_returns_err(
        run: RunId,
        first_step: StepIdx,
        step_count: u16,
        slot_count: u16,
    )
        requires
            step_count > 0 && first_step.as_usize() >= usize::from(step_count),
        ensures
            RunFrame::new(run, first_step, step_count, slot_count).is_err(),
    {
        // Production checks first_step >= step_count and returns Err.
        assert(RunFrame::new(run, first_step, step_count, slot_count).is_err());
    }

    /// Proof: valid inputs produce Ok with Pending states.
    pub proof fn lemma_run_frame_new_valid_inputs_produce_pending_states(
        run: RunId,
        first_step: StepIdx,
        step_count: u16,
        slot_count: u16,
    )
        requires
            step_count > 0 && first_step.as_usize() < usize::from(step_count) && slot_count > 0,
        ensures
            let result = RunFrame::new(run, first_step, step_count, slot_count);
            result.is_ok()
                && (forall|i: usize| i < usize::from(step_count) ==> result.unwrap().states[i] == StepState::Pending),
    {
        let result = RunFrame::new(run, first_step, step_count, slot_count);
        assert(result.is_ok());
        let frame = result.unwrap();
        assert(forall|i: usize| i < usize::from(step_count) ==> frame.states[i] == StepState::Pending);
    }
}
