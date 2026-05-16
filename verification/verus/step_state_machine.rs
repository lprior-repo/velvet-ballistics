// Verus proof obligations for canonical step-state transitions.
//
// Source model: `crates/vb_proof_kernels/src/step_state.rs`.
// Registry obligation: VB-CORE-STATE-001.
// Exact verifier command: `verus verification/verus/step_state_machine.rs`.

use vstd::prelude::*;

verus! {

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

pub open spec fn is_terminal(s: SpecStepState) -> bool {
    match s {
        SpecStepState::Succeeded => true,
        SpecStepState::Failed => true,
        SpecStepState::Cancelled => true,
        SpecStepState::Skipped => true,
        _ => false,
    }
}

pub open spec fn is_suspended(s: SpecStepState) -> bool {
    match s {
        SpecStepState::Waiting => true,
        SpecStepState::Asking => true,
        _ => false,
    }
}

pub open spec fn validate_transition(current: SpecStepState, next: SpecStepState) -> bool {
    match current {
        SpecStepState::Pending => match next {
            SpecStepState::Running => true,
            SpecStepState::Succeeded => true,
            SpecStepState::Failed => true,
            SpecStepState::Cancelled => true,
            SpecStepState::Skipped => true,
            _ => false,
        },
        SpecStepState::Running => match next {
            SpecStepState::Succeeded => true,
            SpecStepState::Failed => true,
            SpecStepState::Waiting => true,
            SpecStepState::Asking => true,
            SpecStepState::Cancelled => true,
            SpecStepState::Skipped => true,
            _ => false,
        },
        SpecStepState::Waiting => match next {
            SpecStepState::Running => true,
            _ => false,
        },
        SpecStepState::Asking => match next {
            SpecStepState::Running => true,
            _ => false,
        },
        SpecStepState::Succeeded => next == SpecStepState::Succeeded,
        SpecStepState::Failed => next == SpecStepState::Failed,
        SpecStepState::Cancelled => next == SpecStepState::Cancelled,
        SpecStepState::Skipped => next == SpecStepState::Skipped,
    }
}

pub proof fn lemma_pending_targets(next: SpecStepState)
    requires validate_transition(SpecStepState::Pending, next),
    ensures
        next == SpecStepState::Running
        || next == SpecStepState::Succeeded
        || next == SpecStepState::Failed
        || next == SpecStepState::Cancelled
        || next == SpecStepState::Skipped,
{
    assert(validate_transition(SpecStepState::Pending, next) ==> (
        next == SpecStepState::Running
        || next == SpecStepState::Succeeded
        || next == SpecStepState::Failed
        || next == SpecStepState::Cancelled
        || next == SpecStepState::Skipped
    )) by(compute);
}

pub proof fn lemma_running_targets(next: SpecStepState)
    requires validate_transition(SpecStepState::Running, next),
    ensures
        next == SpecStepState::Succeeded
        || next == SpecStepState::Failed
        || next == SpecStepState::Waiting
        || next == SpecStepState::Asking
        || next == SpecStepState::Cancelled
        || next == SpecStepState::Skipped,
{
    assert(validate_transition(SpecStepState::Running, next) ==> (
        next == SpecStepState::Succeeded
        || next == SpecStepState::Failed
        || next == SpecStepState::Waiting
        || next == SpecStepState::Asking
        || next == SpecStepState::Cancelled
        || next == SpecStepState::Skipped
    )) by(compute);
}

pub proof fn lemma_suspended_targets(current: SpecStepState, next: SpecStepState)
    requires is_suspended(current), validate_transition(current, next),
    ensures next == SpecStepState::Running,
{
    assert((is_suspended(current) && validate_transition(current, next)) ==> next == SpecStepState::Running) by(compute);
}

pub proof fn lemma_terminal_idempotency(current: SpecStepState)
    requires is_terminal(current),
    ensures validate_transition(current, current),
{
    assert(is_terminal(current) ==> validate_transition(current, current)) by(compute);
}

pub proof fn lemma_terminal_blocking(current: SpecStepState, next: SpecStepState)
    requires is_terminal(current), current != next,
    ensures !validate_transition(current, next),
{
    assert((is_terminal(current) && current != next) ==> !validate_transition(current, next)) by(compute);
}

pub proof fn lemma_non_terminal_self_rejected(current: SpecStepState)
    requires !is_terminal(current),
    ensures !validate_transition(current, current),
{
    assert(!is_terminal(current) ==> !validate_transition(current, current)) by(compute);
}

pub proof fn lemma_running_self_rejected()
    ensures !validate_transition(SpecStepState::Running, SpecStepState::Running),
{
    assert(!validate_transition(SpecStepState::Running, SpecStepState::Running)) by(compute);
}

pub proof fn lemma_all_pairs()
    ensures
        validate_transition(SpecStepState::Pending, SpecStepState::Pending) == false,
        validate_transition(SpecStepState::Pending, SpecStepState::Running) == true,
        validate_transition(SpecStepState::Pending, SpecStepState::Succeeded) == true,
        validate_transition(SpecStepState::Pending, SpecStepState::Failed) == true,
        validate_transition(SpecStepState::Pending, SpecStepState::Cancelled) == true,
        validate_transition(SpecStepState::Pending, SpecStepState::Skipped) == true,
        validate_transition(SpecStepState::Pending, SpecStepState::Waiting) == false,
        validate_transition(SpecStepState::Pending, SpecStepState::Asking) == false,
        validate_transition(SpecStepState::Running, SpecStepState::Pending) == false,
        validate_transition(SpecStepState::Running, SpecStepState::Running) == false,
        validate_transition(SpecStepState::Running, SpecStepState::Succeeded) == true,
        validate_transition(SpecStepState::Running, SpecStepState::Failed) == true,
        validate_transition(SpecStepState::Running, SpecStepState::Waiting) == true,
        validate_transition(SpecStepState::Running, SpecStepState::Asking) == true,
        validate_transition(SpecStepState::Running, SpecStepState::Cancelled) == true,
        validate_transition(SpecStepState::Running, SpecStepState::Skipped) == true,
        validate_transition(SpecStepState::Waiting, SpecStepState::Running) == true,
        validate_transition(SpecStepState::Waiting, SpecStepState::Waiting) == false,
        validate_transition(SpecStepState::Asking, SpecStepState::Running) == true,
        validate_transition(SpecStepState::Asking, SpecStepState::Asking) == false,
        validate_transition(SpecStepState::Succeeded, SpecStepState::Succeeded) == true,
        validate_transition(SpecStepState::Succeeded, SpecStepState::Running) == false,
        validate_transition(SpecStepState::Failed, SpecStepState::Failed) == true,
        validate_transition(SpecStepState::Failed, SpecStepState::Succeeded) == false,
        validate_transition(SpecStepState::Cancelled, SpecStepState::Cancelled) == true,
        validate_transition(SpecStepState::Skipped, SpecStepState::Skipped) == true,
        validate_transition(SpecStepState::Skipped, SpecStepState::Running) == false,
{
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Pending) == false) by(compute);
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Running) == true) by(compute);
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Succeeded) == true) by(compute);
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Failed) == true) by(compute);
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Cancelled) == true) by(compute);
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Skipped) == true) by(compute);
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Waiting) == false) by(compute);
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Asking) == false) by(compute);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Pending) == false) by(compute);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Running) == false) by(compute);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Succeeded) == true) by(compute);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Failed) == true) by(compute);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Waiting) == true) by(compute);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Asking) == true) by(compute);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Cancelled) == true) by(compute);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Skipped) == true) by(compute);
    assert(validate_transition(SpecStepState::Waiting, SpecStepState::Running) == true) by(compute);
    assert(validate_transition(SpecStepState::Waiting, SpecStepState::Waiting) == false) by(compute);
    assert(validate_transition(SpecStepState::Asking, SpecStepState::Running) == true) by(compute);
    assert(validate_transition(SpecStepState::Asking, SpecStepState::Asking) == false) by(compute);
    assert(validate_transition(SpecStepState::Succeeded, SpecStepState::Succeeded) == true) by(compute);
    assert(validate_transition(SpecStepState::Succeeded, SpecStepState::Running) == false) by(compute);
    assert(validate_transition(SpecStepState::Failed, SpecStepState::Failed) == true) by(compute);
    assert(validate_transition(SpecStepState::Failed, SpecStepState::Succeeded) == false) by(compute);
    assert(validate_transition(SpecStepState::Cancelled, SpecStepState::Cancelled) == true) by(compute);
    assert(validate_transition(SpecStepState::Skipped, SpecStepState::Skipped) == true) by(compute);
    assert(validate_transition(SpecStepState::Skipped, SpecStepState::Running) == false) by(compute);
}

fn main() {}

} // verus!
