//! Targets for a single owning lane walk of a compiled workflow.
//!
//! Concurrent-group boundaries are handled atomically: nested starts skip to
//! their join and then to the join's continuation; lane ends only return error
//! routing. All other node kinds use graph-successor edges plus optional error
//! handlers.

use crate::ids::StepIdx;
use crate::workflow::{
    CompiledNode, CompiledNodeKind as K, ContractResource, WorkflowError, WorkflowParts,
};

use super::concurrent_pair;
use super::slot_flow::graph;

/// Compute execution targets for one lane step at `node`.
pub(super) fn targets(
    parts: &WorkflowParts,
    node: &CompiledNode,
) -> Result<Vec<StepIdx>, WorkflowError> {
    match &node.kind {
        K::ForEachConcurrentStart { join, .. }
        | K::TogetherConcurrentStart { join, .. }
        | K::FirstSuccessStart { join, .. } => {
            concurrent_pair::validate_concurrent_pair(parts, node.id)?;
            nested_target(parts, node, *join)
        }
        K::ConcurrentLaneEnd { .. } => {
            let mut out = Vec::new();
            if let Some(h) = node.routing.error.handler() {
                reserve_one(&mut out)?;
                out.push(h);
            }
            Ok(out)
        }
        K::ConcurrentGroupJoin { .. } => {
            Err(WorkflowError::LoopRegionUnpaired { control: node.id })
        }
        _ => old_targets(node),
    }
}

/// A nested start jumps to its join, then to the join's continuation.
fn nested_target(
    parts: &WorkflowParts,
    node: &CompiledNode,
    join: StepIdx,
) -> Result<Vec<StepIdx>, WorkflowError> {
    let j = concurrent_pair::at(parts, join)?;
    let next = j
        .routing
        .next
        .step()
        .ok_or(WorkflowError::LoopRegionUnpaired { control: join })?;
    let mut out = Vec::new();
    reserve_one(&mut out)?;
    out.push(next);
    if let Some(h) = node.routing.error.handler() {
        reserve_one(&mut out)?;
        out.push(h);
    }
    if let Some(h) = j.routing.error.handler() {
        reserve_one(&mut out)?;
        out.push(h);
    }
    Ok(out)
}

/// Non-concurrent nodes use graph-successor edges plus optional error handlers.
fn old_targets(node: &CompiledNode) -> Result<Vec<StepIdx>, WorkflowError> {
    let edges = graph::successors(node)?;
    let mut out = Vec::new();
    out.try_reserve_exact(edges.len())
        .map_err(|_| WorkflowError::ResourceContractExceeded {
            resource: ContractResource::LoopRegions,
        })?;
    for e in edges {
        out.push(e.target);
    }
    if let Some(h) = node.routing.error.handler() {
        reserve_one(&mut out)?;
        out.push(h);
    }
    Ok(out)
}

fn reserve_one(v: &mut Vec<StepIdx>) -> Result<(), WorkflowError> {
    v.try_reserve_exact(1)
        .map_err(|_| WorkflowError::ResourceContractExceeded {
            resource: ContractResource::LoopRegions,
        })
}

pub(super) fn has_normal_successor(node: &CompiledNode) -> Result<bool, WorkflowError> {
    match &node.kind {
        K::ForEachConcurrentStart { .. }
        | K::TogetherConcurrentStart { .. }
        | K::FirstSuccessStart { .. } => Ok(true),
        _ => Ok(!graph::successors(node)?.is_empty()),
    }
}
