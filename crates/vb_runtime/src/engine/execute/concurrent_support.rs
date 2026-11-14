#![forbid(unsafe_code)]

//! Shared helpers for deterministic concurrent-group execution.

use vb_core::frame::RunFrame;
use vb_core::ids::{ListId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;

use crate::engine::types::{RuntimeEngineError, RuntimeEngineResult};

pub(super) fn invariant(reason: &'static str) -> RuntimeEngineError {
    RuntimeEngineError::Core(vb_core::errors::EngineError::InternalInvariantViolation {
        reason: reason.into(),
    })
}

/// Static per-lane step bound: every node may run once per item/attempt scale.
pub(super) fn lane_bound(plan: &CompiledWorkflow, work: usize) -> usize {
    usize::from(plan.node_count())
        .saturating_mul(work.saturating_add(1))
        .saturating_mul(8)
        .saturating_add(1)
}

/// Cross-checks the lanes a group is about to run against the cached checked
/// requirements and the declared fanout policy.
pub(super) fn check_lane_bound(
    plan: &CompiledWorkflow,
    owner: StepIdx,
    capacity: usize,
) -> RuntimeEngineResult<()> {
    let (join, maximum) = group_bounds(plan, owner)?;
    check_capacity(capacity, maximum)?;
    check_join_owner(plan, owner, join)?;
    check_frame_bound(plan, capacity)?;
    check_contract_fanout(plan, capacity)
}

fn group_bounds(plan: &CompiledWorkflow, owner: StepIdx) -> RuntimeEngineResult<(StepIdx, usize)> {
    let group = plan
        .node(owner)
        .ok_or_else(|| invariant("concurrent group owner is out of bounds"))?;
    match &group.kind {
        vb_core::CompiledNodeKind::ForEachConcurrentStart {
            at_once,
            limit,
            join,
            ..
        } => Ok((*join, usize::from(at_once.get()).min(limit.as_usize()))),
        vb_core::CompiledNodeKind::TogetherConcurrentStart { branches, join }
        | vb_core::CompiledNodeKind::FirstSuccessStart { branches, join } => {
            Ok((*join, branches.len()))
        }
        _ => Err(invariant("concurrent region owner is not a group start")),
    }
}

fn check_capacity(capacity: usize, maximum: usize) -> RuntimeEngineResult<()> {
    if capacity > maximum {
        return Err(invariant("concurrent lane capacity exceeds owning group"));
    }
    Ok(())
}

fn check_join_owner(
    plan: &CompiledWorkflow,
    owner: StepIdx,
    join: StepIdx,
) -> RuntimeEngineResult<()> {
    if !matches!(
        plan.node(join).map(|node| &node.kind),
        Some(vb_core::CompiledNodeKind::ConcurrentGroupJoin { owner: join_owner })
            if *join_owner == owner
    ) {
        return Err(invariant("concurrent group join owner mismatch"));
    }
    Ok(())
}

fn check_frame_bound(plan: &CompiledWorkflow, capacity: usize) -> RuntimeEngineResult<()> {
    let requirements = plan
        .concurrent_requirements()
        .ok_or_else(|| invariant("concurrent plan missing checked requirements"))?;
    let frames = usize::from(requirements.frames());
    if capacity.saturating_add(1) > frames {
        return Err(invariant(
            "concurrent lane capacity exceeds checked frame bound",
        ));
    }
    if requirements.groups() == 0 {
        return Err(invariant("concurrent requirements report zero groups"));
    }
    Ok(())
}

fn check_contract_fanout(plan: &CompiledWorkflow, capacity: usize) -> RuntimeEngineResult<()> {
    let fanout = usize::from(plan.resource_contract().max_fanout);
    if capacity > fanout {
        return Err(invariant(
            "concurrent lane capacity exceeds contract fanout",
        ));
    }
    Ok(())
}

pub(super) fn group_output(plan: &CompiledWorkflow, join: StepIdx) -> RuntimeEngineResult<SlotIdx> {
    plan.node(join)
        .and_then(|node| node.output.slot())
        .ok_or_else(|| invariant("concurrent join output slot"))
}

pub(super) fn jump(run: &mut RunFrame, target: StepIdx) -> RuntimeEngineResult<()> {
    run.set_pc(target).map_err(RuntimeEngineError::Core)
}

pub(super) fn item_value(
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

pub(crate) fn ordinary_lane_failure(
    plan: &vb_core::CompiledWorkflow,
    step: StepIdx,
    error: &RuntimeEngineError,
) -> bool {
    matches!(
        plan.node(step).map(|node| &node.kind),
        Some(vb_core::CompiledNodeKind::EvalExpr { .. })
    ) && matches!(error, RuntimeEngineError::Core(error) if vb_core::engine::is_ordinary_expression_failure(error))
}

#[cfg(test)]
#[path = "concurrent_support/ordinary_lane_failure_tests.rs"]
mod ordinary_lane_failure_tests;
