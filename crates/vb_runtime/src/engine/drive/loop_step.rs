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
use crate::engine::execute::{NodeDispatchContext, execute_node_deferred};
use crate::engine::types::{
    EvidenceCollector, RetryPolicy, RuntimeEngineError, RuntimeEngineResult, RuntimeSignal,
};
use crate::primitives::collect::CollectStates;

#[path = "loop_step/first_success_error.rs"]
mod first_success_error;
use first_success_error::{FirstSuccessErrorContext, route_resumed_first_success_error};

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
    state_run_attempt: u16,
) -> RuntimeEngineResult<RuntimeSignal> {
    drive_and_report_gaps(DriveLoopContext {
        plan,
        run,
        budget,
        store,
        contracts,
        retry_policy,
        evidence,
        collect_states,
        granted,
        drive_state: DriveState::new(),
        state_run_attempt,
    })
}

/// Inner drive loop. Returns the natural `RuntimeSignal` from the
/// drive; gap surfacing happens in [`drive_deterministic_full`].
struct DriveLoopContext<'a> {
    plan: &'a CompiledWorkflow,
    run: &'a mut RunFrame,
    budget: &'a mut StepBudget,
    store: &'a mut ValueStore,
    contracts: &'a [ActionContract],
    retry_policy: RetryPolicy,
    evidence: &'a mut EvidenceCollector,
    collect_states: &'a mut CollectStates,
    granted: &'a vb_core::capability::CapabilitySet,
    drive_state: DriveState,
    state_run_attempt: u16,
}

enum DriveStepResult {
    RecoveredFailure,
    Signal(RuntimeSignal),
}

/// Reports evidence gaps only after a natural terminal signal.
fn drive_and_report_gaps(mut drive: DriveLoopContext<'_>) -> RuntimeEngineResult<RuntimeSignal> {
    initialize_drive(drive.run, drive.plan)?;
    let signal = run_drive_loop(&mut drive)?;
    let report = drive.drive_state.take_evidence_gaps_report();
    if report.count > 0 && matches!(signal, RuntimeSignal::Finished(_)) {
        return Err(RuntimeEngineError::EvidenceGapsRecorded {
            count: report.count,
            last_slot: report.last_slot,
        });
    }
    Ok(signal)
}

fn run_drive_loop(drive: &mut DriveLoopContext<'_>) -> RuntimeEngineResult<RuntimeSignal> {
    loop {
        let Some(step) = begin_drive_step(drive.plan, drive.run, drive.budget, drive.evidence)?
        else {
            return Ok(RuntimeSignal::StepBudgetExhausted);
        };
        match drive_one_step(drive, step)? {
            DriveStepResult::RecoveredFailure => continue,
            DriveStepResult::Signal(RuntimeSignal::Continue) => {}
            DriveStepResult::Signal(signal) => return Ok(signal),
        }
    }
}

fn drive_one_step(
    drive: &mut DriveLoopContext<'_>,
    step: crate::engine::drive::transitions::DriveStep<'_>,
) -> RuntimeEngineResult<DriveStepResult> {
    let signal = dispatch_drive_step(drive, &step)?;
    let Some(signal) = signal else {
        return Ok(DriveStepResult::RecoveredFailure);
    };
    finish_drive_step(
        drive.plan,
        drive.run,
        drive.evidence,
        drive.collect_states,
        step,
        &signal,
        &mut drive.drive_state,
    )?;
    Ok(DriveStepResult::Signal(signal))
}

fn dispatch_drive_step(
    drive: &mut DriveLoopContext<'_>,
    step: &crate::engine::drive::transitions::DriveStep<'_>,
) -> RuntimeEngineResult<Option<RuntimeSignal>> {
    let ctx = NodeDispatchContext {
        plan: drive.plan,
        run: drive.run,
        store: drive.store,
        node: step.node,
        contracts: drive.contracts,
        retry_policy: drive.retry_policy,
        collect_states: drive.collect_states,
        granted: drive.granted,
        state_run_attempt: drive.state_run_attempt,
        budget: Some(drive.budget),
        evidence: Some(drive.evidence),
        drive_state: Some(&mut drive.drive_state),
    };
    match execute_node_deferred(ctx) {
        Ok(signal) => Ok(Some(signal)),
        Err(error) => route_drive_error(drive, step.pc, error),
    }
}

fn route_drive_error(
    drive: &mut DriveLoopContext<'_>,
    step: vb_core::ids::StepIdx,
    error: RuntimeEngineError,
) -> RuntimeEngineResult<Option<RuntimeSignal>> {
    let mut route = FirstSuccessErrorContext {
        plan: drive.plan,
        run: drive.run,
        store: drive.store,
        evidence: drive.evidence,
    };
    if route_resumed_first_success_error(&mut route, step, &error)? {
        Ok(None)
    } else {
        Err(error)
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
        1,
    )
}

fn initialize_drive(run: &mut RunFrame, plan: &CompiledWorkflow) -> RuntimeEngineResult<()> {
    let max_parallel = compute_max_parallel_in_flight(plan)?;
    run.set_max_parallel_in_flight(max_parallel);
    Ok(())
}
