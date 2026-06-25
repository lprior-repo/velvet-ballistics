#![forbid(unsafe_code)]

//! Main drive loop orchestration.

use vb_core::action::ActionContract;
use vb_core::engine::StepBudget;
use vb_core::frame::RunFrame;
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;

use crate::engine::drive::actions::compute_max_parallel_in_flight;
use crate::engine::drive::recovery::DriveState;
use crate::engine::drive::transitions::{begin_drive_step, finish_drive_step};
use crate::engine::execute::execute_node_full;
use crate::engine::types::{
    EvidenceCollector, RetryPolicy, RuntimeEngineResult, RuntimeSignal,
};
use crate::primitives::collect::CollectStates;

/// Enhanced drive loop that handles all node kinds including
/// iteration, compound, action, and suspension primitives.
///
/// Collects evidence events (StepStarted/StepSucceeded) for every step
/// executed during the drive loop. The caller drains these events to emit
/// them to the journal and trace ring.
#[allow(clippy::too_many_arguments)]
pub fn drive_deterministic_full(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    budget: &mut StepBudget,
    store: &mut ValueStore,
    contracts: &[ActionContract],
    retry_policy: RetryPolicy,
    evidence: &mut EvidenceCollector,
    collect_states: &mut CollectStates,
    granted: &vb_core::capability::CapabilitySet,
) -> RuntimeEngineResult<RuntimeSignal> {
    initialize_drive(run, plan)?;

    // RE-004: thread a drive-state object through the loop so that
    // emit_slot_evidence can record evidence gaps (read_slot errors)
    // instead of silently swallowing them. The counter is owned by
    // the loop body and never escapes the public drive_deterministic_full
    // signature.
    let mut drive_state = DriveState::new();

    loop {
        let Some(step) = begin_drive_step(plan, run, budget, evidence)? else {
            return Ok(RuntimeSignal::StepBudgetExhausted);
        };
        let signal = execute_node_full(
            plan,
            run,
            store,
            step.node,
            contracts,
            retry_policy,
            collect_states,
            granted,
        )?;
        finish_drive_step(
            run,
            evidence,
            collect_states,
            step,
            &signal,
            &mut drive_state,
        )?;
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
    let mut collect_states = CollectStates::new();
    drive_deterministic_full(
        plan,
        run,
        budget,
        &mut store,
        contracts,
        retry_policy,
        &mut evidence,
        &mut collect_states,
        &vb_core::capability::CapabilitySet::empty(),
    )
}

fn initialize_drive(run: &mut RunFrame, plan: &CompiledWorkflow) -> RuntimeEngineResult<()> {
    let max_parallel = compute_max_parallel_in_flight(plan)?;
    run.set_max_parallel_in_flight(max_parallel);
    Ok(())
}