use crate::ids::StepIdx;
use crate::workflow::traversal::collect_node_targets;
use crate::workflow::{CompiledNode, CompiledNodeKind as K, ContractResource, WorkflowError};

fn branches(kind: &K) -> usize {
    match kind {
        K::Choose { branches, .. } => branches.len(),
        K::ChooseSlot { branches, .. } => branches.len(),
        K::TogetherStart { branches, .. } => branches.len(),
        K::TogetherConcurrentStart { branches, .. } => branches.len(),
        K::FirstSuccessStart { branches, .. } => branches.len(),
        _ => 1,
    }
}

fn capacity_error() -> WorkflowError {
    WorkflowError::ResourceContractExceeded {
        resource: ContractResource::LoopRegions,
    }
}

pub(super) fn of(node: &CompiledNode) -> Result<Vec<StepIdx>, WorkflowError> {
    let capacity = branches(&node.kind)
        .checked_add(3)
        .ok_or_else(capacity_error)?;
    let mut out = Vec::new();
    out.try_reserve_exact(capacity)
        .map_err(|_| capacity_error())?;
    if let Some(next) = node.routing.next.step() {
        out.push(next);
    }
    if let Some(handler) = node.routing.error.handler() {
        out.push(handler);
    }
    collect_node_targets(&node.kind, &mut out);
    Ok(out)
}
