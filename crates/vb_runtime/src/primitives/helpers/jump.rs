#![forbid(unsafe_code)]
//! Jump helper functions.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::StepIdx;

/// Sets the program counter to `target` and increments the executed counter.
pub(crate) fn jump_to(
    run: &mut RunFrame,
    target: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    run.set_pc(target)?;
    run.increment_executed()?;
    Ok(vb_core::EngineSignal::Continue)
}

/// Jumps to a step body, explicitly admitting a completed body through Pending
/// before marking it [`Running`][vb_core::frame::StepState::Running].
///
/// Terminal states (`Failed`, `Cancelled`, `Skipped`) are absorbing and
/// re-entering them via a body jump is an internal invariant violation —
/// return [`EngineError::InternalInvariantViolation`] for those cases so
/// the lifecycle FSM rejects the path instead of silently pointing the
/// program counter at an unexecutable step. Only `Succeeded` admits a
/// reset to `Pending -> Running` via the explicit admission path used by
/// loop re-entry (RQ-W0-13, RP-013).
///
/// [`Running`][vb_core::frame::StepState::Running]: vb_core::frame::StepState::Running
pub(crate) fn jump_to_body(
    run: &mut RunFrame,
    body: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let current = run.step_state(body)?;
    match current {
        vb_core::frame::StepState::Succeeded => {
            run.mark_pending(body)?;
            run.mark_running(body)?;
            jump_to(run, body)
        }
        vb_core::frame::StepState::Failed
        | vb_core::frame::StepState::Cancelled
        | vb_core::frame::StepState::Skipped => Err(EngineError::InternalInvariantViolation {
            reason: "jump_to_body on terminal step",
        }),
        // Pending, Running, Waiting, Asking all permit a body jump without
        // re-admission: the engine already owns the step lifecycle.
        _ => jump_to(run, body),
    }
}

/// Jumps to the next step, which must be present.
pub(crate) fn jump_to_next(
    run: &mut RunFrame,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let target = next.ok_or(EngineError::MissingNextStep { step })?;
    jump_to(run, target)
}
