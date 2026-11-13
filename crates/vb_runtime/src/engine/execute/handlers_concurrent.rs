#![forbid(unsafe_code)]

//! Deterministic concurrent-group execution.
//!
//! Concurrent execution is an upper bound, not a throughput guarantee: lanes are
//! executed one at a time in ordinal order and their values are collected in
//! that same order, so results are deterministic for a given IR and input.

use vb_core::frame::RunFrame;
use vb_core::ids::{BranchCount, FanoutLimit, ListId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint, join_taint};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow};

use crate::engine::execute::NodeDispatchContext;
use crate::engine::execute::execute_node_deferred;
use crate::engine::types::{RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};

/// Static per-lane step bound: every node may run once per item/attempt scale.
fn lane_bound(plan: &CompiledWorkflow, work: usize) -> usize {
    usize::from(plan.node_count())
        .saturating_mul(work.saturating_add(1))
        .saturating_mul(8)
        .saturating_add(1)
}

fn invariant(reason: &'static str) -> RuntimeEngineError {
    RuntimeEngineError::Core(vb_core::errors::EngineError::InternalInvariantViolation {
        reason: reason.into(),
    })
}

/// Cross-checks the lanes a group is about to run against the cached checked
/// requirements and the declared fanout policy.
fn check_lane_bound(plan: &CompiledWorkflow, capacity: usize) -> RuntimeEngineResult<()> {
    let requirements = plan
        .concurrent_requirements()
        .ok_or_else(|| invariant("concurrent plan missing checked requirements"))?;
    let frames = usize::from(requirements.frames());
    if capacity.saturating_add(1) > frames {
        return Err(invariant("concurrent lane capacity exceeds checked frame bound"));
    }
    if requirements.groups() == 0 {
        return Err(invariant("concurrent requirements report zero groups"));
    }
    let fanout = usize::from(plan.resource_contract().max_fanout);
    if capacity > fanout {
        return Err(invariant("concurrent lane capacity exceeds contract fanout"));
    }
    Ok(())
}

fn group_output(plan: &CompiledWorkflow, join: StepIdx) -> RuntimeEngineResult<SlotIdx> {
    plan.node(join)
        .and_then(|node| node.output.slot())
        .ok_or_else(|| invariant("concurrent join output slot"))
}

fn jump(run: &mut RunFrame, target: StepIdx) -> RuntimeEngineResult<()> {
    run.set_pc(target).map_err(RuntimeEngineError::Core)
}

/// Executes one lane body until its owning lane end and returns the lane value.
fn run_lane(
    ctx: &mut NodeDispatchContext<'_>,
    owner: StepIdx,
    entry: StepIdx,
    bound: usize,
) -> RuntimeEngineResult<Option<(SlotValue, Taint)>> {
    let plan = ctx.plan;
    jump(ctx.run, entry)?;
    for _ in 0..bound {
        let pc = ctx.run.pc();
        let node = plan
            .node(pc)
            .ok_or_else(|| invariant("concurrent lane pc out of bounds"))?;
        if let CompiledNodeKind::ConcurrentLaneEnd { owner: end_owner, result } = node.kind {
            if end_owner != owner {
                return Err(invariant("concurrent lane end owner mismatch"));
            }
            let Some(slot) = result else {
                return Ok(None);
            };
            let value = *ctx
                .run
                .read_slot(slot)
                .map_err(RuntimeEngineError::Core)?;
            let taint = ctx
                .run
                .read_taint(slot)
                .map_err(RuntimeEngineError::Core)?;
            return Ok(Some((value, taint)));
        }
        let node = plan
            .node(pc)
            .ok_or_else(|| invariant("concurrent lane node"))?;
        if ctx.run.step_state(pc).map_err(RuntimeEngineError::Core)?
            == vb_core::StepState::Succeeded
        {
            ctx.run.mark_pending(pc).map_err(RuntimeEngineError::Core)?;
        }
        let signal = execute_node_deferred(NodeDispatchContext {
            plan,
            run: &mut *ctx.run,
            store: &mut *ctx.store,
            node,
            contracts: ctx.contracts,
            retry_policy: ctx.retry_policy,
            collect_states: &mut *ctx.collect_states,
            granted: ctx.granted,
            state_run_attempt: ctx.state_run_attempt,
        })?;
        if !matches!(signal, RuntimeSignal::Continue) {
            return Err(invariant("concurrent lane suspended or finished"));
        }
    }
    Err(invariant("concurrent lane step bound exceeded"))
}

fn item_value(
    store: &ValueStore,
    list_id: ListId,
    index: usize,
) -> RuntimeEngineResult<(SlotValue, Taint)> {
    let source = store.list(list_id).map_err(RuntimeEngineError::Core)?;
    let value = source
        .get(index)
        .copied()
        .ok_or_else(|| invariant("concurrent item index"))?;
    let ordinal = u32::try_from(index).map_err(|_| invariant("concurrent item ordinal"))?;
    let taint = store
        .list_item_with_taint(list_id, ordinal)
        .map_err(RuntimeEngineError::Core)?
        .1;
    Ok((value, taint))
}

fn foreach_group(
    ctx: &mut NodeDispatchContext<'_>,
    input: SlotIdx,
    item_slot: SlotIdx,
    limit: FanoutLimit,
    at_once: BranchCount,
    body: StepIdx,
    join: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    let list_id = match *ctx.run.read_slot(input).map_err(RuntimeEngineError::Core)? {
        SlotValue::List(id) => id,
        _ => {
            return Err(invariant("concurrent for_each input must be a list"));
        }
    };
    let count = ctx.store.list(list_id).map_err(RuntimeEngineError::Core)?.len();
    if count > limit.as_usize() {
        return Err(RuntimeEngineError::Core(
            vb_core::errors::EngineError::IterationLimitExceeded {
                resource: "for_each_limit",
            },
        ));
    }
    let capacity = usize::from(at_once.get()).min(count).min(limit.as_usize());
    check_lane_bound(ctx.plan, capacity)?;
    let bound = lane_bound(ctx.plan, count);
    let owner = ctx.node.id;
    let mut results = Vec::new();
    results
        .try_reserve_exact(count)
        .map_err(|_| RuntimeEngineError::Core(vb_core::errors::EngineError::AllocationFailed))?;
    let mut taint = ctx.run.read_taint(input).map_err(RuntimeEngineError::Core)?;
    for index in 0..count {
        let (value, item_taint) = item_value(ctx.store, list_id, index)?;
        ctx.run
            .write_slot_with_taint(item_slot, value, item_taint)
            .map_err(RuntimeEngineError::Core)?;
        let lane = run_lane(ctx, owner, body, bound)?;
        match lane {
            Some((value, lane_taint)) => {
                taint = join_taint(taint, lane_taint);
                results.push(value);
            }
            None => results.push(SlotValue::Null),
        }
    }
    finish_group(ctx, join, results, taint)
}

fn fixed_group(
    ctx: &mut NodeDispatchContext<'_>,
    branches: &[StepIdx],
    join: StepIdx,
    first_success: bool,
) -> RuntimeEngineResult<RuntimeSignal> {
    let owner = ctx.node.id;
    check_lane_bound(ctx.plan, branches.len())?;
    let bound = lane_bound(ctx.plan, branches.len());
    let mut taint = Taint::Clean;
    let mut results = Vec::new();
    results
        .try_reserve_exact(branches.len())
        .map_err(|_| RuntimeEngineError::Core(vb_core::errors::EngineError::AllocationFailed))?;
    for entry in branches {
        match run_lane(ctx, owner, *entry, bound) {
            Ok(lane) => {
                if first_success {
                    let (value, lane_taint) = lane.unwrap_or((SlotValue::Null, Taint::Clean));
                    return finish_group_value(ctx, join, value, lane_taint);
                }
                match lane {
                    Some((value, lane_taint)) => {
                        taint = join_taint(taint, lane_taint);
                        results.push(value);
                    }
                    None => results.push(SlotValue::Null),
                }
            }
            Err(error) if first_success => {
                drop(error);
            }
            Err(error) => return Err(error),
        }
    }
    if first_success {
        return Err(invariant("first success group had no successful lane"));
    }
    finish_group(ctx, join, results, taint)
}

fn finish_group(
    ctx: &mut NodeDispatchContext<'_>,
    join: StepIdx,
    results: Vec<SlotValue>,
    taint: Taint,
) -> RuntimeEngineResult<RuntimeSignal> {
    let list = ctx
        .store
        .insert_list(results)
        .map_err(RuntimeEngineError::Core)?;
    finish_group_value(ctx, join, SlotValue::List(list), taint)
}

fn finish_group_value(
    ctx: &mut NodeDispatchContext<'_>,
    join: StepIdx,
    value: SlotValue,
    taint: Taint,
) -> RuntimeEngineResult<RuntimeSignal> {
    let output = group_output(ctx.plan, join)?;
    ctx.run
        .write_slot_with_taint(output, value, taint)
        .map_err(RuntimeEngineError::Core)?;
    jump(ctx.run, join)?;
    Ok(RuntimeSignal::Continue)
}

fn lane_end(
    ctx: &mut NodeDispatchContext<'_>,
    owner: StepIdx,
) -> RuntimeEngineResult<RuntimeSignal> {
    let start = ctx
        .plan
        .node(owner)
        .ok_or_else(|| invariant("lane end owner missing"))?;
    let join = match start.kind {
        CompiledNodeKind::ForEachConcurrentStart { join, .. }
        | CompiledNodeKind::TogetherConcurrentStart { join, .. }
        | CompiledNodeKind::FirstSuccessStart { join, .. } => join,
        _ => return Err(invariant("lane end owner is not a concurrent start")),
    };
    jump(ctx.run, join)?;
    Ok(RuntimeSignal::Continue)
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
        } => foreach_group(ctx, *input, *item_slot, *limit, *at_once, *body, *join),
        CompiledNodeKind::TogetherConcurrentStart { branches, join } => {
            fixed_group(ctx, branches, *join, false)
        }
        CompiledNodeKind::FirstSuccessStart { branches, join } => {
            fixed_group(ctx, branches, *join, true)
        }
        CompiledNodeKind::ConcurrentLaneEnd { owner, .. } => lane_end(ctx, *owner),
        CompiledNodeKind::ConcurrentGroupJoin { .. } => group_join(ctx),
        _ => Err(invariant("non-concurrent node routed to concurrent dispatch")),
    }
}
