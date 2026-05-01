//! Wait and Ask suspension primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::SlotValue;

/// Executes WaitUntil: reads the deadline slot and suspends.
///
/// Returns AwaitingWait signal. The host runtime is responsible for
/// resuming the run after the deadline passes.
pub fn wait_until(
    run: &mut RunFrame,
    deadline_slot: SlotIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    expect_number(*run.read_slot(deadline_slot)?)?;
    Ok(vb_core::EngineSignal::AwaitingWait)
}

/// Executes WaitEvent: reads the event slot and optional timeout,
/// then suspends.
///
/// Returns AwaitingWait signal. The host runtime resumes when the
/// event fires or the timeout expires.
pub fn wait_event(
    run: &mut RunFrame,
    event: SlotIdx,
    timeout_slot: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    expect_symbol(*run.read_slot(event)?)?;
    if let Some(timeout) = timeout_slot {
        expect_number(*run.read_slot(timeout)?)?;
    }
    Ok(vb_core::EngineSignal::AwaitingWait)
}

/// Executes Ask: reads the prompt slot and optional timeout,
/// creates an ask ticket, and suspends.
///
/// Returns AwaitingAsk signal. The host runtime presents the prompt
/// to the user and resumes with the answer.
pub fn ask(
    run: &mut RunFrame,
    prompt: SlotIdx,
    timeout_slot: Option<SlotIdx>,
) -> Result<vb_core::EngineSignal, EngineError> {
    expect_symbol(*run.read_slot(prompt)?)?;
    if let Some(timeout) = timeout_slot {
        expect_number(*run.read_slot(timeout)?)?;
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
    run.set_pc(target);
    run.increment_executed()?;
    Ok(vb_core::EngineSignal::Continue)
}

fn expect_number(value: SlotValue) -> Result<(), EngineError> {
    match value {
        SlotValue::I64(_) | SlotValue::F64(_) => Ok(()),
        other => Err(EngineError::TypeMismatch {
            expected: "number",
            found: other.type_name(),
        }),
    }
}

fn expect_symbol(value: SlotValue) -> Result<(), EngineError> {
    match value {
        SlotValue::Symbol(_) => Ok(()),
        other => Err(EngineError::TypeMismatch {
            expected: "symbol",
            found: other.type_name(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::EngineSignal;
    use vb_core::RunId;
    use vb_core::ids::SymbolId;

    fn frame() -> Result<RunFrame, EngineError> {
        RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 4)
    }

    #[test]
    fn wait_until_accepts_numeric_deadline() -> Result<(), EngineError> {
        let mut run = frame()?;
        run.write_slot(SlotIdx::new(0), SlotValue::I64(100))?;

        let signal = wait_until(&mut run, SlotIdx::new(0))?;

        assert_eq!(signal, EngineSignal::AwaitingWait);
        Ok(())
    }

    #[test]
    fn wait_until_rejects_non_numeric_deadline() -> Result<(), EngineError> {
        let mut run = frame()?;
        run.write_slot(SlotIdx::new(0), SlotValue::Bool(true))?;

        let result = wait_until(&mut run, SlotIdx::new(0));

        assert!(matches!(
            result,
            Err(EngineError::TypeMismatch {
                expected: "number",
                found: "boolean"
            })
        ));
        Ok(())
    }

    #[test]
    fn ask_accepts_symbol_prompt_and_numeric_timeout() -> Result<(), EngineError> {
        let mut run = frame()?;
        run.write_slot(SlotIdx::new(0), SlotValue::Symbol(SymbolId::new(7)))?;
        run.write_slot(SlotIdx::new(1), SlotValue::I64(30))?;

        let signal = ask(&mut run, SlotIdx::new(0), Some(SlotIdx::new(1)))?;

        assert_eq!(signal, EngineSignal::AwaitingAsk);
        Ok(())
    }

    #[test]
    fn ask_rejects_non_symbol_prompt() -> Result<(), EngineError> {
        let mut run = frame()?;
        run.write_slot(SlotIdx::new(0), SlotValue::I64(30))?;

        let result = ask(&mut run, SlotIdx::new(0), None);

        assert!(matches!(
            result,
            Err(EngineError::TypeMismatch {
                expected: "symbol",
                found: "number"
            })
        ));
        Ok(())
    }
}
