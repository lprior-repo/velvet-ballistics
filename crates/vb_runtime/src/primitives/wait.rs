//! Wait suspension primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::SlotIdx;
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
    Ok(vb_core::EngineSignal::AwaitingWait)
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
