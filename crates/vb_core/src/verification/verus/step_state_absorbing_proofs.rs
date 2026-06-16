//! Standalone Verus proofs for StepState transition invariants.
//!
//! This file defines spec mirror types for StepState and proves:
//! - Terminal states (Succeeded, Failed, Skipped, Cancelled) block all non-self transitions
//! - Self-transitions are always valid
//! - The valid transition table is exhaustive and consistent
//!
//! Production binding:
//! - StepState enum → crate::frame::StepState (8 variants)
//! - is_valid_step_state_transition → crate::frame::is_valid_step_state_transition
//!
//! GOD RULE 2: Specs mirror production logic without depending on crate imports.

use vstd::prelude::*;

verus! {

    // ===========================================================================
    // Spec mirror types for StepState
    // ===========================================================================

    /// Mirrors crate::frame::StepState (8 variants).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SpecStepState {
        Pending,
        Running,
        Succeeded,
        Failed,
        Skipped,
        Waiting,
        Asking,
        Cancelled,
    }

    /// Mirrors crate::frame::is_valid_step_state_transition.
    pub closed spec fn spec_is_valid_transition(current: SpecStepState, new: SpecStepState) -> bool {
        if current == new {
            true
        } else {
            matches!(
                (current, new),
                (SpecStepState::Pending, SpecStepState::Running)
                    | (SpecStepState::Pending, SpecStepState::Succeeded)
                    | (SpecStepState::Pending, SpecStepState::Failed)
                    | (SpecStepState::Pending, SpecStepState::Cancelled)
                    | (SpecStepState::Pending, SpecStepState::Skipped)
                    | (SpecStepState::Running, SpecStepState::Succeeded)
                    | (SpecStepState::Running, SpecStepState::Failed)
                    | (SpecStepState::Running, SpecStepState::Waiting)
                    | (SpecStepState::Running, SpecStepState::Asking)
                    | (SpecStepState::Running, SpecStepState::Cancelled)
                    | (SpecStepState::Running, SpecStepState::Skipped)
                    | (SpecStepState::Waiting, SpecStepState::Running)
                    | (SpecStepState::Asking, SpecStepState::Running)
            )
        }
    }

    /// Spec: terminal states in StepState.
    pub closed spec fn spec_is_terminal(s: SpecStepState) -> bool {
        matches!(s, SpecStepState::Succeeded | SpecStepState::Failed | SpecStepState::Skipped | SpecStepState::Cancelled)
    }

    // ===========================================================================
    // PO-STEP-001: Self-transitions are always valid.
    // ===========================================================================

    /// Proof: every state has a valid self-transition.
    pub proof fn proof_self_transitions_always_valid()
        ensures
            forall|s: SpecStepState| spec_is_valid_transition(s, s),
    {
        assert forall|s: SpecStepState| spec_is_valid_transition(s, s) by {
            reveal(spec_is_valid_transition);
            // Self-equality short-circuits to true.
        };
    }

    // ===========================================================================
    // PO-STEP-002: Terminal states block all non-self transitions.
    // ===========================================================================

    /// Proof: terminal states have no outgoing transitions except self.
    pub proof fn proof_terminal_states_absorbing()
        ensures
            forall|t: SpecStepState, s: SpecStepState|
                spec_is_terminal(t) && t != s ==> !spec_is_valid_transition(t, s),
    {
        assert forall|t: SpecStepState, s: SpecStepState|
            spec_is_terminal(t) && t != s ==> !spec_is_valid_transition(t, s) by {
            if spec_is_terminal(t) && t != s {
                reveal(spec_is_terminal);
                reveal(spec_is_valid_transition);
                // Terminal states (Succeeded, Failed, Skipped, Cancelled) only have self-transitions.
                // None appear as source in the VALID_TRANSITIONS constant.
                assert(!spec_is_valid_transition(t, s));
            }
        };
    }

    // ===========================================================================
    // PO-STEP-003: The valid transition table is exhaustive.
    // ===========================================================================

    /// Proof: all 13 explicit transitions plus self-transitions cover the spec.
    pub proof fn proof_transition_table_exhaustive()
        ensures
            forall|current: SpecStepState, new: SpecStepState|
                spec_is_valid_transition(current, new)
                    ==> (current == new
                        || matches!(
                            (current, new),
                            (SpecStepState::Pending, SpecStepState::Running)
                                | (SpecStepState::Pending, SpecStepState::Succeeded)
                                | (SpecStepState::Pending, SpecStepState::Failed)
                                | (SpecStepState::Pending, SpecStepState::Cancelled)
                                | (SpecStepState::Pending, SpecStepState::Skipped)
                                | (SpecStepState::Running, SpecStepState::Succeeded)
                                | (SpecStepState::Running, SpecStepState::Failed)
                                | (SpecStepState::Running, SpecStepState::Waiting)
                                | (SpecStepState::Running, SpecStepState::Asking)
                                | (SpecStepState::Running, SpecStepState::Cancelled)
                                | (SpecStepState::Running, SpecStepState::Skipped)
                                | (SpecStepState::Waiting, SpecStepState::Running)
                                | (SpecStepState::Asking, SpecStepState::Running)
                        )),
    {
        assert forall|current: SpecStepState, new: SpecStepState|
            spec_is_valid_transition(current, new)
                ==> (current == new
                    || matches!(
                        (current, new),
                        (SpecStepState::Pending, SpecStepState::Running)
                            | (SpecStepState::Pending, SpecStepState::Succeeded)
                            | (SpecStepState::Pending, SpecStepState::Failed)
                            | (SpecStepState::Pending, SpecStepState::Cancelled)
                            | (SpecStepState::Pending, SpecStepState::Skipped)
                            | (SpecStepState::Running, SpecStepState::Succeeded)
                            | (SpecStepState::Running, SpecStepState::Failed)
                            | (SpecStepState::Running, SpecStepState::Waiting)
                            | (SpecStepState::Running, SpecStepState::Asking)
                            | (SpecStepState::Running, SpecStepState::Cancelled)
                            | (SpecStepState::Running, SpecStepState::Skipped)
                            | (SpecStepState::Waiting, SpecStepState::Running)
                            | (SpecStepState::Asking, SpecStepState::Running)
                    )) by {
            reveal(spec_is_valid_transition);
            // The spec definition is exactly the self-check plus the 13 explicit transitions.
        };
    }

    // ===========================================================================
    // PO-STEP-004: No illegal transitions exist in the spec.
    // ===========================================================================

    /// Proof: any transition not in the table is invalid.
    pub proof fn proof_no_illegal_transitions()
        ensures
            forall|current: SpecStepState, new: SpecStepState|
                !(current == new
                    || matches!(
                        (current, new),
                        (SpecStepState::Pending, SpecStepState::Running)
                            | (SpecStepState::Pending, SpecStepState::Succeeded)
                            | (SpecStepState::Pending, SpecStepState::Failed)
                            | (SpecStepState::Pending, SpecStepState::Cancelled)
                            | (SpecStepState::Pending, SpecStepState::Skipped)
                            | (SpecStepState::Running, SpecStepState::Succeeded)
                            | (SpecStepState::Running, SpecStepState::Failed)
                            | (SpecStepState::Running, SpecStepState::Waiting)
                            | (SpecStepState::Running, SpecStepState::Asking)
                            | (SpecStepState::Running, SpecStepState::Cancelled)
                            | (SpecStepState::Running, SpecStepState::Skipped)
                            | (SpecStepState::Waiting, SpecStepState::Running)
                            | (SpecStepState::Asking, SpecStepState::Running)
                    )) ==> !spec_is_valid_transition(current, new),
    {
        assert forall|current: SpecStepState, new: SpecStepState|
            !(current == new
                || matches!(
                    (current, new),
                    (SpecStepState::Pending, SpecStepState::Running)
                        | (SpecStepState::Pending, SpecStepState::Succeeded)
                        | (SpecStepState::Pending, SpecStepState::Failed)
                        | (SpecStepState::Pending, SpecStepState::Cancelled)
                        | (SpecStepState::Pending, SpecStepState::Skipped)
                        | (SpecStepState::Running, SpecStepState::Succeeded)
                        | (SpecStepState::Running, SpecStepState::Failed)
                        | (SpecStepState::Running, SpecStepState::Waiting)
                        | (SpecStepState::Running, SpecStepState::Asking)
                        | (SpecStepState::Running, SpecStepState::Cancelled)
                        | (SpecStepState::Running, SpecStepState::Skipped)
                        | (SpecStepState::Waiting, SpecStepState::Running)
                        | (SpecStepState::Asking, SpecStepState::Running)
                )) ==> !spec_is_valid_transition(current, new) by {
            reveal(spec_is_valid_transition);
            // If the transition is not in the table, spec_is_valid_transition returns false.
        };
    }

    // ===========================================================================
    // PO-STEP-005: No terminal state can transition to Running.
    // ===========================================================================

    /// Proof: terminal states (Succeeded, Failed, Skipped, Cancelled) cannot go to Running.
    pub proof fn proof_terminal_to_running_forbidden()
        ensures
            forall|t: SpecStepState|
                spec_is_terminal(t) ==> !spec_is_valid_transition(t, SpecStepState::Running),
    {
        assert forall|t: SpecStepState|
            spec_is_terminal(t) ==> !spec_is_valid_transition(t, SpecStepState::Running) by {
            if spec_is_terminal(t) {
                reveal(spec_is_terminal);
                reveal(spec_is_valid_transition);
                // Running only appears as a destination from Pending or from Waiting/Asking.
                // Terminal states never appear as source of Running.
                assert(!spec_is_valid_transition(t, SpecStepState::Running));
            }
        };
    }

    // ===========================================================================
    // PO-STEP-006: Pending is the only entry point to Running (from non-terminal).
    // ===========================================================================

    /// Proof: the only non-terminal states that can transition to Running are Pending, Waiting, Asking (and self).
    pub proof fn proof_only_pending_can_enter_running()
        ensures
            forall|current: SpecStepState, new: SpecStepState|
                spec_is_valid_transition(current, new) && new == SpecStepState::Running
                    ==> current == SpecStepState::Running
                        || current == SpecStepState::Pending
                        || current == SpecStepState::Waiting || current == SpecStepState::Asking,
    {
        assert forall|current: SpecStepState, new: SpecStepState|
            spec_is_valid_transition(current, new) && new == SpecStepState::Running
                ==> current == SpecStepState::Running
                    || current == SpecStepState::Pending
                    || current == SpecStepState::Waiting || current == SpecStepState::Asking by {
            reveal(spec_is_valid_transition);
            // Running can only be reached from Pending, Waiting, Asking (explicit transitions)
            // or from Running itself (self-transition).
        };
    }
}
