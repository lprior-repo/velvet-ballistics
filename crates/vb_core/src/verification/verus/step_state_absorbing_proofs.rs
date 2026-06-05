//! Verus absorbing-terminal invariant proofs — PO-VERUS-001, PO-VERUS-002, PO-VERUS-003.
//!
//! ## Obligations
//! - PO-VERUS-001: proof_fn_terminal_absorbing — forall terminal t, forall s != t: !is_valid(t, s)
//! - PO-VERUS-002: is_valid_step_state_transition exec fn binding with requires/ensures
//! - PO-VERUS-003: terminal_cannot_transition_to_non_terminal uniform terminal treatment
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

            // All other transitions (including Succeeded->*) invalid
            _ => false,
        }
    }
}

// ===========================================================================
// PO-VERUS-001: Absorbing-terminal invariant
// ===========================================================================

/// Lemma: For all terminal states t and all states s != t,
/// spec_is_valid_transition(t, s) is false.
pub proof fn lemma_terminal_absorbing(t: StepState, s: StepState)
    requires
        spec_is_terminal(t),
        t != s,
    ensures
        !spec_is_valid_transition(t, s),
{
    // Exhaustive case analysis over all 4 terminal states
    // and the 7 non-self states for each.
    // Since StepState is a finite 8-variant enum, this is a bounded proof.
    // Verus can discharge this via SMT + by(compute) on the finite space.

    // We reveal the spec function bodies and let the SMT solver
    // check all (4 × 7 = 28) terminal→non-terminal combinations.
    assert(!spec_is_valid_transition(t, s)) by {
        // The SMT solver can handle this finite case split natively
        // because spec_is_valid_transition and spec_is_terminal are both
        // defined over an 8-element enum.
    }
}

/// PO-VERUS-001: Top-level proof function proving the absorbing-terminal invariant
/// for all terminal states and all distinct target states.
pub proof fn proof_fn_terminal_absorbing()
    ensures
        forall|t: StepState, s: StepState|
            spec_is_terminal(t) && t != s ==> !spec_is_valid_transition(t, s),
{
    // Exhaustive proof over 8-element StepState enum.
    // The SMT solver + by(compute) handles the finite case split.
    assert forall|t: StepState, s: StepState|
        spec_is_terminal(t) && t != s ==> !spec_is_valid_transition(t, s) by {
        // For each (t, s) where t is terminal and t != s:
        // spec_is_valid_transition(t, s) must be false.
        // This is a finite check over 4 × 7 = 28 pairs.
        // The lemma_terminal_absorbing provides the per-pair proof.
        if spec_is_terminal(t) && t != s {
            lemma_terminal_absorbing(t, s);
        }
    };
}

// ===========================================================================
// PO-VERUS-003: Uniform terminal treatment
// ===========================================================================

/// Lemma: All 4 terminal states are uniformly absorbing.
/// For each terminal t, the only valid transition is t → t (idempotent).
pub proof fn lemma_uniform_terminal_treatment()
    ensures
        forall|t: StepState, s: StepState|
            spec_is_terminal(t) && t != s ==> !spec_is_valid_transition(t, s),
{
    assert forall|t: StepState, s: StepState|
        spec_is_terminal(t) && t != s ==> !spec_is_valid_transition(t, s) by {
        if spec_is_terminal(t) && t != s {
            lemma_terminal_absorbing(t, s);
        }
    };
}

/// Spec: terminal_cannot_transition_to_non_terminal — mirrors the proof kernel.
/// Returns true iff every terminal state t has next_states(t) = [t].
/// In the corrected contract, there are no exceptions.
pub open spec fn spec_terminal_cannot_transition_to_non_terminal() -> bool {
    forall|t: StepState| spec_is_terminal(t) ==>
        forall|s: StepState| t != s ==> !spec_is_valid_transition(t, s)
}

/// PO-VERUS-003: Proof that terminal_cannot_transition_to_non_terminal holds.
pub proof fn proof_fn_terminal_cannot_transition_to_non_terminal()
    ensures
        spec_terminal_cannot_transition_to_non_terminal(),
{
    // Unfold and prove the forall over all StepState variants
    assert(spec_terminal_cannot_transition_to_non_terminal()) by {
        assert forall|t: StepState| spec_is_terminal(t) ==>
            forall|s: StepState| t != s ==> !spec_is_valid_transition(t, s) by {
            if spec_is_terminal(t) {
                assert forall|s: StepState| t != s ==> !spec_is_valid_transition(t, s) by {
                    if t != s {
                        lemma_terminal_absorbing(t, s);
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
/// must return true after removal of the Succeeded special case.
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
