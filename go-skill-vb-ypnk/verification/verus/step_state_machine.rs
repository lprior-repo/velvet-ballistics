// Verus proof obligations for canonical step-state transitions.
//
// Proof-kernel source: `crates/vb_proof_kernels/src/step_state.rs`.
// Runtime refinement target: `crates/vb_core/src/frame.rs`, whose transition
// predicate delegates to the proof-kernel transition function.
// Runtime parity harness: `crates/vb_core/src/kani_step_state_transition.rs`.
// Canonical temporal model: `specs/tla/StepState.tla`.
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

pub open spec fn non_idempotent_transition(current: SpecStepState, next: SpecStepState) -> bool {
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
        SpecStepState::Succeeded => false,
        SpecStepState::Failed => false,
        SpecStepState::Cancelled => false,
        SpecStepState::Skipped => false,
    }
}

pub open spec fn validate_transition(current: SpecStepState, next: SpecStepState) -> bool {
    current == next || non_idempotent_transition(current, next)
}

pub exec fn validate_transition_exec(current: SpecStepState, next: SpecStepState) -> (res: bool)
    ensures res == validate_transition(current, next),
{
    match current {
        SpecStepState::Pending => match next {
            SpecStepState::Pending => true,
            SpecStepState::Running => true,
            SpecStepState::Succeeded => true,
            SpecStepState::Failed => true,
            SpecStepState::Cancelled => true,
            SpecStepState::Skipped => true,
            _ => false,
        },
        SpecStepState::Running => match next {
            SpecStepState::Running => true,
            SpecStepState::Succeeded => true,
            SpecStepState::Failed => true,
            SpecStepState::Waiting => true,
            SpecStepState::Asking => true,
            SpecStepState::Cancelled => true,
            SpecStepState::Skipped => true,
            _ => false,
        },
        SpecStepState::Waiting => match next {
            SpecStepState::Waiting => true,
            SpecStepState::Running => true,
            _ => false,
        },
        SpecStepState::Asking => match next {
            SpecStepState::Asking => true,
            SpecStepState::Running => true,
            _ => false,
        },
        SpecStepState::Succeeded => match next {
            SpecStepState::Succeeded => true,
            _ => false,
        },
        SpecStepState::Failed => match next {
            SpecStepState::Failed => true,
            _ => false,
        },
        SpecStepState::Cancelled => match next {
            SpecStepState::Cancelled => true,
            _ => false,
        },
        SpecStepState::Skipped => match next {
            SpecStepState::Skipped => true,
            _ => false,
        },
    }
}

pub proof fn proof_idempotent_remark_allowed(current: SpecStepState)
    ensures validate_transition(current, current),
{
    assert(validate_transition(current, current)) by(compute);
}

pub proof fn proof_terminal_blocks_outward(current: SpecStepState, next: SpecStepState)
    requires
        is_terminal(current),
        current != next,
    ensures !validate_transition(current, next),
{
    assert((is_terminal(current) && current != next) ==> !validate_transition(current, next)) by(compute);
}

pub proof fn proof_suspended_resumes_only_to_running(current: SpecStepState, next: SpecStepState)
    requires
        is_suspended(current),
        current != next,
        validate_transition(current, next),
    ensures next == SpecStepState::Running,
{
    assert((is_suspended(current) && current != next && validate_transition(current, next)) ==> next == SpecStepState::Running) by(compute);
}

pub proof fn proof_all_pairs()
    ensures
        validate_transition(SpecStepState::Pending, SpecStepState::Pending) == true,
        validate_transition(SpecStepState::Pending, SpecStepState::Running) == true,
        validate_transition(SpecStepState::Pending, SpecStepState::Succeeded) == true,
        validate_transition(SpecStepState::Pending, SpecStepState::Failed) == true,
        validate_transition(SpecStepState::Pending, SpecStepState::Cancelled) == true,
        validate_transition(SpecStepState::Pending, SpecStepState::Skipped) == true,
        validate_transition(SpecStepState::Pending, SpecStepState::Waiting) == false,
        validate_transition(SpecStepState::Pending, SpecStepState::Asking) == false,
        validate_transition(SpecStepState::Running, SpecStepState::Pending) == false,
        validate_transition(SpecStepState::Running, SpecStepState::Running) == true,
        validate_transition(SpecStepState::Running, SpecStepState::Succeeded) == true,
        validate_transition(SpecStepState::Running, SpecStepState::Failed) == true,
        validate_transition(SpecStepState::Running, SpecStepState::Waiting) == true,
        validate_transition(SpecStepState::Running, SpecStepState::Asking) == true,
        validate_transition(SpecStepState::Running, SpecStepState::Cancelled) == true,
        validate_transition(SpecStepState::Running, SpecStepState::Skipped) == true,
        validate_transition(SpecStepState::Waiting, SpecStepState::Waiting) == true,
        validate_transition(SpecStepState::Waiting, SpecStepState::Running) == true,
        validate_transition(SpecStepState::Waiting, SpecStepState::Asking) == false,
        validate_transition(SpecStepState::Asking, SpecStepState::Asking) == true,
        validate_transition(SpecStepState::Asking, SpecStepState::Running) == true,
        validate_transition(SpecStepState::Asking, SpecStepState::Waiting) == false,
        validate_transition(SpecStepState::Succeeded, SpecStepState::Succeeded) == true,
        validate_transition(SpecStepState::Succeeded, SpecStepState::Running) == false,
        validate_transition(SpecStepState::Failed, SpecStepState::Failed) == true,
        validate_transition(SpecStepState::Failed, SpecStepState::Succeeded) == false,
        validate_transition(SpecStepState::Cancelled, SpecStepState::Cancelled) == true,
        validate_transition(SpecStepState::Cancelled, SpecStepState::Running) == false,
        validate_transition(SpecStepState::Skipped, SpecStepState::Skipped) == true,
        validate_transition(SpecStepState::Skipped, SpecStepState::Running) == false,
{
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Pending) == true) by(compute);
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Running) == true) by(compute);
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Succeeded) == true) by(compute);
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Failed) == true) by(compute);
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Cancelled) == true) by(compute);
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Skipped) == true) by(compute);
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Waiting) == false) by(compute);
    assert(validate_transition(SpecStepState::Pending, SpecStepState::Asking) == false) by(compute);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Pending) == false) by(compute);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Running) == true) by(compute);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Succeeded) == true) by(compute);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Failed) == true) by(compute);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Waiting) == true) by(compute);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Asking) == true) by(compute);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Cancelled) == true) by(compute);
    assert(validate_transition(SpecStepState::Running, SpecStepState::Skipped) == true) by(compute);
    assert(validate_transition(SpecStepState::Waiting, SpecStepState::Waiting) == true) by(compute);
    assert(validate_transition(SpecStepState::Waiting, SpecStepState::Running) == true) by(compute);
    assert(validate_transition(SpecStepState::Waiting, SpecStepState::Asking) == false) by(compute);
    assert(validate_transition(SpecStepState::Asking, SpecStepState::Asking) == true) by(compute);
    assert(validate_transition(SpecStepState::Asking, SpecStepState::Running) == true) by(compute);
    assert(validate_transition(SpecStepState::Asking, SpecStepState::Waiting) == false) by(compute);
    assert(validate_transition(SpecStepState::Succeeded, SpecStepState::Succeeded) == true) by(compute);
    assert(validate_transition(SpecStepState::Succeeded, SpecStepState::Running) == false) by(compute);
    assert(validate_transition(SpecStepState::Failed, SpecStepState::Failed) == true) by(compute);
    assert(validate_transition(SpecStepState::Failed, SpecStepState::Succeeded) == false) by(compute);
    assert(validate_transition(SpecStepState::Cancelled, SpecStepState::Cancelled) == true) by(compute);
    assert(validate_transition(SpecStepState::Cancelled, SpecStepState::Running) == false) by(compute);
    assert(validate_transition(SpecStepState::Skipped, SpecStepState::Skipped) == true) by(compute);
    assert(validate_transition(SpecStepState::Skipped, SpecStepState::Running) == false) by(compute);
}

fn main() {}

} // verus!
