//! Deterministic run loop and step budget execution.

use crate::EngineSignal;
use crate::StepBudget;
use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::value_store::ValueStore;
use crate::workflow::CompiledWorkflow;

/// Executes deterministic nodes until finish or budget exhaustion.
pub fn run_until_blocked(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    mut budget: StepBudget,
    store: &mut ValueStore,
) -> Result<EngineSignal, EngineError> {
    drive_deterministic(plan, run, &mut budget, store)
}

/// Executes deterministic nodes until finish, suspension, or budget exhaustion.
pub fn drive_deterministic(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    budget: &mut StepBudget,
    store: &mut ValueStore,
) -> Result<EngineSignal, EngineError> {
    while budget.try_take()? {
        let signal = super::step::step_once(plan, run, store)?;
        if !matches!(signal, EngineSignal::Continue) {
            return Ok(signal);
        }
    }
    Ok(EngineSignal::StepBudgetExhausted)
}
