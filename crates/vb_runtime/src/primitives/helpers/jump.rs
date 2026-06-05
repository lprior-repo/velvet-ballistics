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

/// Jumps to a step body with a pure PC jump.
///
/// Terminal states (Succeeded, Failed, Cancelled, Skipped) are fully absorbing.
/// jump_to_body performs a PC jump and executed-counter increment without any
/// state mutation. The engine (step_once) handles body re-entry by skipping
/// mark_running for already-Succeeded steps, allowing idempotent re-execution
/// while preserving the absorbing invariant.
pub(crate) fn jump_to_body(
    run: &mut RunFrame,
    body: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
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
