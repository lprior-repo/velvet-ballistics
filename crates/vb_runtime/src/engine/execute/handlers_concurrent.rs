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

fn lane_finish(
    ctx: &mut NodeDispatchContext<'_>,
    step: DriveStep<'_>,
    signal: &RuntimeSignal,
) -> RuntimeEngineResult<LaneStep> {
    let Some(evidence) = ctx.evidence.as_deref_mut() else {
        return Err(invariant("concurrent lane evidence missing"));
    };
    let Some(drive_state) = ctx.drive_state.as_deref_mut() else {
        return Err(invariant("concurrent lane drive state missing"));
    };
    let lane_step = match signal {
        RuntimeSignal::Continue => match &step.node.kind {
            CompiledNodeKind::ConcurrentLaneEnd { owner, result } => LaneStep::End(*owner, *result),
            _ => LaneStep::Continue,
        },
        RuntimeSignal::StepBudgetExhausted => LaneStep::Exhausted,
        _ => return Err(invariant("concurrent lane must be a pure synchronous body")),
    };
    finish_drive_step(
        ctx.plan,
        &mut *ctx.run,
        evidence,
        &*ctx.collect_states,
        step,
        signal,
        drive_state,
    )?;
    Ok(lane_step)
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
    lane_finish(ctx, step, &signal)
}

fn run_lane(
    ctx: &mut NodeDispatchContext<'_>,
    owner: StepIdx,
    entry: StepIdx,
    bound: usize,
) -> RuntimeEngineResult<LaneOutcome> {
    let drive_state = ctx
        .drive_state
        .as_deref_mut()
        .ok_or_else(|| invariant("concurrent lane drive state missing"))?;
    let previous = drive_state.enter_concurrent_lane(owner);
    let result = (|| {
        jump(ctx.run, entry)?;
        for _ in 0..bound {
            match lane_step(ctx)? {
                LaneStep::Exhausted => return Ok(LaneOutcome::Exhausted),
                LaneStep::Continue => {}
                LaneStep::End(end_owner, result) => {
                    if end_owner != owner {
                        return Err(invariant("concurrent lane end owner mismatch"));
                    }
                    let Some(slot) = result else {
                        return Ok(LaneOutcome::Value(None));
                    };
                    let value = *ctx.run.read_slot(slot).map_err(RuntimeEngineError::Core)?;
                    let taint = ctx.run.read_taint(slot).map_err(RuntimeEngineError::Core)?;
                    return Ok(LaneOutcome::Value(Some((value, taint))));
                }
            }
        }
        Err(invariant("concurrent lane step bound exceeded"))
    })();
    let drive_state = ctx
        .drive_state
        .as_deref_mut()
        .ok_or_else(|| invariant("concurrent lane drive state missing"))?;
    drive_state.restore_concurrent_lane(previous);
    result
}

fn lane_end(
    ctx: &mut NodeDispatchContext<'_>,
    owner: StepIdx,
    result: Option<SlotIdx>,
) -> RuntimeEngineResult<RuntimeSignal> {
    let start = ctx
        .plan
        .node(owner)
        .ok_or_else(|| invariant("lane end owner missing"))?;
    if ctx
        .drive_state
        .as_deref()
        .and_then(|state| state.active_concurrent_lane())
        == Some(owner)
    {
        let join = match &start.kind {
            CompiledNodeKind::TogetherConcurrentStart { join, .. }
            | CompiledNodeKind::FirstSuccessStart { join, .. } => *join,
            _ => return Err(invariant("active fixed lane owner changed kind")),
        };
        jump(ctx.run, join)?;
        return Ok(RuntimeSignal::Continue);
    }
    let (join, first_success) = match &start.kind {
        CompiledNodeKind::ForEachConcurrentStart { .. } => {
            return super::handlers_concurrent_foreach::complete_lane(ctx, owner, result);
        }
        CompiledNodeKind::TogetherConcurrentStart { join, .. } => (*join, false),
        CompiledNodeKind::FirstSuccessStart { join, .. } => (*join, true),
        _ => return Err(invariant("lane end owner is not a concurrent start")),
    };
    let (value, taint) = match result {
        Some(slot) => (
            *ctx.run.read_slot(slot).map_err(RuntimeEngineError::Core)?,
            ctx.run.read_taint(slot).map_err(RuntimeEngineError::Core)?,
        ),
        None => (SlotValue::Null, Taint::Clean),
    };
    if first_success {
        finish_group_value(ctx, join, value, taint)?;
        complete_resumed_first_success(ctx, owner)?;
        return Ok(RuntimeSignal::Continue);
    }
    let output = group_output(ctx.plan, join)?;
    let current = match *ctx
        .run
        .read_slot(output)
        .map_err(RuntimeEngineError::Core)?
    {
        SlotValue::List(list) => list,
        _ => return Err(invariant("concurrent group cursor is not a list")),
    };
    append_group_value(ctx, output, current, value, taint)?;
    ctx.run
        .mark_waiting(owner)
        .map_err(RuntimeEngineError::Core)?;
    jump(ctx.run, owner)?;
    Ok(RuntimeSignal::Continue)
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
        CompiledNodeKind::ForEachConcurrentStart {
            input,
            item_slot,
            limit,
            at_once,
            body,
            join,
        } => super::handlers_concurrent_foreach::start(
            ctx, *input, *item_slot, *limit, *at_once, *body, *join,
        ),
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
