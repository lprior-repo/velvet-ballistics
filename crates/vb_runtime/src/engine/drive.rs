#![forbid(unsafe_code)]

//! Deterministic drive loop for runtime engine.

use vb_core::action::ActionContract;
use vb_core::engine::{EngineError, StepBudget};
use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

use crate::engine::EvidenceCollector;
use crate::engine::execute::execute_node_full;
use crate::engine::helpers::mark_step_after_signal;
use crate::engine::types::{RetryPolicy, RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};
use crate::primitives::collect::CollectStates;

pub(crate) fn compute_max_parallel_in_flight(plan: &CompiledWorkflow) -> RuntimeEngineResult<u16> {
    let mut max_branches: u16 = 0;
    for i in 0..plan.node_count() {
        let step = StepIdx::new(i);
        if let Some(node) = plan.node(step)
            && let CompiledNodeKind::TogetherStart { branches, .. } = &node.kind
        {
            let branch_count = u16::try_from(branches.len()).map_err(|_| {
                RuntimeEngineError::BranchLimitExceeded {
                    max: u16::MAX.into(),
                    requested: branches.len(),
                }
            })?;
            if branch_count > max_branches {
                max_branches = branch_count;
            }
        }
    }
    Ok(max_branches)
}

/// Enhanced drive loop that handles all node kinds including
/// iteration, compound, action, and suspension primitives.
///
/// Collects evidence events (StepStarted/StepSucceeded) for every step
/// executed during the drive loop. The caller drains these events to emit
/// them to the journal and trace ring.
///
/// # Errors (RE-013)
///
/// Returns [`RuntimeEngineError::RetryZeroMaxAttempts`] when the supplied
/// `retry_policy.max_attempts` is zero. Issuing an action ticket with
/// `attempt = 1` and `capacity = 0` would violate the ticket invariant
/// `attempt <= capacity`, so the runtime refuses to dispatch the run.
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
    if !retry_policy.is_valid_for_dispatch() {
        return Err(RuntimeEngineError::RetryZeroMaxAttempts);
    }
    initialize_drive(run, plan)?;

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
        finish_drive_step(run, evidence, collect_states, step, &signal)?;
        match signal {
            RuntimeSignal::Continue => {}
            other => return Ok(other),
        }
    }
}

struct DriveStep<'a> {
    pc: StepIdx,
    node: &'a CompiledNode,
}

fn initialize_drive(run: &mut RunFrame, plan: &CompiledWorkflow) -> RuntimeEngineResult<()> {
    let max_parallel = compute_max_parallel_in_flight(plan)?;
    run.set_max_parallel_in_flight(max_parallel);
    Ok(())
}

fn begin_drive_step<'a>(
    plan: &'a CompiledWorkflow,
    run: &mut RunFrame,
    budget: &mut StepBudget,
    evidence: &mut EvidenceCollector,
) -> RuntimeEngineResult<Option<DriveStep<'a>>> {
    if !budget.try_take().map_err(RuntimeEngineError::Core)? {
        return Ok(None);
    }
    let pc = run.pc();
    let node = plan
        .node(pc)
        .ok_or(EngineError::InvalidProgramCounter { step: pc })?;
    evidence.push_step_started(pc);
    run.mark_running(pc).map_err(RuntimeEngineError::Core)?;
    Ok(Some(DriveStep { pc, node }))
}

fn finish_drive_step(
    run: &mut RunFrame,
    evidence: &mut EvidenceCollector,
    collect_states: &CollectStates,
    step: DriveStep<'_>,
    signal: &RuntimeSignal,
) -> RuntimeEngineResult<()> {
    mark_step_after_signal(run, step.pc, signal).map_err(RuntimeEngineError::Core)?;
    emit_slot_evidence(run, evidence, collect_states, step.node)?;
    if signal_is_success(signal) {
        // RS-004: the deterministic drive loop emits `attempt: 1` because
        // engine-level retries do not exist. The shard's flush step
        // overrides this with `state.action_attempts[step]` so the durable
        // journal record carries the actual live attempt counter.
        evidence.push_step_succeeded(step.pc, step.node.output, 1);
    }
    Ok(())
}

fn signal_is_success(signal: &RuntimeSignal) -> bool {
    matches!(signal, RuntimeSignal::Continue | RuntimeSignal::Finished(_))
}

fn emit_slot_evidence(
    run: &RunFrame,
    evidence: &mut EvidenceCollector,
    collect_states: &CollectStates,
    node: &CompiledNode,
) -> RuntimeEngineResult<()> {
    if let Some(slot) = collect_written_slot(node)
        && let Ok(value) = run.read_slot(slot)
    {
        let extra = collect_states.capture_state(run.run_id(), slot);
        let taint = run.read_taint(slot).map_err(RuntimeEngineError::Core)?;
        evidence
            .push_slot_written_with_extra(slot, *value, taint, extra)
            .map_err(RuntimeEngineError::Core)?;
    } else if let Some(slot) = node.output
        && let Ok(value) = run.read_slot(slot)
    {
        let taint = run.read_taint(slot).map_err(RuntimeEngineError::Core)?;
        evidence.push_slot_written_with_taint(slot, *value, taint);
    }
    Ok(())
}

fn collect_written_slot(node: &CompiledNode) -> Option<SlotIdx> {
    match &node.kind {
        CompiledNodeKind::CollectStart { source, .. } => match node.output {
            Some(output) => Some(output),
            None => Some(*source),
        },
        CompiledNodeKind::CollectNext { collector_slot, .. }
        | CompiledNodeKind::CollectFinish { collector_slot } => Some(*collector_slot),
        _ => None,
    }
}

/// Backward-compatible drive loop matching the original drive_with_actions signature.
///
/// # Errors (RE-013)
///
/// Returns [`RuntimeEngineError::RetryZeroMaxAttempts`] when the supplied
/// `retry_policy.max_attempts` is zero.
pub fn drive_with_actions(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    budget: &mut StepBudget,
    contracts: &[ActionContract],
    retry_policy: RetryPolicy,
) -> RuntimeEngineResult<RuntimeSignal> {
    if !retry_policy.is_valid_for_dispatch() {
        return Err(RuntimeEngineError::RetryZeroMaxAttempts);
    }
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
