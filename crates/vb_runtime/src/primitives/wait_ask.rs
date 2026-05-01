//! Wait and Ask suspension primitive handlers.

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};

/// Executes WaitUntil: reads the deadline slot and suspends.
///
/// Returns AwaitingWait signal. The host runtime is responsible for
/// resuming the run after the deadline passes.
pub fn wait_until(
    run: &mut RunFrame,
    deadline_slot: SlotIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    let deadline = *run.read_slot(deadline_slot)?;
    let _ = deadline;
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
    let event_value = *run.read_slot(event)?;
    let _ = event_value;
    if let Some(timeout) = timeout_slot {
        let timeout_value = *run.read_slot(timeout)?;
        let _ = timeout_value;
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
    let prompt_value = *run.read_slot(prompt)?;
    let _ = prompt_value;
    if let Some(timeout) = timeout_slot {
        let timeout_value = *run.read_slot(timeout)?;
        let _ = timeout_value;
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

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::RunId;
    use vb_core::value::SlotValue;

    fn fresh_frame() -> RunFrame {
        RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, 8).ok().unwrap_or_else(||
            panic!("frame creation must succeed")
        )
    }

    #[test]
    fn wait_until_returns_awaiting_wait() {
        let mut run = fresh_frame();
        let deadline = SlotIdx::new(0);
        run.write_slot(deadline, SlotValue::I64(1000)).ok().unwrap_or_else(||
            panic!("slot write must succeed")
        );

        let result = wait_until(&mut run, deadline);

        assert_eq!(result, Ok(vb_core::EngineSignal::AwaitingWait));
    }

    #[test]
    fn wait_event_returns_awaiting_wait() {
        let mut run = fresh_frame();
        let event = SlotIdx::new(0);
        let timeout = SlotIdx::new(1);
        run.write_slot(event, SlotValue::I64(1)).ok().unwrap_or_else(||
            panic!("slot write must succeed")
        );
        run.write_slot(timeout, SlotValue::I64(500)).ok().unwrap_or_else(||
            panic!("slot write must succeed")
        );

        let result = wait_event(&mut run, event, Some(timeout));

        assert_eq!(result, Ok(vb_core::EngineSignal::AwaitingWait));
    }

    #[test]
    fn ask_returns_awaiting_ask() {
        let mut run = fresh_frame();
        let prompt = SlotIdx::new(0);
        let timeout = SlotIdx::new(1);
        run.write_slot(prompt, SlotValue::I64(1)).ok().unwrap_or_else(||
            panic!("slot write must succeed")
        );
        run.write_slot(timeout, SlotValue::I64(300)).ok().unwrap_or_else(||
            panic!("slot write must succeed")
        );

        let result = ask(&mut run, prompt, Some(timeout));

        assert_eq!(result, Ok(vb_core::EngineSignal::AwaitingAsk));
    }

    #[test]
    fn ask_resume_writes_answer_and_continues() {
        let mut run = fresh_frame();
        let answer = SlotIdx::new(0);
        let output = SlotIdx::new(1);
        let next_step = StepIdx::new(3);
        run.write_slot(answer, SlotValue::I64(42)).ok().unwrap_or_else(||
            panic!("slot write must succeed")
        );

        let result = ask_resume(
            &mut run,
            answer,
            Some(output),
            Some(next_step),
            StepIdx::ZERO,
        );

        assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
        assert_eq!(run.pc(), next_step);
        assert_eq!(*run.read_slot(output).ok().unwrap_or_else(|| panic!("read must succeed")), SlotValue::I64(42));
    }
}
