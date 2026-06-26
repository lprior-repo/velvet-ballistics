#![forbid(unsafe_code)]

//! Helper utilities for runtime engine.

use vb_core::frame::RunFrame;
use vb_core::ids::StepIdx;

use crate::engine::types::{RuntimeEngineError, RuntimeSignal};

/// Marks the run frame step state after a signal is produced.
///
/// `UnknownEngineSignal` is never silently absorbed: the helper returns
/// [`RuntimeEngineError::UnknownEngineSignal`] so the drive loop
/// aborts instead of advancing the step state for an unmapped core
/// engine variant (master §45 invalid_state_transition contract).
pub fn mark_step_after_signal(
    run: &mut RunFrame,
    step: StepIdx,
    signal: &RuntimeSignal,
) -> Result<(), RuntimeEngineError> {
    match signal {
        RuntimeSignal::AwaitingWait => run.mark_waiting(step).map_err(RuntimeEngineError::Core),
        RuntimeSignal::AwaitingAsk => run.mark_asking(step).map_err(RuntimeEngineError::Core),
        RuntimeSignal::AwaitingAction(_) | RuntimeSignal::StepBudgetExhausted => Ok(()),
        RuntimeSignal::Continue | RuntimeSignal::Finished(_) => {
            run.mark_succeeded(step).map_err(RuntimeEngineError::Core)
        }
        RuntimeSignal::UnknownEngineSignal { signal_debug } => {
            Err(RuntimeEngineError::UnknownEngineSignal {
                signal_debug: signal_debug.clone(),
            })
        }
    }
}
