#![forbid(unsafe_code)]

//! Helper utilities for runtime engine.

use vb_core::engine::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::StepIdx;

use crate::engine::types::RuntimeSignal;

pub fn mark_step_after_signal(
    run: &mut RunFrame,
    step: StepIdx,
    signal: &RuntimeSignal,
) -> Result<(), EngineError> {
    match signal {
        RuntimeSignal::AwaitingWait => run.mark_waiting(step),
        RuntimeSignal::AwaitingAsk => run.mark_asking(step),
        RuntimeSignal::AwaitingAction(_) | RuntimeSignal::StepBudgetExhausted => Ok(()),
        RuntimeSignal::Continue | RuntimeSignal::Finished(_) => run.mark_succeeded(step),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::action::ActionTicket;
    use vb_core::frame::{RunFrame, StepState};
    use vb_core::ids::{RunId, StepIdx};
    use vb_core::value::SlotValue;

    fn fresh_frame() -> RunFrame {
        RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, 4).unwrap()
    }

    fn zeroed_ticket() -> ActionTicket {
        ActionTicket {
            run: RunId::ZERO,
            step: StepIdx::ZERO,
            seq: vb_core::ids::SeqNo::ZERO,
            action: vb_core::ids::ActionId::new(0),
            attempt: 1,
            idempotency_key: 0,
            capacity: 1,
        }
    }

    #[test]
    fn mark_step_awaiting_wait_marks_waiting() {
        let mut frame = fresh_frame();
        let step = StepIdx::new(0);
        frame.mark_running(step).unwrap();
        let result = mark_step_after_signal(&mut frame, step, &RuntimeSignal::AwaitingWait);
        assert_eq!(result, Ok(()));
        assert_eq!(frame.step_state(step), Ok(StepState::Waiting));
    }

    #[test]
    fn mark_step_awaiting_wait_from_pending_fails() {
        let mut frame = fresh_frame();
        let step = StepIdx::new(0);
        let result = mark_step_after_signal(&mut frame, step, &RuntimeSignal::AwaitingWait);
        assert!(result.is_err());
    }

    #[test]
    fn mark_step_awaiting_ask_marks_asking() {
        let mut frame = fresh_frame();
        let step = StepIdx::new(0);
        frame.mark_running(step).unwrap();
        let result = mark_step_after_signal(&mut frame, step, &RuntimeSignal::AwaitingAsk);
        assert_eq!(result, Ok(()));
        assert_eq!(frame.step_state(step), Ok(StepState::Asking));
    }

    #[test]
    fn mark_step_awaiting_ask_from_pending_fails() {
        let mut frame = fresh_frame();
        let step = StepIdx::new(0);
        let result = mark_step_after_signal(&mut frame, step, &RuntimeSignal::AwaitingAsk);
        assert!(result.is_err());
    }

    #[test]
    fn mark_step_awaiting_action_keeps_running_state() {
        let mut frame = fresh_frame();
        let step = StepIdx::new(0);
        frame.mark_running(step).unwrap();
        let result = mark_step_after_signal(&mut frame, step, &RuntimeSignal::AwaitingAction(zeroed_ticket()));
        assert_eq!(result, Ok(()));
        assert_eq!(frame.step_state(step), Ok(StepState::Running));
    }

    #[test]
    fn mark_step_awaiting_action_pending_step_returns_ok() {
        let mut frame = fresh_frame();
        let step = StepIdx::new(0);
        let result = mark_step_after_signal(&mut frame, step, &RuntimeSignal::AwaitingAction(zeroed_ticket()));
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn mark_step_budget_exhausted_keeps_running_state() {
        let mut frame = fresh_frame();
        let step = StepIdx::new(0);
        frame.mark_running(step).unwrap();
        let result = mark_step_after_signal(&mut frame, step, &RuntimeSignal::StepBudgetExhausted);
        assert_eq!(result, Ok(()));
        assert_eq!(frame.step_state(step), Ok(StepState::Running));
    }

    #[test]
    fn mark_step_continue_marks_succeeded() {
        let mut frame = fresh_frame();
        let step = StepIdx::new(0);
        frame.mark_running(step).unwrap();
        let result = mark_step_after_signal(&mut frame, step, &RuntimeSignal::Continue);
        assert_eq!(result, Ok(()));
        assert_eq!(frame.step_state(step), Ok(StepState::Succeeded));
    }

    #[test]
    fn mark_step_continue_from_pending_succeeds() {
        let mut frame = fresh_frame();
        let step = StepIdx::new(0);
        let result = mark_step_after_signal(&mut frame, step, &RuntimeSignal::Continue);
        assert_eq!(result, Ok(()));
        assert_eq!(frame.step_state(step), Ok(StepState::Succeeded));
    }

    #[test]
    fn mark_step_finished_marks_succeeded() {
        let mut frame = fresh_frame();
        let step = StepIdx::new(0);
        frame.mark_running(step).unwrap();
        let result = mark_step_after_signal(&mut frame, step, &RuntimeSignal::Finished(SlotValue::I64(42)));
        assert_eq!(result, Ok(()));
        assert_eq!(frame.step_state(step), Ok(StepState::Succeeded));
    }

    #[test]
    fn mark_step_finished_null_value_marks_succeeded() {
        let mut frame = fresh_frame();
        let step = StepIdx::new(0);
        frame.mark_running(step).unwrap();
        let result = mark_step_after_signal(&mut frame, step, &RuntimeSignal::Finished(SlotValue::Null));
        assert_eq!(result, Ok(()));
        assert_eq!(frame.step_state(step), Ok(StepState::Succeeded));
    }

    #[test]
    fn mark_step_ask_with_nonzero_step() {
        let mut frame = RunFrame::new(RunId::new(5), StepIdx::new(1), 8, 8).unwrap();
        let step = StepIdx::new(3);
        frame.mark_running(step).unwrap();
        let result = mark_step_after_signal(&mut frame, step, &RuntimeSignal::AwaitingAsk);
        assert_eq!(result, Ok(()));
        assert_eq!(frame.step_state(step), Ok(StepState::Asking));
    }

    #[test]
    fn mark_step_continue_with_step_past_first() {
        let mut frame = RunFrame::new(RunId::new(6), StepIdx::new(2), 10, 10).unwrap();
        let step = StepIdx::new(7);
        frame.mark_running(step).unwrap();
        let result = mark_step_after_signal(&mut frame, step, &RuntimeSignal::Continue);
        assert_eq!(result, Ok(()));
        assert_eq!(frame.step_state(step), Ok(StepState::Succeeded));
    }

    #[test]
    fn mark_step_all_variants_no_panic() {
        let signals: [RuntimeSignal; 5] = [
            RuntimeSignal::Continue,
            RuntimeSignal::Finished(SlotValue::I64(1)),
            RuntimeSignal::StepBudgetExhausted,
            RuntimeSignal::AwaitingAction(zeroed_ticket()),
            RuntimeSignal::AwaitingWait,
        ];
        for i in 0u64..5 {
            let mut frame = RunFrame::new(RunId::new(i + 10), StepIdx::ZERO, 4, 4).unwrap();
            let step = StepIdx::new(0);
            frame.mark_running(step).unwrap();
            let _ = mark_step_after_signal(&mut frame, step, &signals[i as usize]);
        }
    }
}
