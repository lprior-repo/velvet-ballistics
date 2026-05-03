#![forbid(unsafe_code)]

//! Deterministic drive loop for runtime engine.

use vb_core::action::ActionContract;
use vb_core::engine::{EngineError, StepBudget};
use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow};

use crate::engine::execute::execute_node_full;
use crate::engine::helpers::mark_step_after_signal;
use crate::engine::types::{
    EvidenceCollector, RetryPolicy, RuntimeEngineError, RuntimeEngineResult, RuntimeSignal,
};
use crate::primitives::collect::CollectStates;

fn compute_max_parallel_in_flight(plan: &CompiledWorkflow) -> u16 {
    let mut max_branches: u16 = 0;
    for i in 0..plan.node_count() {
        let step = StepIdx::new(i);
        if let Some(node) = plan.node(step)
            && let CompiledNodeKind::TogetherStart { branches, .. } = &node.kind
        {
            let branch_count = u16::try_from(branches.len()).unwrap_or(u16::MAX);
            if branch_count > max_branches {
                max_branches = branch_count;
            }
        }
    }
    max_branches
}

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
) -> RuntimeEngineResult<RuntimeSignal> {
    let max_parallel = compute_max_parallel_in_flight(plan);
    run.set_max_parallel_in_flight(max_parallel);

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

        let signal = execute_node_full(
            plan,
            run,
            store,
            node,
            contracts,
            retry_policy,
            collect_states,
        )?;

        match mark_step_after_signal(run, pc, &signal) {
            Ok(()) => {}
            Err(e) => return Err(RuntimeEngineError::Core(e)),
        }

        // Evidence chain: emit SlotWritten with actual value for all slot writes,
        // including internal expression evaluations (SetConst, Copy, EvalExpr,
        // BuildObject, BuildList). This satisfies Phase 40/44 requirement.
        if let Some(slot) = node.output {
            if let Ok(value) = run.read_slot(slot) {
                evidence.push_slot_written(slot, value);
            }
        }

        // Evidence chain: emit StepSucceeded after execution with output slot.
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
    )
}
