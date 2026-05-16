//! Ask suspension primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::SlotValue;

/// Executes Ask: reads the prompt slot, validates it is a Symbol,
/// and optional timeout validated as numeric, creates an ask ticket,
/// and suspends.
///
/// Returns AwaitingAsk signal. The host runtime presents the prompt
/// to the user and resumes with the answer.
pub fn ask(
    run: &mut RunFrame,
    prompt: SlotIdx,
    timeout_slot: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    let prompt_value = *run.read_slot(prompt)?;
    validate_symbol(prompt_value, "prompt")?;
    if let Some(timeout) = timeout_slot {
        let timeout_value = *run.read_slot(timeout)?;
        validate_numeric(timeout_value, "timeout")?;
    }
    Ok(vb_core::EngineSignal::AwaitingAsk)
}

/// Executes AskResume: validates the answer slot is populated
/// and continues execution.
///
/// The host runtime writes the answer to the answer slot before
/// resuming. This primitive reads and validates the answer.
pub fn ask_resume(
    run: &mut RunFrame,
    answer: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let answer_value = *run.read_slot(answer)?;
    if let Some(out) = output {
        run.write_slot(out, answer_value)?;
    }
    let target = next.ok_or(EngineError::MissingNextStep { step })?;
    run.set_pc(target)?;
    run.increment_executed()?;
    Ok(vb_core::EngineSignal::Continue)
}

fn validate_symbol(value: SlotValue, expected: &'static str) -> Result<(), EngineError> {
    match value {
        SlotValue::Symbol(_) => Ok(()),
        other => Err(EngineError::TypeMismatch {
            expected,
            found: other.type_name(),
        }),
    }
}

fn validate_numeric(value: SlotValue, expected: &'static str) -> Result<(), EngineError> {
    match value {
        SlotValue::I64(_) | SlotValue::F64(_) => Ok(()),
        other => Err(EngineError::TypeMismatch {
            expected,
            found: other.type_name(),
        }),
    }
}
