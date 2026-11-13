use crate::ids::StepIdx;
use crate::workflow::{CompiledNode, CompiledNodeKind as K, WorkflowError, WorkflowParts};

#[path = "together_policy_walk.rs"]
mod walk;

pub(super) fn starts(node: &CompiledNode) -> Option<(&[StepIdx], StepIdx)> {
    match &node.kind {
        K::TogetherStart { branches, join } => Some((branches.as_ref(), *join)),
        K::TogetherConcurrentStart { branches, join } => Some((branches.as_ref(), *join)),
        _ => None,
    }
}

pub(super) fn is_together(kind: &K) -> bool {
    matches!(
        kind,
        K::TogetherStart { .. } | K::TogetherConcurrentStart { .. }
    )
}

fn has_concurrent_start(parts: &WorkflowParts) -> bool {
    parts.nodes.iter().any(|node| {
        matches!(
            node.kind,
            K::ForEachConcurrentStart { .. }
                | K::TogetherConcurrentStart { .. }
                | K::FirstSuccessStart { .. }
        )
    })
}

pub(super) fn validate(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    if !has_concurrent_start(parts) {
        return Ok(());
    }
    let count = parts
        .nodes
        .iter()
        .filter(|n| is_together(&n.kind))
        .take(2)
        .count();
    if count < 2 {
        return Ok(());
    }

    for node in parts.nodes.iter() {
        let Some((branches, join)) = starts(node) else {
            continue;
        };
        for entry in branches {
            if let Some(inner) = walk::find(parts, node.id, *entry, join)? {
                return Err(WorkflowError::NestedTogether {
                    outer: node.id,
                    inner,
                });
            }
        }
    }

    Ok(())
}
