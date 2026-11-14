use vb_core::frame::{RunFrame, StepState};
use vb_core::ids::StepIdx;
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow};

use crate::engine::execute::handlers_concurrent::ordinary_lane_failure;
use crate::engine::types::{EvidenceCollector, RuntimeEngineError, RuntimeEngineResult};

pub(super) struct FirstSuccessErrorContext<'a> {
    pub(super) plan: &'a CompiledWorkflow,
    pub(super) run: &'a mut RunFrame,
    pub(super) store: &'a mut ValueStore,
    pub(super) evidence: &'a mut EvidenceCollector,
}

struct FirstSuccessTarget {
    owner: StepIdx,
    output: vb_core::ids::SlotIdx,
}

pub(super) fn route_resumed_first_success_error(
    ctx: &mut FirstSuccessErrorContext<'_>,
    step: StepIdx,
    error: &RuntimeEngineError,
) -> RuntimeEngineResult<bool> {
    if !ordinary_lane_failure(ctx.plan, step, error) {
        return Ok(false);
    }
    let Some(target) = find_first_success_owner(ctx, step)? else {
        return Ok(false);
    };
    record_failed_lane(ctx, target, step)
}

fn find_first_success_owner(
    ctx: &FirstSuccessErrorContext<'_>,
    step: StepIdx,
) -> RuntimeEngineResult<Option<FirstSuccessTarget>> {
    let mut candidate = None;
    for raw in 0..ctx.plan.node_count() {
        let owner = StepIdx::new(raw);
        if let Some(target) = first_success_target(ctx, owner, step)?
            && candidate
                .as_ref()
                .is_none_or(|previous: &FirstSuccessTarget| previous.owner < owner)
        {
            candidate = Some(target);
        }
    }
    Ok(candidate)
}

fn first_success_target(
    ctx: &FirstSuccessErrorContext<'_>,
    owner: StepIdx,
    step: StepIdx,
) -> RuntimeEngineResult<Option<FirstSuccessTarget>> {
    let Some((branches, join)) = waiting_first_success(ctx, owner)? else {
        return Ok(None);
    };
    let Some((output, completed)) = group_progress(ctx, join)? else {
        return Ok(None);
    };
    Ok(active_branch(branches, join, completed, step)
        .then_some(FirstSuccessTarget { owner, output }))
}

fn waiting_first_success<'a>(
    ctx: &'a FirstSuccessErrorContext<'_>,
    owner: StepIdx,
) -> RuntimeEngineResult<Option<(&'a [StepIdx], StepIdx)>> {
    let Some(node) = ctx.plan.node(owner) else {
        return Ok(None);
    };
    let CompiledNodeKind::FirstSuccessStart { branches, join } = &node.kind else {
        return Ok(None);
    };
    if ctx
        .run
        .step_state(owner)
        .map_err(RuntimeEngineError::Core)?
        != StepState::Waiting
    {
        return Ok(None);
    }
    Ok(Some((branches, *join)))
}

fn group_progress(
    ctx: &FirstSuccessErrorContext<'_>,
    join: StepIdx,
) -> RuntimeEngineResult<Option<(vb_core::ids::SlotIdx, usize)>> {
    let Some(join_node) = ctx.plan.node(join) else {
        return Ok(None);
    };
    let Some(output) = join_node.output.slot() else {
        return Ok(None);
    };
    let completed = match ctx.run.read_slot(output) {
        Ok(SlotValue::List(list)) => ctx
            .store
            .list(*list)
            .map_err(RuntimeEngineError::Core)?
            .len(),
        _ => return Ok(None),
    };
    Ok(Some((output, completed)))
}

fn active_branch(branches: &[StepIdx], join: StepIdx, completed: usize, step: StepIdx) -> bool {
    branches.iter().enumerate().any(|(index, entry)| {
        let next = index
            .checked_add(1)
            .and_then(|next| branches.get(next))
            .copied()
            .unwrap_or(join);
        completed == index && *entry <= step && step < next
    })
}

fn record_failed_lane(
    ctx: &mut FirstSuccessErrorContext<'_>,
    target: FirstSuccessTarget,
    step: StepIdx,
) -> RuntimeEngineResult<bool> {
    let current = match ctx.run.read_slot(target.output) {
        Ok(SlotValue::List(list)) => *list,
        _ => return Ok(false),
    };
    let output_taint = ctx
        .run
        .read_taint(target.output)
        .map_err(RuntimeEngineError::Core)?;
    let list = append_failure_placeholder(ctx, current)?;
    ctx.run
        .mark_failed(step)
        .map_err(RuntimeEngineError::Core)?;
    ctx.evidence
        .push_step_failed(step)
        .map_err(RuntimeEngineError::Core)?;
    write_failure_placeholder(ctx, list, target.output, output_taint)?;
    ctx.run
        .set_pc(target.owner)
        .map_err(RuntimeEngineError::Core)?;
    Ok(true)
}

fn append_failure_placeholder(
    ctx: &mut FirstSuccessErrorContext<'_>,
    current: vb_core::ids::ListId,
) -> RuntimeEngineResult<vb_core::ids::ListId> {
    let (values, taints) = ctx
        .store
        .prepare_list_append(current, Some((SlotValue::Null, Taint::Clean)))
        .map_err(RuntimeEngineError::Core)?;
    ctx.store
        .insert_list_with_taint(values, taints)
        .map_err(RuntimeEngineError::Core)
}

fn write_failure_placeholder(
    ctx: &mut FirstSuccessErrorContext<'_>,
    list: vb_core::ids::ListId,
    output: vb_core::ids::SlotIdx,
    output_taint: Taint,
) -> RuntimeEngineResult<()> {
    ctx.run
        .write_slot_with_taint(output, SlotValue::List(list), output_taint)
        .map_err(RuntimeEngineError::Core)
}
