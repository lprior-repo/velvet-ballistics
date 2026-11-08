#![forbid(unsafe_code)]
//! Structural-depth computation over the workflow graph.

use crate::ids::StepIdx;
use crate::workflow::CompiledNodeKind;

use super::BudgetTraversalError;
use super::find_node_position;
use super::successors::bounded_tracking_vec;

/// Cold structural upper bound for vb-8eocj.3.6, independent of DFS order.
/// Reachable owner spans are half-open: an exit retires before a start there.
/// Alternate-entry IR can conservatively overlap mutually exclusive owners;
/// canonical source scopes have their exact lexical nesting depth.
pub(super) fn compute_structural_depth(
    nodes: &[crate::workflow::CompiledNode],
    visited: &[bool],
) -> Result<u16, BudgetTraversalError> {
    let mut owners = reachable_nesting_owners(nodes, visited)?;
    owners.sort_unstable_by_key(|(node, _)| node.id);
    let mut active_exits = bounded_tracking_vec(owners.len());
    let mut max_depth = 0;
    for (node, exit) in owners {
        active_exits.retain(|end| *end > node.id);
        let depth = u16::try_from(active_exits.len())
            .map_err(|_| BudgetTraversalError::DepthOverflow { depth: u16::MAX })?;
        compute_child_depth(&node.kind, depth, &mut max_depth)?;
        active_exits.push(exit);
    }
    Ok(max_depth)
}

fn reachable_nesting_owners<'a>(
    nodes: &'a [crate::workflow::CompiledNode],
    visited: &[bool],
) -> Result<Vec<(&'a crate::workflow::CompiledNode, StepIdx)>, BudgetTraversalError> {
    let mut owners = bounded_tracking_vec(nodes.len());
    for (index, node) in nodes.iter().enumerate() {
        if visited.get(index).copied() != Some(true) {
            continue;
        }
        let Some(exit) = nesting_scope_exit(&node.kind) else {
            continue;
        };
        find_node_position(nodes, exit, nodes.len())?;
        if exit <= node.id {
            return Err(BudgetTraversalError::BackwardScopeExit {
                from: node.id,
                to: exit,
            });
        }
        owners.push((node, exit));
    }
    Ok(owners)
}

fn nesting_scope_exit(kind: &CompiledNodeKind) -> Option<StepIdx> {
    match kind {
        CompiledNodeKind::ForEachStart { done, .. }
        | CompiledNodeKind::CollectStart { done, .. }
        | CompiledNodeKind::ReduceStart { done, .. }
        | CompiledNodeKind::RepeatStart { done, .. } => Some(*done),
        CompiledNodeKind::TogetherStart { join, .. } => Some(*join),
        _ => None,
    }
}

pub(super) fn compute_child_depth(
    kind: &CompiledNodeKind,
    current_depth: u16,
    max_nesting_depth: &mut u16,
) -> Result<u16, BudgetTraversalError> {
    match kind {
        CompiledNodeKind::ForEachStart { .. }
        | CompiledNodeKind::CollectStart { .. }
        | CompiledNodeKind::ReduceStart { .. }
        | CompiledNodeKind::RepeatStart { .. }
        | CompiledNodeKind::TogetherStart { .. } => {
            let new_depth =
                current_depth
                    .checked_add(1)
                    .ok_or(BudgetTraversalError::DepthOverflow {
                        depth: current_depth,
                    })?;
            if new_depth > *max_nesting_depth {
                *max_nesting_depth = new_depth;
            }
            Ok(new_depth)
        }
        _ => Ok(current_depth),
    }
}
