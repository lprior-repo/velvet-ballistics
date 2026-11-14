use crate::frame::RunFrame;
use crate::ids::{BranchCount, FanoutLimit, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint, join_taint};
use crate::value_store::ValueStore;
use crate::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

use super::super::{ReplayError, engine_to_replay_err};
use super::concurrent_validation::check_for_each_capacity;
use super::dispatch::replay_step_with_collect;
use super::types::{ReplayAction, ReplayCollectStates};

fn first_success_can_continue(node: Option<&CompiledNode>, error: &ReplayError) -> bool {
    matches!((node, error),
        (Some(node), ReplayError::ExpressionEvalFailed { step })
            if node.id == *step && matches!(node.kind, CompiledNodeKind::EvalExpr { .. }))
}

fn invariant(reason: &'static str) -> ReplayError {
    ReplayError::Internal { reason }
}

fn lane_bound(plan: &CompiledWorkflow, work: usize) -> usize {
    usize::from(plan.node_count())
        .saturating_mul(work.saturating_add(1))
        .saturating_mul(8)
        .saturating_add(1)
}

fn join_output(plan: &CompiledWorkflow, join: StepIdx) -> Result<SlotIdx, ReplayError> {
    plan.node(join)
        .and_then(|node| node.output.slot())
        .ok_or_else(|| invariant("concurrent join output slot"))
}

fn jump(run: &mut RunFrame, target: StepIdx) -> Result<ReplayAction, ReplayError> {
    run.set_pc(target).map_err(engine_to_replay_err)?;
    Ok(ReplayAction::Continue(target))
}

/// Replays one lane body through the same step dispatcher until its lane end.
fn run_lane(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    collect_states: &mut ReplayCollectStates,
    owner: StepIdx,
    entry: StepIdx,
    bound: usize,
) -> Result<Option<(SlotValue, Taint)>, ReplayError> {
    run.set_pc(entry).map_err(engine_to_replay_err)?;
    let mut pc = entry;
    for _ in 0..bound {
        let node = plan
            .node(pc)
            .ok_or_else(|| invariant("concurrent lane pc out of bounds"))?;
        if let CompiledNodeKind::ConcurrentLaneEnd {
            owner: end_owner,
            result,
        } = node.kind
        {
            if end_owner != owner {
                return Err(invariant("concurrent lane end owner mismatch"));
            }
            let Some(slot) = result else {
                return Ok(None);
            };
            let value = *run
                .read_slot(slot)
                .map_err(|_| invariant("concurrent lane result slot"))?;
            let taint = run
                .read_taint(slot)
                .map_err(|_| invariant("concurrent lane result taint"))?;
            return Ok(Some((value, taint)));
        }
        match replay_step_with_collect(node, run, store, plan, collect_states)? {
            ReplayAction::Continue(next) => pc = next,
            _ => return Err(invariant("concurrent lane suspended or finished")),
        }
    }
    Err(invariant("concurrent lane step bound exceeded"))
}

fn list_id(run: &RunFrame, slot: SlotIdx) -> Result<crate::ids::ListId, ReplayError> {
    match *run
        .read_slot(slot)
        .map_err(|_| invariant("concurrent input slot"))?
    {
        SlotValue::List(id) => Ok(id),
        _ => Err(invariant("concurrent for_each input must be a list")),
    }
}

fn finish_results(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    join: StepIdx,
    results: Vec<(SlotValue, Taint)>,
    taint: Taint,
) -> Result<ReplayAction, ReplayError> {
    let output = join_output(plan, join)?;
    let (values, taints): (Vec<_>, Vec<_>) = results.into_iter().unzip();
    let list = store
        .insert_list_with_taint(values.into_boxed_slice(), taints.into_boxed_slice())
        .map_err(|_| invariant("concurrent result list"))?;
    run.write_slot_with_taint(output, SlotValue::List(list), taint)
        .map_err(|_| invariant("concurrent result write"))?;
    jump(run, join)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn replay_for_each_concurrent_start(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    collect_states: &mut ReplayCollectStates,
    node: &CompiledNode,
    input: SlotIdx,
    item_slot: SlotIdx,
    limit: FanoutLimit,
    at_once: BranchCount,
    body: StepIdx,
    join: StepIdx,
) -> Result<ReplayAction, ReplayError> {
    let id = list_id(run, input)?;
    let count = store
        .list(id)
        .map_err(|_| invariant("concurrent input list"))?
        .len();
    if count > limit.as_usize() {
        return Err(ReplayError::IterationLimitExceeded {
            resource: "for_each_limit",
        });
    }
    check_for_each_capacity(plan, node.id, join, count, limit, at_once)?;
    let bound = lane_bound(plan, count);
    let mut results = Vec::new();
    results
        .try_reserve_exact(count)
        .map_err(|_| invariant("concurrent result capacity"))?;
    let mut taint = run
        .read_taint(input)
        .map_err(|_| invariant("concurrent input taint"))?;
    let input_taint = taint;
    for index in 0..count {
        let source = store
            .list(id)
            .map_err(|_| invariant("concurrent input list"))?;
        let value = source
            .get(index)
            .copied()
            .ok_or_else(|| invariant("concurrent item index"))?;
        let ordinal = u32::try_from(index).map_err(|_| invariant("concurrent ordinal"))?;
        let item_taint = store
            .list_item_with_taint(id, ordinal)
            .map_err(|_| invariant("concurrent item taint"))?
            .1;
        run.write_slot_with_taint(item_slot, value, join_taint(input_taint, item_taint))
            .map_err(|_| invariant("concurrent item write"))?;
        match run_lane(plan, run, store, collect_states, node.id, body, bound)? {
            Some((value, lane_taint)) => {
                taint = join_taint(taint, lane_taint);
                results.push((value, lane_taint));
            }
            None => results.push((SlotValue::Null, Taint::Clean)),
        }
    }
    finish_results(plan, run, store, join, results, taint)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn replay_fixed_concurrent_start(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    collect_states: &mut ReplayCollectStates,
    node: &CompiledNode,
    branches: &[StepIdx],
    join: StepIdx,
    first_success: bool,
) -> Result<ReplayAction, ReplayError> {
    let bound = lane_bound(plan, branches.len());
    let mut taint = Taint::Clean;
    let mut results = Vec::new();
    results
        .try_reserve_exact(branches.len())
        .map_err(|_| invariant("concurrent result capacity"))?;
    for entry in branches {
        match run_lane(plan, run, store, collect_states, node.id, *entry, bound) {
            Ok(lane) => {
                if first_success {
                    let (value, lane_taint) = lane.unwrap_or((SlotValue::Null, Taint::Clean));
                    return finish_value(plan, run, join, value, lane_taint);
                }
                match lane {
                    Some((value, lane_taint)) => {
                        taint = join_taint(taint, lane_taint);
                        results.push((value, lane_taint));
                    }
                    None => results.push((SlotValue::Null, Taint::Clean)),
                }
            }
            Err(error)
                if first_success && first_success_can_continue(plan.node(run.pc()), &error) => {}
            Err(error) => return Err(error),
        }
    }
    if first_success {
        return Err(invariant("first success group had no successful lane"));
    }
    finish_results(plan, run, store, join, results, taint)
}

fn finish_value(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    join: StepIdx,
    value: SlotValue,
    taint: Taint,
) -> Result<ReplayAction, ReplayError> {
    let output = join_output(plan, join)?;
    run.write_slot_with_taint(output, value, taint)
        .map_err(|_| invariant("concurrent result write"))?;
    jump(run, join)
}

pub(super) fn replay_concurrent_lane_end(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    owner: StepIdx,
) -> Result<ReplayAction, ReplayError> {
    let start = plan
        .node(owner)
        .ok_or_else(|| invariant("concurrent lane owner missing"))?;
    let join = match start.kind {
        CompiledNodeKind::ForEachConcurrentStart { join, .. }
        | CompiledNodeKind::TogetherConcurrentStart { join, .. }
        | CompiledNodeKind::FirstSuccessStart { join, .. } => join,
        _ => return Err(invariant("lane end owner is not a concurrent start")),
    };
    jump(run, join)
}

pub(super) fn replay_concurrent_group_join(
    run: &mut RunFrame,
    node: &CompiledNode,
) -> Result<ReplayAction, ReplayError> {
    let next = node
        .routing
        .next
        .step()
        .ok_or_else(|| invariant("concurrent join has no continuation"))?;
    jump(run, next)
}
#[cfg(test)]
#[path = "concurrent/tests.rs"]
mod jump_tests;

#[cfg(test)]
mod tests {
    use super::first_success_can_continue;
    use crate::ids::StepIdx;
    use crate::replay::ReplayError;

    #[test]
    fn first_success_continues_only_after_expression_failures() {
        let node = super::jump_tests::eval_node();
        assert!(first_success_can_continue(
            Some(&node),
            &ReplayError::ExpressionEvalFailed {
                step: StepIdx::new(3),
            }
        ));
        assert!(!first_success_can_continue(
            Some(&node),
            &ReplayError::Internal {
                reason: "allocation",
            }
        ));
        assert!(!first_success_can_continue(
            Some(&node),
            &ReplayError::SlotNotAvailable {
                slot: crate::ids::SlotIdx::new(1),
            }
        ));
    }
}
