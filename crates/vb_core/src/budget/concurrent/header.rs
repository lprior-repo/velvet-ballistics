use super::node_at;
use crate::budget::BudgetTraversalError;
use crate::ids::StepIdx;
use crate::workflow::{CompiledNode, CompiledNodeKind as K};
fn validate_join(
    nodes: &[CompiledNode],
    node: &CompiledNode,
    join: StepIdx,
) -> Result<(), BudgetTraversalError> {
    if join.as_usize() <= node.id.as_usize() {
        return Err(BudgetTraversalError::BackwardScopeExit {
            from: node.id,
            to: join,
        });
    }
    let j = node_at(nodes, join)?;
    if !matches!(j.kind, K::ConcurrentGroupJoin { owner } if owner == node.id) {
        return Err(BudgetTraversalError::BackwardScopeExit {
            from: node.id,
            to: join,
        });
    }
    Ok(())
}

fn validate_entries(
    nodes: &[CompiledNode],
    node: &CompiledNode,
    branches: &[StepIdx],
    join: StepIdx,
) -> Result<(), BudgetTraversalError> {
    for b in branches {
        if b.as_usize() <= node.id.as_usize() || b.as_usize() >= join.as_usize() {
            return Err(BudgetTraversalError::BackwardScopeExit {
                from: node.id,
                to: *b,
            });
        }
        node_at(nodes, *b)?;
    }
    Ok(())
}

fn validate_new_start(
    nodes: &[CompiledNode],
    node: &CompiledNode,
) -> Result<(), BudgetTraversalError> {
    let (branches, join, at_once) = match node.kind {
        K::ForEachConcurrentStart {
            body,
            join,
            at_once,
            ..
        } => (vec![body], join, Some(at_once)),
        K::TogetherConcurrentStart {
            ref branches, join, ..
        }
        | K::FirstSuccessStart {
            ref branches, join, ..
        } => (branches.to_vec(), join, None),
        _ => return Ok(()),
    };
    if branches.is_empty() {
        return Err(BudgetTraversalError::StepOutOfBounds { step: node.id });
    }
    validate_join(nodes, node, join)?;
    if let Some(n) = at_once
        && n.get() == 0
    {
        return Err(BudgetTraversalError::StepOutOfBounds { step: node.id });
    }
    validate_entries(nodes, node, &branches, join)
}

pub(super) fn validate_start(
    nodes: &[CompiledNode],
    node: &CompiledNode,
) -> Result<(), BudgetTraversalError> {
    validate_new_start(nodes, node)
}
