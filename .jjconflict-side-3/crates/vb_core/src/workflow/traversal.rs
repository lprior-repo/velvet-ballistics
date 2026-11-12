#![forbid(unsafe_code)]
//! Reachability traversal validation.

use super::{
    BranchContinuation, CompiledNodeKind, ContractResource, ExprBranch, SlotBranch, WorkflowError,
    WorkflowParts,
};
use crate::ids::StepIdx;

/// Check A: every node must be reachable from the entry step via a forward walk
/// following `next` edges and kind-specific targets.
pub(super) fn validate_reachability(parts: &WorkflowParts) -> Result<(), WorkflowError> {
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
        if let Some(next) = node.routing.next.step() {
            targets.push(next);
        }
        if let Some(handler) = node.routing.error.handler() {
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
                        resource: ContractResource::Steps,
                    }
                })?),
            });
        }
    }
    Ok(())
}

/// Collects all StepIdx targets referenced by a node kind (branch targets,
/// loop body/done, jump target, etc.) but NOT the `next` field.
pub(super) fn collect_node_targets(kind: &CompiledNodeKind, targets: &mut Vec<StepIdx>) {
    match kind {
        CompiledNodeKind::Nop
        | CompiledNodeKind::SetConst { .. }
        | CompiledNodeKind::Copy { .. }
        | CompiledNodeKind::EvalExpr { .. }
        | CompiledNodeKind::BuildObject { .. }
        | CompiledNodeKind::BuildList { .. }
        | CompiledNodeKind::Do { .. }
        | CompiledNodeKind::ForEachJoin { .. }
        | CompiledNodeKind::CollectFinish { .. }
        | CompiledNodeKind::ReduceFinish { .. }
        | CompiledNodeKind::RepeatFinish { .. }
        | CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::AskResume { .. }
        | CompiledNodeKind::Finish { .. }
        | CompiledNodeKind::TogetherJoin { .. }
        | CompiledNodeKind::WaitEvent { .. } => {}
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            collect_choose_slot_targets(branches, *otherwise, targets);
        }
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => {
            collect_choose_expr_targets(branches, *otherwise, targets);
        }
        CompiledNodeKind::ForEachStart { body, done, .. }
        | CompiledNodeKind::ForEachNext { body, done, .. }
        | CompiledNodeKind::CollectStart { body, done, .. }
        | CompiledNodeKind::CollectPage { body, done, .. }
        | CompiledNodeKind::CollectNext { body, done, .. }
        | CompiledNodeKind::ReduceStart { body, done, .. }
        | CompiledNodeKind::ReduceNext { body, done, .. }
        | CompiledNodeKind::RepeatStart { body, done, .. }
        | CompiledNodeKind::RepeatAttempt { body, done, .. }
        | CompiledNodeKind::RetryCheck {
            body,
            exhausted: done,
            ..
        } => {
            targets.push(*body);
            targets.push(*done);
        }
        CompiledNodeKind::RepeatCheck { done, .. } => {
            targets.push(*done);
        }
        CompiledNodeKind::TogetherStart { branches, join } => {
            collect_together_start_targets(branches, *join, targets);
        }
        CompiledNodeKind::TogetherBranch { entry, join, .. } => {
            targets.push(*entry);
            targets.push(*join);
        }
        CompiledNodeKind::ErrorHandler { body, handler, .. } => {
            targets.push(*body);
            targets.push(*handler);
        }
        CompiledNodeKind::Jump { target } => {
            targets.push(*target);
        }
    }
}

fn collect_choose_slot_targets(
    branches: &[SlotBranch],
    otherwise: BranchContinuation,
    targets: &mut Vec<StepIdx>,
) {
    for branch in branches {
        targets.push(branch.target);
    }
    if let Some(fallback) = otherwise.step() {
        targets.push(fallback);
    }
}

fn collect_choose_expr_targets(
    branches: &[ExprBranch],
    otherwise: BranchContinuation,
    targets: &mut Vec<StepIdx>,
) {
    for branch in branches {
        targets.push(branch.target);
    }
    if let Some(fallback) = otherwise.step() {
        targets.push(fallback);
    }
}

fn collect_together_start_targets(branches: &[StepIdx], join: StepIdx, targets: &mut Vec<StepIdx>) {
    for branch in branches {
        targets.push(*branch);
    }
    targets.push(join);
}
