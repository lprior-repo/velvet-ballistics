#![forbid(unsafe_code)]
use crate::budget::BudgetTraversalError;
use crate::ids::StepIdx;
use crate::workflow::CompiledNode;
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum Boundary {
    Run,
    Lane {
        owner: StepIdx,
        join: StepIdx,
    },
    Loop {
        owner: StepIdx,
        body: StepIdx,
        done: StepIdx,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Key {
    pub(super) entry: StepIdx,
    pub(super) boundary: Boundary,
    pub(super) active: bool,
}

pub(super) fn node_at(
    nodes: &[CompiledNode],
    id: StepIdx,
) -> Result<&CompiledNode, BudgetTraversalError> {
    nodes
        .get(id.as_usize())
        .filter(|n| n.id == id)
        .or_else(|| nodes.iter().find(|node| node.id == id))
        .ok_or(BudgetTraversalError::StepOutOfBounds { step: id })
}

impl Boundary {
    pub(super) fn stops(&self, step: StepIdx) -> bool {
        match *self {
            Boundary::Lane { join, .. } => step == join,
            Boundary::Loop { done, .. } => step == done,
            Boundary::Run => false,
        }
    }
}

#[path = "dependencies.rs"]
mod dependencies;
#[path = "edges.rs"]
mod edges;
#[path = "header.rs"]
mod header;
#[path = "legacy.rs"]
mod legacy;

pub(super) fn dependencies(node: &CompiledNode, boundary: Boundary, active: bool) -> Vec<Key> {
    let active = active
        && !matches!(node.kind,
        crate::workflow::CompiledNodeKind::ForEachConcurrentStart { limit, .. } if limit.get() == 0);
    dependencies::build(node, boundary)
        .into_iter()
        .map(|key| Key { active, ..key })
        .collect()
}
pub(super) fn validate_start(
    nodes: &[CompiledNode],
    node: &CompiledNode,
) -> Result<(), BudgetTraversalError> {
    header::validate_start(nodes, node)
}
pub(super) fn targets(
    nodes: &[CompiledNode],
    node: &CompiledNode,
    boundary: Boundary,
) -> Result<Vec<StepIdx>, BudgetTraversalError> {
    edges::targets(nodes, node, boundary)
}

pub(super) fn stops(
    nodes: &[CompiledNode],
    boundary: Boundary,
    step: StepIdx,
) -> Result<bool, BudgetTraversalError> {
    if !boundary.stops(step) {
        return Ok(false);
    }
    if let Boundary::Lane { owner, .. } = boundary
        && !matches!(
            node_at(nodes, owner)?.kind,
            crate::workflow::CompiledNodeKind::TogetherStart { .. }
        )
    {
        return Err(BudgetTraversalError::StepOutOfBounds { step });
    }
    Ok(true)
}
