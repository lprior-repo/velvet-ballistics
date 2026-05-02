#![forbid(unsafe_code)]

//! Main run execution loop and driver functions.

use vb_core::action::ActionContract;
use vb_core::engine::{EngineSignal, StepBudget};
use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::StepIdx;
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;

use crate::engine::signals::{RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};
use crate::engine::step_engine::execute_node_full;
use crate::engine::transition::mark_step_after_signal;

/// Enhanced drive loop that handles all node kinds including
/// iteration, compound, action, and suspension primitives.
pub fn drive_deterministic_full(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    budget: &mut StepBudget,
    store: &mut ValueStore,
    contracts: &[ActionContract],
    retry_policy: crate::engine::RetryPolicy,
) -> RuntimeEngineResult<RuntimeSignal> {
    loop {
        if !budget.try_take().map_err(RuntimeEngineError::Core)? {
            return Ok(RuntimeSignal::StepBudgetExhausted);
        }

        let pc = run.pc();
        let node = plan
            .node(pc)
            .ok_or(EngineError::InvalidProgramCounter { step: pc })?;

        run.mark_running(pc).map_err(RuntimeEngineError::Core)?;

        let signal = execute_node_full(plan, run, store, node, contracts, retry_policy);

        let signal = match signal {
            Ok(signal) => signal,
            Err(error) => {
                run.mark_failed(pc).map_err(RuntimeEngineError::Core)?;
                return Err(error);
            }
        };

        match mark_step_after_signal(run, pc, &signal) {
            Ok(()) => {}
            Err(e) => return Err(RuntimeEngineError::Core(e)),
        }

        match signal {
            RuntimeSignal::Continue => {}
            other => return Ok(other),
        }
    }
}

/// Backward-compatible drive loop matching the original drive_with_actions signature.
pub fn drive_with_actions(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    budget: &mut StepBudget,
    contracts: &[ActionContract],
    retry_policy: crate::engine::RetryPolicy,
) -> RuntimeEngineResult<RuntimeSignal> {
    let mut store = ValueStore::new();
    drive_deterministic_full(plan, run, budget, &mut store, contracts, retry_policy)
}
