#![forbid(unsafe_code)]

//! Deterministic drive loop for runtime engine.

use vb_core::action::ActionContract;
use vb_core::engine::{EngineError, StepBudget};
use vb_core::frame::RunFrame;
use vb_core::ids::StepIdx;
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;

use crate::engine::execute::execute_node_full;
use crate::engine::helpers::mark_step_after_signal;
use crate::engine::signal::runtime_from_core;
use crate::engine::types::{
    EvidenceCollector, RetryPolicy, RuntimeEngineError, RuntimeEngineResult, RuntimeSignal,
};
use crate::primitives;

/// Enhanced drive loop that handles all node kinds including
/// iteration, compound, action, and suspension primitives.
///
/// Collects evidence events (StepStarted/StepSucceeded) for every step
/// executed during the drive loop. The caller drains these events to emit
/// them to the journal and trace ring.
pub fn drive_deterministic_full(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    budget: &mut StepBudget,
    store: &mut ValueStore,
    contracts: &[ActionContract],
    retry_policy: RetryPolicy,
    evidence: &mut EvidenceCollector,
) -> RuntimeEngineResult<RuntimeSignal> {
    loop {
        if !budget.try_take().map_err(RuntimeEngineError::Core)? {
            return Ok(RuntimeSignal::StepBudgetExhausted);
        }

        let pc = run.pc();
        let node = plan
            .node(pc)
            .ok_or(EngineError::InvalidProgramCounter { step: pc })?;

        // Evidence chain: emit StepStarted before execution.
        evidence.push_step_started(pc);

        run.mark_running(pc).map_err(RuntimeEngineError::Core)?;

        let signal = execute_node_full(plan, run, store, node, contracts, retry_policy)?;

        match mark_step_after_signal(run, pc, &signal) {
            Ok(()) => {}
            Err(e) => return Err(RuntimeEngineError::Core(e)),
        }

        // Evidence chain: emit StepSucceeded after execution with output slot.
        // Only nodes with an explicit output slot produce SlotWritten events.
        // Boundary nodes (Finish, Jump, Nop) have no output slot.
        evidence.push_step_succeeded(pc, node.output);

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
    retry_policy: RetryPolicy,
) -> RuntimeEngineResult<RuntimeSignal> {
    let mut store = ValueStore::new();
    let mut evidence = EvidenceCollector::new();
    drive_deterministic_full(
        plan,
        run,
        budget,
        &mut store,
        contracts,
        retry_policy,
        &mut evidence,
    )
}
