//! Reference engine model.
//!
//! This is a slow, obvious reference implementation for the execution engine.
//! Use this to verify the optimized implementation matches this behavior.

use vb_core::engine::{EngineSignal, StepBudget};
use vb_core::frame::{RunFrame, StepState};
use vb_core::ids::StepIdx;
use vb_core::value_store::ValueStore;
use vb_core::{CoreResult, Taint};

#[derive(Default)]
pub struct EngineModel;

impl EngineModel {
    pub fn new() -> Self {
        EngineModel
    }

    pub fn step_budget_new(budget: u64) -> StepBudget {
        StepBudget::new(budget)
    }

    pub fn step_budget_try_take(budget: &mut StepBudget) -> (bool, u64) {
        let remaining = budget.remaining();
        if remaining == 0 {
            (false, 0)
        } else {
            let new_remaining = remaining.saturating_sub(1);
            *budget = StepBudget::new(new_remaining);
            (true, new_remaining)
        }
    }

    pub fn drive_deterministic_ref(
        frame: &mut RunFrame,
        _value_store: &mut ValueStore,
        initial_budget: u64,
    ) -> CoreResult<EngineSignal> {
        let mut budget = StepBudget::new(initial_budget);

        loop {
            if budget.remaining() == 0 {
                return Ok(EngineSignal::StepBudgetExhausted);
            }

            match frame.step_state(StepIdx::ZERO)? {
                StepState::Succeeded
                | StepState::Failed
                | StepState::Cancelled
                | StepState::Skipped => {
                    return Ok(EngineSignal::Finished(
                        vb_core::value::SlotValue::Null,
                        Taint::Clean,
                    ));
                }
                _ => {}
            }

            let taken = budget.try_take()?;
            if !taken {
                return Ok(EngineSignal::StepBudgetExhausted);
            }
        }
    }
}

/// Every [`vb_core::value::SlotValue`] is a valid finish value: the value model
/// carries no invalid sentinel, so any finished signal validates.
pub fn validate_finish_signal(_signal: &EngineSignal) -> bool {
    true
}

pub fn signal_to_string(signal: &EngineSignal) -> String {
    match signal {
        EngineSignal::Continue => "Continue".to_string(),
        EngineSignal::Finished(value, taint) => {
            format!("Finished({:?}, {:?})", value, taint)
        }
        EngineSignal::StepBudgetExhausted => "StepBudgetExhausted".to_string(),
        EngineSignal::AwaitingAction => "AwaitingAction".to_string(),
        EngineSignal::ActionFailureUnhandled => "ActionFailureUnhandled".to_string(),
        EngineSignal::AwaitingWait => "AwaitingWait".to_string(),
        EngineSignal::AwaitingAsk => "AwaitingAsk".to_string(),
        _ => "Unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_zero_returns_exhausted() {
        let budget = StepBudget::new(0);
        assert_eq!(budget.remaining(), 0);
    }

    #[test]
    fn test_budget_one_allows_one_take() {
        let mut budget = StepBudget::new(1);
        assert_eq!(budget.try_take(), Ok(true));
        assert_eq!(budget.try_take(), Ok(false));
    }

    #[test]
    fn test_budget_try_take_decrements() {
        let mut budget = StepBudget::new(5);
        assert_eq!(budget.remaining(), 5);
        assert_eq!(budget.try_take(), Ok(true));
        assert_eq!(budget.remaining(), 4);
        assert_eq!(budget.try_take(), Ok(true));
        assert_eq!(budget.remaining(), 3);
    }
}
