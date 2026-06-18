#![forbid(unsafe_code)]
//! Graph reachability validation.
//!
//! Ensures every node is reachable from the entry step via a forward walk
/// following `next` edges and kind-specific targets.

use crate::ids::StepIdx;
use crate::workflow::{WorkflowError, WorkflowParts};

/// Validates that every node is reachable from the entry step.
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
        if let Some(handler) = node.on_error {
            targets.push(handler);
        }
        collect_node_targets(&node.kind, &mut targets);

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

/// Collects all [`StepIdx`] targets referenced by a node kind (branch targets,
/// loop body/done, jump target, etc.) but NOT the `next` field.
fn collect_node_targets(
    kind: &crate::workflow::CompiledNodeKind,
    targets: &mut Vec<StepIdx>,
) {
    match kind {
        crate::workflow::CompiledNodeKind::Nop
        | crate::workflow::CompiledNodeKind::SetConst { .. }
        | crate::workflow::CompiledNodeKind::Copy { .. }
        | crate::workflow::CompiledNodeKind::EvalExpr { .. }
        | crate::workflow::CompiledNodeKind::BuildObject { .. }
        | crate::workflow::CompiledNodeKind::BuildList { .. }
        | crate::workflow::CompiledNodeKind::Do { .. }
        | crate::workflow::CompiledNodeKind::ForEachJoin { .. }
        | crate::workflow::CompiledNodeKind::CollectFinish { .. }
        | crate::workflow::CompiledNodeKind::ReduceFinish { .. }
        | crate::workflow::CompiledNodeKind::RepeatFinish { .. }
        | crate::workflow::CompiledNodeKind::WaitUntil { .. }
        | crate::workflow::CompiledNodeKind::Ask { .. }
        | crate::workflow::CompiledNodeKind::AskResume { .. }
        | crate::workflow::CompiledNodeKind::Finish { .. }
        | crate::workflow::CompiledNodeKind::TogetherJoin { .. }
        | crate::workflow::CompiledNodeKind::WaitEvent { .. } => {}
        crate::workflow::CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            collect_choose_slot_targets(branches, *otherwise, targets);
        }
        crate::workflow::CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => {
            collect_choose_expr_targets(branches, *otherwise, targets);
        }
        crate::workflow::CompiledNodeKind::ForEachStart { body, done, .. }
        | crate::workflow::CompiledNodeKind::ForEachNext { body, done, .. }
        | crate::workflow::CompiledNodeKind::CollectStart { body, done, .. }
        | crate::workflow::CompiledNodeKind::CollectPage { body, done, .. }
        | crate::workflow::CompiledNodeKind::CollectNext { body, done, .. }
        | crate::workflow::CompiledNodeKind::ReduceStart { body, done, .. }
        | crate::workflow::CompiledNodeKind::ReduceNext { body, done, .. }
        | crate::workflow::CompiledNodeKind::RepeatStart { body, done, .. }
        | crate::workflow::CompiledNodeKind::RepeatAttempt { body, done, .. }
        | crate::workflow::CompiledNodeKind::RetryCheck {
            body,
            exhausted: done,
            ..
        } => {
            targets.push(*body);
            targets.push(*done);
        }
        crate::workflow::CompiledNodeKind::RepeatCheck { done, .. } => {
            targets.push(*done);
        }
        crate::workflow::CompiledNodeKind::TogetherStart { branches, join } => {
            collect_together_start_targets(branches, *join, targets);
        }
        crate::workflow::CompiledNodeKind::TogetherBranch { entry, join, .. } => {
            targets.push(*entry);
            targets.push(*join);
        }
        crate::workflow::CompiledNodeKind::ErrorHandler { body, handler, .. } => {
            targets.push(*body);
            targets.push(*handler);
        }
        crate::workflow::CompiledNodeKind::Jump { target } => {
            targets.push(*target);
        }
    }
}

fn collect_choose_slot_targets(
    branches: &[crate::workflow::SlotBranch],
    otherwise: Option<StepIdx>,
    targets: &mut Vec<StepIdx>,
) {
    for branch in branches {
        targets.push(branch.target);
    }
    if let Some(fallback) = otherwise {
        targets.push(fallback);
    }
}

fn collect_choose_expr_targets(
    branches: &[crate::workflow::ExprBranch],
    otherwise: Option<StepIdx>,
    targets: &mut Vec<StepIdx>,
) {
    for branch in branches {
        targets.push(branch.target);
    }
    if let Some(fallback) = otherwise {
        targets.push(fallback);
    }
}

fn collect_together_start_targets(
    branches: &[StepIdx],
    join: StepIdx,
    targets: &mut Vec<StepIdx>,
) {
    for branch in branches {
        targets.push(*branch);
    }
    targets.push(join);
}
