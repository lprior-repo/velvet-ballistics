//! Verus proof obligations for frame state transition soundness.
//!
//! Source: `crates/vb_core/src/frame.rs` lines 10-28, 394-431
//!
//! PO-VERUS-001: step_state_transition_soundness
//!
//! Self-contained. Uses spec fn for all pure spec functions.

use vstd::prelude::*;

verus! {

// ─────────────────────────────────────────────────────────────────────────────
// Spec-level type mirroring Rust StepState (8 variants)
// ─────────────────────────────────────────────────────────────────────────────

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

// ─────────────────────────────────────────────────────────────────────────────
// Spec fns: pure boolean helpers
// ─────────────────────────────────────────────────────────────────────────────

spec fn eq_state(a: SpecStepState, b: SpecStepState) -> bool {
    match a {
        SpecStepState::Pending    => match b { SpecStepState::Pending    => true, _ => false },
        SpecStepState::Running    => match b { SpecStepState::Running    => true, _ => false },
        SpecStepState::Succeeded  => match b { SpecStepState::Succeeded  => true, _ => false },
        SpecStepState::Failed     => match b { SpecStepState::Failed     => true, _ => false },
        SpecStepState::Skipped    => match b { SpecStepState::Skipped    => true, _ => false },
        SpecStepState::Waiting    => match b { SpecStepState::Waiting    => true, _ => false },
        SpecStepState::Asking    => match b { SpecStepState::Asking    => true, _ => false },
        SpecStepState::Cancelled  => match b { SpecStepState::Cancelled  => true, _ => false },
    }
}

spec fn is_terminal(s: SpecStepState) -> bool {
    match s {
        SpecStepState::Succeeded => true,
        SpecStepState::Failed    => true,
        SpecStepState::Cancelled=> true,
        SpecStepState::Skipped  => true,
        _ => false,
    }
}

spec fn is_running(s: SpecStepState) -> bool {
    match s { SpecStepState::Running => true, _ => false }
}

spec fn is_pending(s: SpecStepState) -> bool {
    match s { SpecStepState::Pending => true, _ => false }
}

spec fn is_suspended(s: SpecStepState) -> bool {
    match s {
        SpecStepState::Waiting => true,
        SpecStepState::Asking => true,
        _ => false,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Spec fn: transition validator (mirrors Rust validate_transition 8×8 matrix)
// ─────────────────────────────────────────────────────────────────────────────

spec fn validate_transition(current: SpecStepState, next: SpecStepState) -> bool {
    if is_pending(current) {
        // Pending → {Running, Succeeded, Failed, Cancelled, Skipped} (not Waiting/Asking/self)
        match next {
            SpecStepState::Running   => true,
            SpecStepState::Succeeded=> true,
            SpecStepState::Failed   => true,
            SpecStepState::Cancelled=> true,
            SpecStepState::Skipped  => true,
            _ => false,
        }
    } else if is_running(current) {
        // Running → {Succeeded, Failed, Waiting, Asking, Cancelled, Skipped, self}
        match next {
            SpecStepState::Running   => true,
            SpecStepState::Succeeded=> true,
            SpecStepState::Failed   => true,
            SpecStepState::Waiting  => true,
            SpecStepState::Asking  => true,
            SpecStepState::Cancelled=> true,
            SpecStepState::Skipped  => true,
            _ => false,
        }
    } else if is_terminal(current) {
        // Terminal states: only self allowed
        eq_state(current, next)
    } else if is_suspended(current) {
        // Suspended: Running or self
        eq_state(current, next) || eq_state(next, SpecStepState::Running)
    } else {
        false
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Proof lemmas
// ─────────────────────────────────────────────────────────────────────────────

/// Totality: every pair is classified (True or False).
pub proof fn lemma_totality()
    ensures
        forall|c: SpecStepState, n: SpecStepState|
            validate_transition(c, n) == true || validate_transition(c, n) == false,
{}

/// Determinism: equal inputs → equal outputs.
pub proof fn lemma_determinism(c1: SpecStepState, n1: SpecStepState, c2: SpecStepState, n2: SpecStepState)
    requires c1 == c2 && n1 == n2,
    ensures validate_transition(c1, n1) == validate_transition(c2, n2),
{}

/// Idempotency: self-transition is always allowed except for Pending.
pub proof fn lemma_idempotency(s: SpecStepState)
    ensures is_pending(s) == false ==> validate_transition(s, s) == true,
{
    // Terminal and suspend cases: eq_state(s, s) = true.
    // Running: explicit self-loop arm = true.
    // Pending: self is false (by design).
}

/// Terminal blocking: terminal states block all non-self transitions.
pub proof fn lemma_terminal_blocking(current: SpecStepState, next: SpecStepState)
    requires is_terminal(current) && !eq_state(current, next),
    ensures validate_transition(current, next) == false,
{}

/// Pending targets: Pending can only go to allowed variants.
pub proof fn lemma_pending_targets(current: SpecStepState, next: SpecStepState)
    requires is_pending(current) && validate_transition(current, next) == true,
    ensures
        eq_state(next, SpecStepState::Running)
        || eq_state(next, SpecStepState::Succeeded)
        || eq_state(next, SpecStepState::Failed)
        || eq_state(next, SpecStepState::Cancelled)
        || eq_state(next, SpecStepState::Skipped),
{}

/// Running targets: Running can go to terminal/suspend variants or self.
pub proof fn lemma_running_targets(current: SpecStepState, next: SpecStepState)
    requires is_running(current) && validate_transition(current, next) == true,
    ensures
        eq_state(next, SpecStepState::Running)
        || eq_state(next, SpecStepState::Succeeded)
        || eq_state(next, SpecStepState::Failed)
        || eq_state(next, SpecStepState::Waiting)
        || eq_state(next, SpecStepState::Asking)
        || eq_state(next, SpecStepState::Cancelled)
        || eq_state(next, SpecStepState::Skipped),
{}

/// Suspended targets: Waiting and Asking can only go to Running or self.
pub proof fn lemma_suspended_targets(current: SpecStepState, next: SpecStepState)
    requires is_suspended(current) && validate_transition(current, next) == true,
    ensures eq_state(next, SpecStepState::Running) || eq_state(next, current),
{}

/// All 64 pairs: spot-checks for key allowed/disallowed pairs.
pub proof fn lemma_all_pairs()
    ensures
        // Pending: self is FALSE
        validate_transition(SpecStepState::Pending, SpecStepState::Pending) == false,
        // Pending → allowed
        validate_transition(SpecStepState::Pending, SpecStepState::Running) == true,
        validate_transition(SpecStepState::Pending, SpecStepState::Succeeded) == true,
        validate_transition(SpecStepState::Pending, SpecStepState::Failed) == true,
        validate_transition(SpecStepState::Pending, SpecStepState::Cancelled) == true,
        validate_transition(SpecStepState::Pending, SpecStepState::Skipped) == true,
        // Pending → disallowed
        validate_transition(SpecStepState::Pending, SpecStepState::Waiting) == false,
        validate_transition(SpecStepState::Pending, SpecStepState::Asking) == false,
        // Running self is TRUE
        validate_transition(SpecStepState::Running, SpecStepState::Running) == true,
        // Running → allowed
        validate_transition(SpecStepState::Running, SpecStepState::Succeeded) == true,
        validate_transition(SpecStepState::Running, SpecStepState::Failed) == true,
        validate_transition(SpecStepState::Running, SpecStepState::Waiting) == true,
        validate_transition(SpecStepState::Running, SpecStepState::Asking) == true,
        validate_transition(SpecStepState::Running, SpecStepState::Cancelled) == true,
        validate_transition(SpecStepState::Running, SpecStepState::Skipped) == true,
        // Running → disallowed
        validate_transition(SpecStepState::Running, SpecStepState::Pending) == false,
        // Terminal: self TRUE, others FALSE
        validate_transition(SpecStepState::Succeeded, SpecStepState::Succeeded) == true,
        validate_transition(SpecStepState::Succeeded, SpecStepState::Running) == false,
        validate_transition(SpecStepState::Succeeded, SpecStepState::Failed) == false,
        validate_transition(SpecStepState::Failed, SpecStepState::Failed) == true,
        validate_transition(SpecStepState::Failed, SpecStepState::Succeeded) == false,
        validate_transition(SpecStepState::Cancelled, SpecStepState::Cancelled) == true,
        validate_transition(SpecStepState::Skipped, SpecStepState::Skipped) == true,
        validate_transition(SpecStepState::Skipped, SpecStepState::Running) == false,
        // Suspended → Running or self
        validate_transition(SpecStepState::Waiting, SpecStepState::Running) == true,
        validate_transition(SpecStepState::Waiting, SpecStepState::Waiting) == true,
        validate_transition(SpecStepState::Waiting, SpecStepState::Succeeded) == false,
        validate_transition(SpecStepState::Asking, SpecStepState::Running) == true,
        validate_transition(SpecStepState::Asking, SpecStepState::Asking) == true,
        validate_transition(SpecStepState::Asking, SpecStepState::Failed) == false,
{}

} // verus!