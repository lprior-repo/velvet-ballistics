#![forbid(unsafe_code)]

//! Deterministic concurrent-group execution.
//!
//! Concurrent execution is an upper bound, not a throughput guarantee: lanes run
//! one at a time in ordinal order and values are collected in that order.
//! Every lane node goes through `begin_drive_step`/`finish_drive_step`, so it
//! consumes drive-step budget and emits the same evidence as the main loop.

use vb_core::errors::EngineError;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint, join_taint};
use vb_core::workflow::{CompiledNode, CompiledNodeKind};

use crate::engine::drive::transitions::{DriveStep, begin_drive_step, finish_drive_step};
use crate::engine::execute::NodeDispatchContext;
use crate::engine::execute::concurrent_support::{
    check_lane_bound, group_output, invariant, jump, lane_bound,
};
use crate::engine::execute::execute_node_deferred;
#[path = "handlers_concurrent/fixed.rs"]
mod fixed;
use crate::engine::types::{RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};
use fixed::{append_group_value, finish_group_value, fixed_group};
#[path = "handlers_concurrent/lanes.rs"]
mod lanes;
pub(super) use lanes::lane_end;
use lanes::run_lane;

enum LaneStep {
    Continue,
    End(StepIdx, Option<SlotIdx>),
    Exhausted,
}

enum LaneOutcome {
    Exhausted,
    Value(Option<(SlotValue, Taint)>),
}

fn lane_begin<'a>(ctx: &mut NodeDispatchContext<'a>) -> RuntimeEngineResult<Option<DriveStep<'a>>> {
    let plan = ctx.plan;
    let Some(budget) = ctx.budget.as_deref_mut() else {
        return Err(invariant("concurrent lane requires the drive-step budget"));
    };
    let Some(evidence) = ctx.evidence.as_deref_mut() else {
        return Err(invariant("concurrent lane requires step evidence"));
    };
    let pc = ctx.run.pc();
    if ctx.run.step_state(pc).map_err(RuntimeEngineError::Core)? == vb_core::StepState::Succeeded {
        ctx.run.mark_pending(pc).map_err(RuntimeEngineError::Core)?;
    }
    begin_drive_step(plan, &mut *ctx.run, budget, evidence)
}

fn lane_execute(
    ctx: &mut NodeDispatchContext<'_>,
    node: &CompiledNode,
) -> RuntimeEngineResult<RuntimeSignal> {
    execute_node_deferred(NodeDispatchContext {
        plan: ctx.plan,
        run: &mut *ctx.run,
        store: &mut *ctx.store,
        node,
        contracts: ctx.contracts,
        retry_policy: ctx.retry_policy,
        collect_states: &mut *ctx.collect_states,
        granted: ctx.granted,
        state_run_attempt: ctx.state_run_attempt,
        budget: ctx.budget.as_deref_mut(),
        evidence: ctx.evidence.as_deref_mut(),
        drive_state: ctx.drive_state.as_deref_mut(),
    })
}

fn lane_step(ctx: &mut NodeDispatchContext<'_>) -> RuntimeEngineResult<LaneStep> {
    let Some(step) = lane_begin(ctx)? else {
        return Ok(LaneStep::Exhausted);
    };
    let signal = match lane_execute(ctx, step.node) {
        Ok(signal) => signal,
        Err(error) => {
            ctx.run
                .mark_failed(step.pc)
                .map_err(RuntimeEngineError::Core)?;
            ctx.evidence
                .as_deref_mut()
                .ok_or_else(|| invariant("concurrent lane evidence missing"))?
                .push_step_failed(step.pc)
                .map_err(RuntimeEngineError::Core)?;
            return Err(error);
        }
    };
    lanes::lane_finish(ctx, step, &signal)
}

fn complete_resumed_first_success(
    ctx: &mut NodeDispatchContext<'_>,
    owner: StepIdx,
) -> RuntimeEngineResult<()> {
    if ctx
        .run
        .step_state(owner)
        .map_err(RuntimeEngineError::Core)?
        != vb_core::StepState::Waiting
    {
        return Ok(());
    }
    let start = ctx
        .plan
        .node(owner)
        .ok_or_else(|| invariant("first-success owner missing"))?;
    ctx.run
        .mark_running(owner)
        .map_err(RuntimeEngineError::Core)?;
    ctx.run
        .mark_succeeded(owner)
        .map_err(RuntimeEngineError::Core)?;
    ctx.evidence
        .as_deref_mut()
        .ok_or_else(|| invariant("first-success evidence missing"))?
        .push_step_succeeded(owner, start.output.slot())
        .map_err(RuntimeEngineError::Core)
}

fn group_join(ctx: &mut NodeDispatchContext<'_>) -> RuntimeEngineResult<RuntimeSignal> {
    let next = ctx
        .node
        .routing
        .next
        .step()
        .ok_or_else(|| invariant("concurrent join has no continuation"))?;
    jump(ctx.run, next)?;
    Ok(RuntimeSignal::Continue)
}

pub(super) fn dispatch_concurrent(
    ctx: &mut NodeDispatchContext<'_>,
) -> RuntimeEngineResult<RuntimeSignal> {
    match &ctx.node.kind {
        CompiledNodeKind::ForEachConcurrentStart { .. } => {
            super::handlers_concurrent_foreach::start(ctx)
        }
        CompiledNodeKind::TogetherConcurrentStart { branches, join } => {
            fixed_group(ctx, branches, *join, false)
        }
        CompiledNodeKind::FirstSuccessStart { branches, join } => {
            fixed_group(ctx, branches, *join, true)
        }
        CompiledNodeKind::ConcurrentLaneEnd { owner, result } => lane_end(ctx, *owner, *result),
        CompiledNodeKind::ConcurrentGroupJoin { .. } => group_join(ctx),
        _ => Err(invariant(
            "non-concurrent node routed to concurrent dispatch",
        )),
    }
}

pub(crate) use super::concurrent_support::ordinary_lane_failure;
