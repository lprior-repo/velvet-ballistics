#![forbid(unsafe_code)]
//! Workflow validation functions.

use crate::ids::StepIdx;
use crate::workflow::{
    CompiledNodeKind, CompiledWorkflow, ExprBranch, SlotBranch, WorkflowError, WorkflowParts,
};

/// Validates compiled workflow IR integrity.
pub fn validate_compiled_workflow(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    CompiledWorkflow::try_from_parts(parts.clone())?;
    Ok(())
}

/// Validates resource contract bounds against hard limits.
///
/// Bounds-only pre-check: delegates to the canonical
/// `check_resource_contract_limits` implementation shared
/// with admission (`CompiledWorkflow::try_from_parts`). Actual-usage
/// (`Exceeded`) checks are admission-only and intentionally absent here.
pub fn validate_resource_contract(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    crate::workflow::check_resource_contract_limits(&parts.resource_contract)
}

/// Validates that all node indices are within the node array bounds.
pub fn validate_node_bounds(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let node_count = parts.nodes.len();
    for node in &parts.nodes {
        if node.id.as_usize() >= node_count {
            return Err(WorkflowError::StepOutOfBounds { step: node.id });
        }
        if let Some(next) = node.routing.next.step()
            && next.as_usize() >= node_count
        {
            return Err(WorkflowError::StepOutOfBounds { step: next });
        }
    }
    Ok(())
}

/// Validates that all step transition targets reference valid node indices.
pub fn validate_transition_target(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let node_count = parts.nodes.len();
    for node in &parts.nodes {
        match &node.kind {
            CompiledNodeKind::Jump { target } if target.as_usize() >= node_count => {
                return Err(WorkflowError::StepOutOfBounds { step: *target });
            }
            CompiledNodeKind::Choose {
                branches,
                otherwise,
            } => {
                validate_branch_targets(branches, *otherwise, node_count)?;
            }
            CompiledNodeKind::ChooseSlot {
                branches,
                otherwise,
            } => {
                validate_slot_branch_targets(branches, *otherwise, node_count)?;
            }
            CompiledNodeKind::ForEachStart { body, done, .. }
            | CompiledNodeKind::ForEachNext { body, done, .. }
            | CompiledNodeKind::CollectStart { body, done, .. }
            | CompiledNodeKind::CollectPage { body, done, .. }
            | CompiledNodeKind::CollectNext { body, done, .. }
            | CompiledNodeKind::ReduceStart { body, done, .. }
            | CompiledNodeKind::ReduceNext { body, done, .. }
            | CompiledNodeKind::RepeatStart { body, done, .. }
            | CompiledNodeKind::RepeatAttempt { body, done, .. } => {
                validate_two_step_targets(*body, *done, node_count)?;
            }
            CompiledNodeKind::TogetherStart { branches, join } => {
                for branch in branches {
                    if branch.as_usize() >= node_count {
                        return Err(WorkflowError::StepOutOfBounds { step: *branch });
                    }
                }
                if join.as_usize() >= node_count {
                    return Err(WorkflowError::StepOutOfBounds { step: *join });
                }
            }
            CompiledNodeKind::TogetherBranch { entry, join, .. } => {
                validate_two_step_targets(*entry, *join, node_count)?;
            }
            CompiledNodeKind::RepeatCheck { done, .. } if done.as_usize() >= node_count => {
                return Err(WorkflowError::StepOutOfBounds { step: *done });
            }
            CompiledNodeKind::RetryCheck {
                body, exhausted, ..
            } => {
                validate_two_step_targets(*body, *exhausted, node_count)?;
            }
            CompiledNodeKind::ErrorHandler { body, handler, .. } => {
                validate_two_step_targets(*body, *handler, node_count)?;
            }
            _ => {}
        }
    }
    // Reject nested `TogetherStart` (RE-001, option (b)): nesting causes
    // `compute_max_parallel_in_flight` (in vb_runtime::engine::drive) to
    // report a misleading per-node maximum and yields spurious
    // `ParallelLimitExceeded` failures at runtime. Surface the structural
    // problem at validation time with a typed error.
    validate_no_nested_together(parts)?;
    Ok(())
}

/// Rejects `TogetherStart` nodes that are reachable from the body of
/// another `TogetherStart` node.
///
/// `compute_max_parallel_in_flight` (in `vb_runtime::engine::drive`)
/// returns a per-node maximum branch count rather than the true peak
/// parallel-in-flight, so a nested `TogetherStart` produces a
/// `ParallelLimitExceeded` at runtime even when no real concurrency
/// limit has been violated. Surfacing the structural error at compile
/// time (RE-001, option (b)) gives operators a typed, deterministic
/// rejection with a precise location instead of a misleading runtime
/// signal.
pub fn validate_no_nested_together(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let node_count = parts.nodes.len();
    // Identify all `TogetherStart` step indices for quick lookup.
    let together_starts: Vec<StepIdx> = parts
        .nodes
        .iter()
        .filter_map(|node| match &node.kind {
            CompiledNodeKind::TogetherStart { .. } => Some(node.id),
            _ => None,
        })
        .collect();
    if together_starts.len() < 2 {
        // Cannot be nested with zero or one `TogetherStart` nodes.
        return Ok(());
    }
    let together_set: std::collections::BTreeSet<StepIdx> =
        together_starts.iter().copied().collect();
    for node in &parts.nodes {
        let CompiledNodeKind::TogetherStart { branches, join } = &node.kind else {
            continue;
        };
        let outer = node.id;
        for branch_entry in branches.iter() {
            let entry_idx = branch_entry.as_usize();
            if entry_idx >= node_count {
                // Out-of-bounds branch is reported by `validate_transition_target`.
                continue;
            }
            let join_idx = join.as_usize();
            if join_idx >= node_count {
                continue;
            }
            // Depth-first walk from `branch_entry`; stop at the `join` step
            // (which is the outer TogetherStart's join, exclusive). The walk
            // follows the linear `next` chain emitted by the compiler for
            // TogetherStart branch bodies, so it stays bounded to the outer
            // TogetherStart's body.
            if walk_contains_together_start(&parts.nodes, &together_set, entry_idx, join_idx, outer)
            {
                return Err(WorkflowError::NestedTogether {
                    outer,
                    inner: find_inner_together_start(
                        &parts.nodes,
                        &together_set,
                        entry_idx,
                        join_idx,
                        outer,
                    ),
                });
            }
        }
    }
    Ok(())
}

/// Walks nodes starting at `start_idx` and following each node's `next`
/// field, stopping once `stop_idx` is reached (exclusive). Returns `true`
/// if any reachable node is a `TogetherStart` distinct from `outer`.
fn walk_contains_together_start(
    nodes: &[crate::workflow::CompiledNode],
    together_set: &std::collections::BTreeSet<StepIdx>,
    start_idx: usize,
    stop_idx: usize,
    outer: StepIdx,
) -> bool {
    let mut visited: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    let mut stack: Vec<usize> = vec![start_idx];
    while let Some(idx) = stack.pop() {
        if idx >= nodes.len() || idx == stop_idx || !visited.insert(idx) {
            continue;
        }
        let Ok(step_u16) = u16::try_from(idx) else {
            continue;
        };
        let step = StepIdx::new(step_u16);
        if step != outer && together_set.contains(&step) {
            return true;
        }
        let Some(node) = nodes.get(idx) else {
            continue;
        };
        if let Some(next) = node.routing.next.step() {
            stack.push(next.as_usize());
        }
    }
    false
}

/// Helper that returns the first inner `TogetherStart` step reachable
/// from `start_idx` (without crossing `stop_idx`).
fn find_inner_together_start(
    nodes: &[crate::workflow::CompiledNode],
    together_set: &std::collections::BTreeSet<StepIdx>,
    start_idx: usize,
    stop_idx: usize,
    outer: StepIdx,
) -> StepIdx {
    let mut visited: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    let mut stack: Vec<usize> = vec![start_idx];
    while let Some(idx) = stack.pop() {
        if idx >= nodes.len() || idx == stop_idx || !visited.insert(idx) {
            continue;
        }
        let Ok(step_u16) = u16::try_from(idx) else {
            return outer;
        };
        let step = StepIdx::new(step_u16);
        if step != outer && together_set.contains(&step) {
            return step;
        }
        let Some(node) = nodes.get(idx) else {
            continue;
        };
        if let Some(next) = node.routing.next.step() {
            stack.push(next.as_usize());
        }
    }
    outer
}

fn validate_branch_targets(
    branches: &[ExprBranch],
    otherwise: Option<StepIdx>,
    node_count: usize,
) -> Result<(), WorkflowError> {
    for branch in branches {
        if branch.target.as_usize() >= node_count {
            return Err(WorkflowError::StepOutOfBounds {
                step: branch.target,
            });
        }
    }
    if let Some(target) = otherwise
        && target.as_usize() >= node_count
    {
        return Err(WorkflowError::StepOutOfBounds { step: target });
    }
    Ok(())
}

fn validate_slot_branch_targets(
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
    node_count: usize,
) -> Result<(), WorkflowError> {
    for branch in branches {
        if branch.target.as_usize() >= node_count {
            return Err(WorkflowError::StepOutOfBounds {
                step: branch.target,
            });
        }
    }
    if let Some(target) = otherwise
        && target.as_usize() >= node_count
    {
        return Err(WorkflowError::StepOutOfBounds { step: target });
    }
    Ok(())
}

fn validate_two_step_targets(
    first: StepIdx,
    second: StepIdx,
    node_count: usize,
) -> Result<(), WorkflowError> {
    if first.as_usize() >= node_count {
        return Err(WorkflowError::StepOutOfBounds { step: first });
    }
    if second.as_usize() >= node_count {
        return Err(WorkflowError::StepOutOfBounds { step: second });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
