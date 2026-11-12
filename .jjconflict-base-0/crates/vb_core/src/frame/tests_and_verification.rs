//! Tests and verification harnesses extracted from frame.rs
//!
//! This module contains all `#[cfg(test)]` and `#[cfg(kani)]` modules that were
//! previously inline in `frame.rs`.

#[cfg(test)]
#[allow(clippy::panic_in_result_fn)]
mod tests {
    use crate::errors::{CoreError, CoreResult};
    use crate::frame::{RunFrame, SlotIdx, SlotValue, StepIdx, StepState, Taint};
    use crate::ids::RunId;

    include!("tests_and_verification/tests_frame_state.rs");
    include!("tests_and_verification/tests_security_parallel.rs");
}

// Kani harnesses for PO-RUST-001-FRAME-KANI: validate_transition 64-pair proof.
// Moved to module level (outside impl RunFrame) so Kani can discover them.
// Uses a minimal inline transition function to avoid CoreResult (CoreError -> Capability drop loop).
#[cfg(kani)]
mod frame_kani_harnesses {
    use crate::frame::{
        RunFrame, SlotIdx, SlotValue, StepIdx, StepState, is_valid_step_state_transition,
    };
    use crate::ids::RunId;
    use crate::value::Taint;

    fn validate_transition_inline(current: StepState, new: StepState) -> bool {
        is_valid_step_state_transition(current, new)
    }

    fn step_state_from_u8(v: u8) -> StepState {
        match v % 8 {
            0 => StepState::Pending,
            1 => StepState::Running,
            2 => StepState::Succeeded,
            3 => StepState::Failed,
            4 => StepState::Skipped,
            5 => StepState::Waiting,
            6 => StepState::Asking,
            _ => StepState::Cancelled,
        }
    }

    include!("tests_and_verification/kani_transition_exhaustive.rs");
    include!("tests_and_verification/kani_transition_rest.rs");
}
