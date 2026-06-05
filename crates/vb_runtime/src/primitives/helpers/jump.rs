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

/// Jumps to a step body, marking it [`Running`][vb_core::frame::StepState::Running]
/// if it has completed (Succeeded) to allow loop body re-entry.
///
/// Terminal states (Succeeded, Failed, Cancelled, Skipped) are absorbing for
/// normal transitions, but Succeeded->Running is allowed for loop body re-entry.
///
/// [`Running`][vb_core::frame::StepState::Running]: vb_core::frame::StepState::Running
pub(crate) fn jump_to_body(
    run: &mut RunFrame,
    body: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let current = run.step_state(body)?;
    if current == vb_core::frame::StepState::Succeeded {
        run.mark_running(body)?;
    }
    jump_to(run, body)
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
