use crate::ids::StepIdx;
use crate::workflow::{CompiledNodeKind, WorkflowError, WorkflowParts};

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
    let together_starts = collect_together_starts(&parts.nodes);
    if together_starts.len() < 2 {
        return Ok(());
    }
    let together_set = together_starts.iter().copied().collect();
    for node in &parts.nodes {
        if let Some(inner) = find_nested_together_in_node(&parts.nodes, &together_set, node) {
            return Err(WorkflowError::NestedTogether {
                outer: node.id,
                inner,
            });
        }
    }
    Ok(())
}

fn collect_together_starts(nodes: &[crate::workflow::CompiledNode]) -> Vec<StepIdx> {
    nodes
        .iter()
        .filter_map(|node| match &node.kind {
            CompiledNodeKind::TogetherStart { .. }
            | CompiledNodeKind::TogetherConcurrentStart { .. } => Some(node.id),
            _ => None,
        })
        .collect()
}

fn find_nested_together_in_node(
    nodes: &[crate::workflow::CompiledNode],
    together_set: &std::collections::BTreeSet<StepIdx>,
    node: &crate::workflow::CompiledNode,
) -> Option<StepIdx> {
    let (branches, join) = match &node.kind {
        CompiledNodeKind::TogetherStart { branches, join }
        | CompiledNodeKind::TogetherConcurrentStart { branches, join } => (branches, *join),
        _ => return None,
    };
    for branch_entry in branches {
        if let Some(inner) =
            find_nested_together_in_branch(nodes, together_set, branch_entry, join, node.id)
        {
            return Some(inner);
        }
    }
    None
}

fn find_nested_together_in_branch(
    nodes: &[crate::workflow::CompiledNode],
    together_set: &std::collections::BTreeSet<StepIdx>,
    branch_entry: &StepIdx,
    join: StepIdx,
    outer: StepIdx,
) -> Option<StepIdx> {
    let entry_idx = branch_entry.as_usize();
    if entry_idx >= nodes.len() || join.as_usize() >= nodes.len() {
        return None;
    }
    if walk_contains_together_start(nodes, together_set, entry_idx, join.as_usize(), outer) {
        return Some(find_inner_together_start(
            nodes,
            together_set,
            entry_idx,
            join.as_usize(),
            outer,
        ));
    }
    None
}

fn push_nested_group_entries(node: &crate::workflow::CompiledNode, stack: &mut Vec<usize>) {
    match &node.kind {
        CompiledNodeKind::ForEachConcurrentStart { body, .. }
        | CompiledNodeKind::ForEachStart { body, .. }
        | CompiledNodeKind::CollectStart { body, .. }
        | CompiledNodeKind::ReduceStart { body, .. }
        | CompiledNodeKind::RepeatStart { body, .. } => stack.push(body.as_usize()),
        CompiledNodeKind::TogetherStart { branches, .. }
        | CompiledNodeKind::TogetherConcurrentStart { branches, .. }
        | CompiledNodeKind::FirstSuccessStart { branches, .. } => {
            stack.extend(branches.iter().map(|step| step.as_usize()));
        }
        _ => {}
    }
    if let Some(next) = node.routing.next.step() {
        stack.push(next.as_usize());
    }
}

enum TogetherWalkVisit {
    Continue,
    Found(StepIdx),
    Overflow,
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
        match visit_together_node(
            nodes,
            together_set,
            idx,
            stop_idx,
            outer,
            &mut visited,
            &mut stack,
        ) {
            TogetherWalkVisit::Found(_) => return true,
            TogetherWalkVisit::Continue | TogetherWalkVisit::Overflow => {}
        }
    }
    false
}

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
        match visit_together_node(
            nodes,
            together_set,
            idx,
            stop_idx,
            outer,
            &mut visited,
            &mut stack,
        ) {
            TogetherWalkVisit::Found(step) => return step,
            TogetherWalkVisit::Overflow => return outer,
            TogetherWalkVisit::Continue => {}
        }
    }
    outer
}

fn visit_together_node(
    nodes: &[crate::workflow::CompiledNode],
    together_set: &std::collections::BTreeSet<StepIdx>,
    idx: usize,
    stop_idx: usize,
    outer: StepIdx,
    visited: &mut std::collections::BTreeSet<usize>,
    stack: &mut Vec<usize>,
) -> TogetherWalkVisit {
    if idx >= nodes.len() || idx == stop_idx || !visited.insert(idx) {
        return TogetherWalkVisit::Continue;
    }
    let Ok(step_u16) = u16::try_from(idx) else {
        return TogetherWalkVisit::Overflow;
    };
    let step = StepIdx::new(step_u16);
    if step != outer && together_set.contains(&step) {
        return TogetherWalkVisit::Found(step);
    }
    let Some(node) = nodes.get(idx) else {
        return TogetherWalkVisit::Continue;
    };
    push_nested_group_entries(node, stack);
    TogetherWalkVisit::Continue
}
