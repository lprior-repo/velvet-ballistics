#![forbid(unsafe_code)]

use crate::budget::{BudgetTraversalError, WholeWorkflowBudget};
use crate::ids::StepIdx;
use crate::workflow::{CompiledNode, CompiledNodeKind as K, ResourceContract};

mod compose;
mod engine;
mod metrics;
mod output;
mod scope;

pub(super) fn contains_new_nodes(nodes: &[CompiledNode]) -> bool {
    nodes.iter().any(|node| {
        matches!(
            node.kind,
            K::ForEachConcurrentStart { .. }
                | K::TogetherConcurrentStart { .. }
                | K::FirstSuccessStart { .. }
                | K::ConcurrentLaneEnd { .. }
                | K::ConcurrentGroupJoin { .. }
        )
    })
}

pub(super) fn compute(
    nodes: &[CompiledNode],
    entry: StepIdx,
    contract: &ResourceContract,
) -> Result<WholeWorkflowBudget, BudgetTraversalError> {
    output::budget(engine::analyze(nodes, entry)?, contract)
}

fn roots(nodes: &[CompiledNode]) -> Vec<&CompiledNode> {
    let mut incoming = std::collections::HashSet::new();
    for node in nodes {
        let mut targets = Vec::new();
        crate::budget::successors::push_successor_targets(&node.kind, &mut targets);
        targets.extend(node.routing.next.step());
        targets.extend(node.routing.error.handler());
        incoming.extend(targets);
    }
    nodes
        .iter()
        .filter(|node| !incoming.contains(&node.id))
        .collect()
}

pub(super) fn peak(nodes: &[CompiledNode]) -> Result<u64, BudgetTraversalError> {
    let mut roots = roots(nodes).into_iter();
    let entry = nodes
        .first()
        .ok_or(BudgetTraversalError::EntryOutOfBounds {
            entry: StepIdx::new(0),
        })?
        .id;
    let first = roots.next().ok_or(BudgetTraversalError::JumpCycle {
        step: entry,
        target: entry,
    })?;
    let initial = u64::from(engine::analyze(nodes, first.id)?.parallel);
    roots.try_fold(initial, |peak, node| {
        engine::analyze(nodes, node.id).map(|metrics| peak.max(u64::from(metrics.parallel)))
    })
}
