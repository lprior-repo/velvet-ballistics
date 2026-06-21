#![forbid(unsafe_code)]

//! Helper utilities for runtime engine.

use vb_core::engine::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::StepIdx;

use crate::engine::types::RuntimeSignal;

/// Marks the run frame step state after a signal is produced.
pub fn mark_step_after_signal(
    run: &mut RunFrame,
    step: StepIdx,
    signal: &RuntimeSignal,
) -> Result<(), EngineError> {
    match signal {
        RuntimeSignal::AwaitingWait(_) | RuntimeSignal::AwaitingEvent { .. } => {
            run.mark_waiting(step)
        }
        RuntimeSignal::AwaitingAsk(_) => run.mark_asking(step),
        RuntimeSignal::AwaitingAction(_) | RuntimeSignal::StepBudgetExhausted => Ok(()),
        RuntimeSignal::Continue | RuntimeSignal::Finished(_) => run.mark_succeeded(step),
    }
}
