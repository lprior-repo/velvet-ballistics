// Helpers used by every K-F proof in `frame/parts/kani_*.rs`. These are
// `include!`-d into the `frame` module scope by `frame.rs:126`, so they live
// at frame-module scope (not inside a private submodule). Sibling kani
// files (`kani_f1_exhaustive.rs`, `kani_f2345_transitions.rs`, etc.) call
// `validate_transition_inline` and `step_state_from_u8` directly. The
// `use crate::frame::*` and `use crate::ids::RunId` imports previously
// present here are unused because the file is already inside `frame` module
// scope where these names are in scope from `frame.rs:40-42`.

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

