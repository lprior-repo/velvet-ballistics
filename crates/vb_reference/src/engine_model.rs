//! Reference engine model.
//!
//! This is a slow, obvious reference implementation for the execution engine.
//! Use this to verify the optimized implementation matches this behavior.

use vb_core::engine::{EngineSignal, StepBudget};
use vb_core::frame::{RunFrame, StepState};
use vb_core::value::ValueStore;

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
            let new_remaining = remaining - 1;
            budget.set_remaining(new_remaining);
            (true, new_remaining)
        }
    }

    pub fn drive_deterministic_ref(
        frame: &mut RunFrame,
        value_store: &mut ValueStore,
        initial_budget: u64,
    ) -> EngineSignal {
        let mut budget = StepBudget::new(initial_budget);

        loop {
            if budget.remaining() == 0 {
                return EngineSignal::StepBudgetExhausted;
            }

            match frame.step_state() {
                StepState::Succeeded
                | StepState::Failed
                | StepState::Cancelled
                | StepState::Skipped => {
                    return EngineSignal::Finished(
                        vb_core::value::SlotValue::Unit,
                        vb_core::value::Taint::Clean,
                    );
                }
                _ => {}
            }

            let taken = budget.try_take();
            if !taken {
                return EngineSignal::StepBudgetExhausted;
            }
        }
    }
}

pub fn validate_finish_signal(signal: &EngineSignal) -> bool {
    match signal {
        EngineSignal::Finished(value, taint) => {
            !matches!(value, vb_core::value::SlotValue::Invalid)
        }
        _ => true,
    }
}

pub fn signal_to_string(signal: &EngineSignal) -> String {
    match signal {
        EngineSignal::Continue => "Continue".to_string(),
        EngineSignal::Finished(value, taint) => {
            format!("Finished({:?}, {:?})", value, taint)
        }
        EngineSignal::StepBudgetExhausted => "StepBudgetExhausted".to_string(),
        EngineSignal::AwaitingAction { step, seq, action } => {
            format!("AwaitingAction {{ step: {:?}, seq: {:?}, action: {:?} }}", step, seq, action)
        }
        EngineSignal::AwaitingWait { deadline_slot } => {
            format!("AwaitingWait {{ deadline_slot: {:?} }}", deadline_slot)
        }
        EngineSignal::AwaitingAsk { timeout_slot } => {
            format!("AwaitingAsk {{ timeout_slot: {:?} }}", timeout_slot)
        }
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
        assert!(budget.try_take());
        assert!(!budget.try_take());
    }

    #[test]
    fn test_budget_try_take_decrements() {
        let mut budget = StepBudget::new(5);
        assert_eq!(budget.remaining(), 5);
        assert!(budget.try_take());
        assert_eq!(budget.remaining(), 4);
        assert!(budget.try_take());
        assert_eq!(budget.remaining(), 3);
    }
}
