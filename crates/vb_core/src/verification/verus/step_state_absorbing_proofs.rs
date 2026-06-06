//! Verus terminal-transition invariant proofs — PO-VERUS-001, PO-VERUS-002, PO-VERUS-003.
//!
//! ## Obligations
//! - PO-VERUS-001: proof_fn_terminal_blocks_outward_transitions — all terminal non-self transitions are invalid (master contract: no terminal state transitions back to running)
//! - PO-VERUS-002: is_valid_step_state_transition exec fn binding with requires/ensures
//! - PO-VERUS-003: terminal_cannot_transition_to_non_terminal holds for all terminal states with no reentry exception
//!
//! ## Production Binding
//! - `is_valid_step_state_transition` → `vb_core::frame::is_valid_step_state_transition`
//! - `StepState` → `vb_core::frame::StepState`
//! - `terminal_cannot_transition_to_non_terminal` → `vb_proof_kernels::step_state::terminal_cannot_transition_to_non_terminal`
//!
//! ## GOD RULE 2 Compliance
//! The `exec fn` stubs below use `#[verifier::external_body]` to bind to the
//! production implementation. This is a **trust boundary** that must be resolved
//! by annotating the production source with Verus requires/ensures.
//! See TB-005 in trusted-base-ledger.jsonl.
//!
//! ## Trust Boundaries
//! - TB-005: Exec fn stubs with `#[verifier::external_body]` trust production behavior
//! - TB-005a: `binding_is_valid_transition` trusts `vb_core::frame::is_valid_step_state_transition`
//! - TB-005b: `binding_terminal_cannot` trusts `vb_proof_kernels::step_state::terminal_cannot_transition_to_non_terminal`

#![allow(unused_imports)]

use vstd::prelude::*;

verus! {

// ===========================================================================
// StepState model — mirrors vb_core::frame::StepState
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepState {
    Pending,
    Running,
    Succeeded,
    Failed,
    Skipped,
    Waiting,
    Asking,
    Cancelled,
}

// ===========================================================================
// Spec: terminal state predicate
// ===========================================================================

pub open spec fn spec_is_terminal(s: StepState) -> bool {
    match s {
        StepState::Succeeded | StepState::Failed | StepState::Skipped | StepState::Cancelled => true,
        _ => false,
    }
}

// ===========================================================================
// Spec: is_valid_step_state_transition — authoritative formal contract
// ===========================================================================

pub open spec fn spec_is_valid_transition(current: StepState, new: StepState) -> bool {
    // Self-transition (idempotent re-mark) always valid
    if current == new {
        true
    } else {
        match (current, new) {
            // Pending can transition to any execution result
            (StepState::Pending, StepState::Running)
            | (StepState::Pending, StepState::Succeeded)
            | (StepState::Pending, StepState::Failed)
            | (StepState::Pending, StepState::Cancelled)
            | (StepState::Pending, StepState::Skipped) => true,

            // Running can reach any terminal or suspend state
            (StepState::Running, StepState::Succeeded)
            | (StepState::Running, StepState::Failed)
            | (StepState::Running, StepState::Waiting)
            | (StepState::Running, StepState::Asking)
            | (StepState::Running, StepState::Cancelled)
            | (StepState::Running, StepState::Skipped) => true,

            // Suspended states resume to Running
            (StepState::Waiting, StepState::Running)
            | (StepState::Asking, StepState::Running) => true,

            // All other transitions invalid. No terminal->Running edge is
            // admitted; loop reentry uses the explicit Succeeded->Pending
            // admission path before mark_running.
            _ => false,
        }
    }
}

// ===========================================================================
// PO-VERUS-001: Terminal-transition invariant
// ===========================================================================

/// Lemma: For all terminal states t and all states s != t, transitions are
/// invalid (no terminal state transitions back to running).
pub proof fn lemma_terminal_blocks_outward_transitions(t: StepState, s: StepState)
    requires
        spec_is_terminal(t),
        t != s,
    ensures
        !spec_is_valid_transition(t, s),
{
    // Exhaustive case analysis over all terminal non-self transitions.
    // Since StepState is a finite 8-variant enum, this is a bounded proof.
    // Verus can discharge this via SMT + by(compute) on the finite space.

    // We reveal the spec function bodies and let the SMT solver
    // check all terminal non-self combinations.
    assert(!spec_is_valid_transition(t, s)) by {
        // The SMT solver can handle this finite case split natively
        // because spec_is_valid_transition and spec_is_terminal are both
        // defined over an 8-element enum.
    }
}

/// PO-VERUS-001: Top-level proof function proving the terminal transition
/// invariant for all terminal states and all distinct target states.
pub proof fn proof_fn_terminal_blocks_outward_transitions()
    ensures
        forall|t: StepState, s: StepState|
            spec_is_terminal(t) && t != s
                ==> !spec_is_valid_transition(t, s),
{
    // Exhaustive proof over 8-element StepState enum.
    // The SMT solver + by(compute) handles the finite case split.
    assert forall|t: StepState, s: StepState|
        spec_is_terminal(t) && t != s
            ==> !spec_is_valid_transition(t, s) by {
        if spec_is_terminal(t) && t != s {
            lemma_terminal_blocks_outward_transitions(t, s);
        }
    };
}

// ===========================================================================
// PO-VERUS-003: Terminal treatment
// ===========================================================================

/// Lemma: terminal states allow only self-transition.
pub proof fn lemma_terminal_treatment()
    ensures
        forall|t: StepState, s: StepState|
            spec_is_terminal(t) && t != s
                ==> !spec_is_valid_transition(t, s),
{
    assert forall|t: StepState, s: StepState|
        spec_is_terminal(t) && t != s
            ==> !spec_is_valid_transition(t, s) by {
        if spec_is_terminal(t) && t != s {
            lemma_terminal_blocks_outward_transitions(t, s);
        }
    };
}

/// Spec: terminal_cannot_transition_to_non_terminal — mirrors the proof kernel.
/// Returns true iff terminal states are fully absorbing (self-only).
pub open spec fn spec_terminal_cannot_transition_to_non_terminal() -> bool {
    forall|t: StepState| spec_is_terminal(t) ==>
        forall|s: StepState| t != s
            ==> !spec_is_valid_transition(t, s)
}

/// PO-VERUS-003: Proof that terminal_cannot_transition_to_non_terminal holds.
pub proof fn proof_fn_terminal_cannot_transition_to_non_terminal()
    ensures
        spec_terminal_cannot_transition_to_non_terminal(),
{
    // Unfold and prove the forall over all StepState variants
    assert(spec_terminal_cannot_transition_to_non_terminal()) by {
        assert forall|t: StepState| spec_is_terminal(t) ==>
            forall|s: StepState| t != s
                ==> !spec_is_valid_transition(t, s) by {
            if spec_is_terminal(t) {
                assert forall|s: StepState| t != s
                    ==> !spec_is_valid_transition(t, s) by {
                    if t != s {
                        lemma_terminal_blocks_outward_transitions(t, s);
                    }
                };
            }
        };
    };
}

// ===========================================================================
// PO-VERUS-002: Production function binding (trust boundary)
// ===========================================================================

/// exec fn stub binding to the production is_valid_step_state_transition.
///
/// This function is marked `#[verifier::external_body]` because the production
/// function (vb_core::frame::is_valid_step_state_transition) does not yet have
/// Verus requires/ensures annotations.
///
/// TRUST BOUNDARY TB-005a: We trust that the production implementation
/// satisfies `spec_is_valid_transition` for all input pairs.
/// Full GOD RULE 2 compliance requires adding `requires`/`ensures` to the
/// production function in frame.rs.
#[verifier::external_body]
pub exec fn binding_is_valid_transition(current: StepState, new: StepState) -> (result: bool)
    ensures
        result == spec_is_valid_transition(current, new),
{
    // In production, this would delegate to vb_core::frame::is_valid_step_state_transition.
    // For now, use the spec itself as a placeholder until production annotations exist.
    spec_is_valid_transition(current, new)
}

/// exec fn stub binding to the proof kernel's
/// terminal_cannot_transition_to_non_terminal.
///
/// TRUST BOUNDARY TB-005b: The production proof kernel function
/// (vb_proof_kernels::step_state::terminal_cannot_transition_to_non_terminal)
/// must return true (all terminal states are absorbing; no terminal state
/// transitions back to running).
#[verifier::external_body]
pub exec fn binding_terminal_cannot_transition_to_non_terminal() -> (result: bool)
    ensures
        result == spec_terminal_cannot_transition_to_non_terminal(),
{
    // Placeholder: returns the spec value directly.
    // Full binding requires Verus annotations on the proof kernel.
    spec_terminal_cannot_transition_to_non_terminal()
}

} // verus!
