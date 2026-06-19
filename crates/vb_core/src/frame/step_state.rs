//! Per-step execution state stored in the hot run frame.
//!
//! This enum defines the state machine for individual workflow steps.
//! All terminal states (Succeeded, Failed, Skipped, Cancelled) are self-only:
//! no terminal state transitions back to Running. Loop-body reentry uses
//! the explicit `Succeeded -> Pending` admission path in
//! `RunFrame::mark_pending` followed by `mark_running`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[non_exhaustive]
pub enum StepState {
    /// Step has not been entered.
    Pending,
    /// Step is currently executing.
    Running,
    /// Step completed successfully.
    Succeeded,
    /// Step failed.
    Failed,
    /// Step was skipped by control flow.
    Skipped,
    /// Step is suspended on a wait primitive.
    Waiting,
    /// Step is suspended on an ask primitive.
    Asking,
    /// Step was cancelled.
    Cancelled,
}

/// Pure transition predicate shared by runtime validation and proof harnesses.
///
/// Inlines the step-state machine contract from vb_proof_kernels::step_state.
/// All terminal states (Succeeded, Failed, Cancelled, Skipped) are self-only;
/// no terminal state transitions back to Running. Loop-body reentry uses
/// the explicit `Succeeded -> Pending` admission path in
/// `RunFrame::mark_pending` followed by `mark_running`.
#[must_use]
pub fn is_valid_step_state_transition(current: StepState, new: StepState) -> bool {
    if current == new {
        return true;
    }
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
