//! Workflow validation - graph structural validation.

use crate::ids::StepIdx;
use crate::nodes::CompiledNodeKind;
use crate::compiled_workflow::WorkflowParts;
use crate::validation::WorkflowError;

pub(crate) mod targets {
    pub(crate) use super::super::targets::collect_node_targets;
}

pub(crate) fn validate_reachability(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let node_count = parts.nodes.len();
    if node_count == 0 {
        return Ok(());
    }

    let mut visited: Vec<bool> = vec![false; node_count];
    let mut queue: Vec<usize> = Vec::new();

    let entry_usize = parts.entry.as_usize();
    if entry_usize >= node_count {
        return Ok(());
    }
    let Some(entry_flag) = visited.get_mut(entry_usize) else {
        return Err(WorkflowError::EntryOutOfBounds { entry: parts.entry });
    };
    *entry_flag = true;
    queue.push(entry_usize);

    let mut head = 0usize;
    while head < queue.len() {
        let current = match queue.get(head) {
            Some(&v) => v,
            None => break,
        };
        head = match head.checked_add(1) {
            Some(v) => v,
            None => break,
        };

        let mut targets: Vec<StepIdx> = Vec::new();
        let node = match parts.nodes.get(current) {
            Some(n) => n,
            None => break,
        };
        if let Some(next) = node.next {
            targets.push(next);
        }
        targets::collect_node_targets(&node.kind, &mut targets);

        for target in targets {
            let target_usize = target.as_usize();
            if target_usize < node_count {
                let Some(flag) = visited.get_mut(target_usize) else {
                    continue;
                };
                if !*flag {
                    *flag = true;
                    queue.push(target_usize);
                }
            }
        }
    }

    for (index, was_visited) in visited.iter().enumerate() {
        if !was_visited {
            return Err(WorkflowError::UnreachableNode {
                step: StepIdx::new(u16::try_from(index).map_err(|_| {
                    WorkflowError::ResourceContractExceeded {
                        resource: "max_steps",
                    }
                })?),
            });
        }
    }
    Ok(())
}

/// Check B: all edges must point forward except recognized loop back-edges.
/// Check D: loop spans must be properly nested (no overlapping loops).
pub(crate) fn validate_forward_edges(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let mut loop_spans: Vec<(usize, usize)> = Vec::new();

    for (index, node) in parts.nodes.iter().enumerate() {
        let current_id = StepIdx::new(u16::try_from(index).map_err(|_| {
            WorkflowError::ResourceContractExceeded {
                resource: "max_steps",
            }
        })?);

        if let Some(next) = node.next {
            validate_forward_target(next, index, current_id)?;
        }

        validate_kind_edges(&node.kind, index, current_id)?;

        push_loop_span(&node.kind, index, &mut loop_spans)?;
    }
    Ok(())
}

/// Validates that kind-specific edges respect the forward-only rule.
fn validate_kind_edges(
    kind: &CompiledNodeKind,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    match kind {
        // Stateless: no outgoing edges
        CompiledNodeKind::Nop => Ok(()),
        CompiledNodeKind::SetConst { .. } => Ok(()),
        CompiledNodeKind::Copy { .. } => Ok(()),
        CompiledNodeKind::EvalExpr { .. } => Ok(()),
        CompiledNodeKind::BuildObject { .. } => Ok(()),
        CompiledNodeKind::BuildList { .. } => Ok(()),
        CompiledNodeKind::Do { .. } => Ok(()),
        CompiledNodeKind::ForEachJoin { .. } => Ok(()),
        CompiledNodeKind::TogetherJoin { .. } => Ok(()),
        CompiledNodeKind::CollectFinish { .. } => Ok(()),
        CompiledNodeKind::ReduceFinish { .. } => Ok(()),
        CompiledNodeKind::RepeatFinish { .. } => Ok(()),
        CompiledNodeKind::WaitUntil { .. } => Ok(()),
        CompiledNodeKind::WaitEvent { .. } => Ok(()),
        CompiledNodeKind::Ask { .. } => Ok(()),
        CompiledNodeKind::AskResume { .. } => Ok(()),
        CompiledNodeKind::Finish { .. } => Ok(()),
        CompiledNodeKind::Jump { .. } => Ok(()),

        // Choice branches
        CompiledNodeKind::ChooseSlot { branches, otherwise } => {
            validate_choose_slot_edges(branches, *otherwise, ci, cid)
        }
        CompiledNodeKind::Choose { branches, otherwise } => {
            validate_choose_edges(branches, *otherwise, ci, cid)
        }

        // ForEach edges
        CompiledNodeKind::ForEachStart { done, .. } => validate_for_each_start(done, ci, cid),
        CompiledNodeKind::ForEachNext { done, .. } => validate_for_each_next(done, ci, cid),

        // Together edges
        CompiledNodeKind::TogetherStart { join, .. } => validate_together_start(join, ci, cid),
        CompiledNodeKind::TogetherBranch { join, .. } => validate_together_branch(join, ci, cid),

        // Collect/Reduce/Repeat done edges
        CompiledNodeKind::CollectStart { done, .. } => validate_collect_start(done, ci, cid),
        CompiledNodeKind::CollectPage { done, .. } => validate_collect_page(done, ci, cid),
        CompiledNodeKind::CollectNext { done, .. } => validate_collect_next(done, ci, cid),
        CompiledNodeKind::ReduceStart { done, .. } => validate_reduce_start(done, ci, cid),
        CompiledNodeKind::ReduceNext { done, .. } => validate_reduce_next(done, ci, cid),
        CompiledNodeKind::RepeatStart { done, .. } => validate_repeat_start(done, ci, cid),
        CompiledNodeKind::RepeatAttempt { done, .. } => validate_repeat_attempt(done, ci, cid),
        CompiledNodeKind::RepeatCheck { done, .. } => validate_repeat_check(done, ci, cid),

        // Retry exhausted edge
        CompiledNodeKind::RetryCheck { exhausted, .. } => validate_retry_check(exhausted, ci, cid),

        // Error handler edge
        CompiledNodeKind::ErrorHandler { handler, .. } => validate_error_handler(handler, ci, cid),
    }
}

// Per-kind validation functions

fn validate_choose_slot_edges(
    branches: &[crate::nodes::Branch],
    otherwise: Option<StepIdx>,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    for branch in branches {
        validate_forward_target(branch.target, ci, cid)?;
    }
    if let Some(fallback) = otherwise {
        validate_forward_target(fallback, ci, cid)?;
    }
    Ok(())
}

fn validate_choose_edges(
    branches: &[crate::nodes::Branch],
    otherwise: Option<StepIdx>,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    for branch in branches {
        validate_forward_target(branch.target, ci, cid)?;
    }
    if let Some(fallback) = otherwise {
        validate_forward_target(fallback, ci, cid)?;
    }
    Ok(())
}

fn validate_for_each_start(done: &StepIdx, ci: usize, cid: StepIdx) -> Result<(), WorkflowError> {
    validate_forward_target(*done, ci, cid)
}

fn validate_for_each_next(done: &StepIdx, ci: usize, cid: StepIdx) -> Result<(), WorkflowError> {
    validate_forward_target(*done, ci, cid)
}

fn validate_together_start(join: &StepIdx, ci: usize, cid: StepIdx) -> Result<(), WorkflowError> {
    validate_forward_target(*join, ci, cid)
}

fn validate_together_branch(join: &StepIdx, ci: usize, cid: StepIdx) -> Result<(), WorkflowError> {
    validate_forward_target(*join, ci, cid)
}

fn validate_collect_start(done: &StepIdx, ci: usize, cid: StepIdx) -> Result<(), WorkflowError> {
    validate_forward_target(*done, ci, cid)
}

fn validate_collect_page(done: &StepIdx, ci: usize, cid: StepIdx) -> Result<(), WorkflowError> {
    validate_forward_target(*done, ci, cid)
}

fn validate_collect_next(done: &StepIdx, ci: usize, cid: StepIdx) -> Result<(), WorkflowError> {
    validate_forward_target(*done, ci, cid)
}

fn validate_reduce_start(done: &StepIdx, ci: usize, cid: StepIdx) -> Result<(), WorkflowError> {
    validate_forward_target(*done, ci, cid)
}

fn validate_reduce_next(done: &StepIdx, ci: usize, cid: StepIdx) -> Result<(), WorkflowError> {
    validate_forward_target(*done, ci, cid)
}

fn validate_repeat_start(done: &StepIdx, ci: usize, cid: StepIdx) -> Result<(), WorkflowError> {
    validate_forward_target(*done, ci, cid)
}

fn validate_repeat_attempt(done: &StepIdx, ci: usize, cid: StepIdx) -> Result<(), WorkflowError> {
    validate_forward_target(*done, ci, cid)
}

fn validate_repeat_check(done: &StepIdx, ci: usize, cid: StepIdx) -> Result<(), WorkflowError> {
    validate_forward_target(*done, ci, cid)
}

fn validate_retry_check(exhausted: &StepIdx, ci: usize, cid: StepIdx) -> Result<(), WorkflowError> {
    validate_forward_target(*exhausted, ci, cid)
}

fn validate_error_handler(handler: &StepIdx, ci: usize, cid: StepIdx) -> Result<(), WorkflowError> {
    validate_forward_target(*handler, ci, cid)
}

/// Validates a target step is strictly forward from the current node.
fn validate_forward_target(target: StepIdx, ci: usize, cid: StepIdx) -> Result<(), WorkflowError> {
    if target.as_usize() > ci {
        Ok(())
    } else {
        Err(WorkflowError::BackwardEdge {
            from: cid,
            to: target,
        })
    }
}

/// Tracks loop spans for nesting validation (Check D).
pub(crate) fn push_loop_span(
    kind: &CompiledNodeKind,
    ci: usize,
    spans: &mut Vec<(usize, usize)>,
) -> Result<(), WorkflowError> {
    let done_usize: Option<usize> = match kind {
        CompiledNodeKind::ForEachStart { done, .. }
        | CompiledNodeKind::CollectStart { done, .. }
        | CompiledNodeKind::ReduceStart { done, .. }
        | CompiledNodeKind::RepeatStart { done, .. } => Some(done.as_usize()),
        CompiledNodeKind::TogetherStart { join, .. } => Some(join.as_usize()),
        _ => None,
    };

    let Some(done_idx) = done_usize else {
        return Ok(());
    };

    if let Some(&(_outer_start, outer_done)) = spans.last()
        && done_idx > outer_done
    {
        return Err(WorkflowError::ImproperLoopNesting {
            inner: StepIdx::new(u16::try_from(ci).map_err(|_| {
                WorkflowError::ResourceContractExceeded {
                    resource: "max_steps",
                }
            })?),
            outer_done: StepIdx::new(u16::try_from(outer_done).map_err(|_| {
                WorkflowError::ResourceContractExceeded {
                    resource: "max_steps",
                }
            })?),
        });
    }

    while spans
        .last()
        .is_some_and(|&(_, done): &(usize, usize)| done <= ci)
    {
        spans.pop();
    }

    spans.push((ci, done_idx));
    Ok(())
}
