#![forbid(unsafe_code)]
//! Wait and Ask suspension primitive handlers.

use std::time::{Duration, Instant};
use vb_core::engine::suspension::{validate_ask_inputs, validate_numeric};
use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::SlotValue;

const TIMER_DURATION_RESOURCE: &str = "timer_duration";

fn timer_duration_error() -> EngineError {
    EngineError::ResourceLimitExceeded {
        resource: TIMER_DURATION_RESOURCE,
    }
}

fn timeout_duration(value: SlotValue) -> Result<Duration, EngineError> {
    match value {
        SlotValue::I64(millis) => u64::try_from(millis.max(0))
            .map(Duration::from_millis)
            .map_err(|_| timer_duration_error()),
        SlotValue::F64(millis) if millis.get() > 0.0 => {
            let seconds = millis.get() / 1000.0;
            let duration =
                Duration::try_from_secs_f64(seconds).map_err(|_| timer_duration_error())?;
            let millis = u64::try_from(duration.as_millis()).map_err(|_| timer_duration_error())?;
            Ok(Duration::from_millis(millis))
        }
        _ => Ok(Duration::ZERO),
    }
}

pub(crate) fn checked_timer_deadline(value: SlotValue) -> Result<Instant, EngineError> {
    Instant::now()
        .checked_add(timeout_duration(value)?)
        .ok_or_else(timer_duration_error)
}

fn validate_timer_timeout(value: SlotValue) -> Result<(), EngineError> {
    checked_timer_deadline(value).map(|_| ())
}

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
    validate_timer_timeout(deadline)?;
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
        validate_timer_timeout(timeout_value)?;
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
    validate_ask_inputs(run, prompt, timeout_slot)?;
    if let Some(timeout) = timeout_slot {
        validate_timer_timeout(*run.read_slot(timeout)?)?;
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

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "timer_duration_tests.rs"]
mod timer_duration_tests;
