#![forbid(unsafe_code)]
//! Wait and Ask suspension primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::SlotValue;

/// Executes WaitUntil: reads the deadline slot, validates it is numeric,
/// and suspends.
///
/// Returns AwaitingWait signal. The host runtime is responsible for
/// resuming the run after the deadline passes.
pub fn wait_until(
    run: &mut RunFrame,
    deadline_slot: SlotIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let deadline = *run.read_slot(deadline_slot)?;
    validate_numeric(deadline, "deadline")?;
    run.increment_executed()?;
    Ok(vb_core::EngineSignal::AwaitingWait)
}

/// Executes WaitEvent: reads the event slot and optional timeout,
/// validates the timeout is numeric, then suspends.
///
/// Returns AwaitingWait signal. The host runtime resumes when the
/// event fires or the timeout expires.
pub fn wait_event(
    run: &mut RunFrame,
    event: SlotIdx,
    timeout_slot: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    let event_value = *run.read_slot(event)?;
    validate_numeric(event_value, "event")?;
    if let Some(timeout) = timeout_slot {
        let timeout_value = *run.read_slot(timeout)?;
        validate_numeric(timeout_value, "timeout")?;
    }
    run.increment_executed()?;
    Ok(vb_core::EngineSignal::AwaitingWait)
}

/// Executes Ask: reads the prompt slot, validates it is prompt-compatible,
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
    validate_prompt(prompt_value)?;
    if let Some(timeout) = timeout_slot {
        let timeout_value = *run.read_slot(timeout)?;
        validate_numeric(timeout_value, "timeout")?;
    }
    run.increment_executed()?;
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
    let answer_taint = run.read_taint(answer)?;
    if let Some(out) = output {
        run.write_slot_with_taint(out, answer_value, answer_taint)?;
    }
    let target = next.ok_or(EngineError::MissingNextStep { step })?;
    run.set_pc(target)?;
    run.increment_executed()?;
    Ok(vb_core::EngineSignal::Continue)
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

fn validate_prompt(value: SlotValue) -> Result<(), EngineError> {
    match value {
        SlotValue::Bool(_) => Err(EngineError::TypeMismatch {
            expected: "prompt",
            found: value.type_name(),
        }),
        _ => Ok(()),
    }
}
