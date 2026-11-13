use super::{Boundary, legacy, node_at};
use crate::budget::BudgetTraversalError;
use crate::budget::successors::push_successor_targets;
use crate::ids::StepIdx;
use crate::workflow::{CompiledNode, CompiledNodeKind as K};
fn ordinary_targets(node: &CompiledNode) -> Result<Vec<StepIdx>, BudgetTraversalError> {
    let mut out = Vec::new();
    push_successor_targets(&node.kind, &mut out);
    if let Some(t) = node.routing.next.step() {
        out.push(t);
    }
    Ok(out)
}

fn should_queue_error(boundary: Boundary, handler: StepIdx) -> bool {
    if let Boundary::Loop { done, .. } = boundary {
        handler.as_usize() >= done.as_usize()
    } else {
        false
    }
}

fn normal_targets(
    nodes: &[CompiledNode],
    node: &CompiledNode,
    boundary: Boundary,
) -> Result<Vec<StepIdx>, BudgetTraversalError> {
    if legacy::valid_latch(nodes, node, boundary)? {
        return Ok(Vec::new());
    }
    match node.kind {
        K::ForEachConcurrentStart { join, .. }
        | K::TogetherConcurrentStart { join, .. }
        | K::FirstSuccessStart { join, .. }
        | K::TogetherStart { join, .. } => Ok(vec![join]),
        K::ForEachStart { body, done, .. }
        | K::CollectStart { body, done, .. }
        | K::ReduceStart { body, done, .. }
        | K::RepeatStart { body, done, .. } => {
            let mut out = vec![done];
            crate::budget::loop_region::push_body_error_exits(nodes, body, done, &mut out);
            Ok(out)
        }
        K::Choose { .. } | K::ChooseSlot { .. } => Ok(Vec::new()),
        K::ConcurrentLaneEnd { owner, .. } => lane_end(node, boundary, owner),
        K::Finish { .. } => terminal(nodes, node, boundary),
        _ => ordinary_checked(nodes, node, boundary),
    }
}

fn lane_end(
    node: &CompiledNode,
    boundary: Boundary,
    owner: StepIdx,
) -> Result<Vec<StepIdx>, BudgetTraversalError> {
    if let Boundary::Lane {
        owner: expected,
        join,
    } = boundary
        && owner == expected
        && node.routing.next.step().is_none_or(|next| next == join)
    {
        return Ok(Vec::new());
    }
    Err(BudgetTraversalError::StepOutOfBounds { step: node.id })
}

fn new_lane(nodes: &[CompiledNode], boundary: Boundary) -> Result<bool, BudgetTraversalError> {
    let Boundary::Lane { owner, .. } = boundary else {
        return Ok(false);
    };
    Ok(!matches!(
        node_at(nodes, owner)?.kind,
        K::TogetherStart { .. }
    ))
}

fn terminal(
    nodes: &[CompiledNode],
    node: &CompiledNode,
    boundary: Boundary,
) -> Result<Vec<StepIdx>, BudgetTraversalError> {
    if new_lane(nodes, boundary)? {
        return Err(BudgetTraversalError::StepOutOfBounds { step: node.id });
    }
    Ok(Vec::new())
}

fn ordinary_checked(
    nodes: &[CompiledNode],
    node: &CompiledNode,
    boundary: Boundary,
) -> Result<Vec<StepIdx>, BudgetTraversalError> {
    let out = ordinary_targets(node)?;
    if new_lane(nodes, boundary)?
        && (out.is_empty() || out.iter().any(|step| boundary.stops(*step)))
    {
        return Err(BudgetTraversalError::StepOutOfBounds { step: node.id });
    }
    Ok(out)
}
pub(super) fn targets(
    nodes: &[CompiledNode],
    node: &CompiledNode,
    boundary: Boundary,
) -> Result<Vec<StepIdx>, BudgetTraversalError> {
    let mut out = normal_targets(nodes, node, boundary)?;
    if let Some(h) = node.routing.error.handler()
        && !should_queue_error(boundary, h)
    {
        out.push(h);
    }
    Ok(out)
}
