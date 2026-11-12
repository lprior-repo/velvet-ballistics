#![forbid(unsafe_code)]
//! Successor expansion and tracked visited-path bookkeeping for workflow graph traversal.

use crate::ids::StepIdx;
use crate::workflow::CompiledNodeKind;

use super::BudgetTraversalError;

/// Pushes all successor StepIdx targets from a node kind onto the stack,
/// excluding the `next` field which is handled separately.
pub(super) fn push_successor_targets(kind: &CompiledNodeKind, stack: &mut Vec<StepIdx>) {
    if node_kind_has_no_successors(kind) {
        return;
    }
    match kind {
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => push_slot_choose_successors(branches, otherwise.step(), stack),
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => push_expr_choose_successors(branches, otherwise.step(), stack),
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
        } => push_loop_successors(*body, *done, stack),
        CompiledNodeKind::RepeatCheck { done, .. } => push_repeat_check_successors(*done, stack),
        CompiledNodeKind::TogetherStart { branches, join } => {
            push_together_start_successors(branches, *join, stack)
        }
        CompiledNodeKind::TogetherBranch { entry, join, .. } => {
            push_together_branch_successors(*entry, *join, stack)
        }
        CompiledNodeKind::ErrorHandler { body, handler, .. } => {
            push_error_handler_successors(*body, *handler, stack)
        }
        CompiledNodeKind::Jump { target } => stack.push(*target),
        _ => {}
    }
}

/// Returns true if the node kind has no successor targets.
#[inline]
fn node_kind_has_no_successors(kind: &CompiledNodeKind) -> bool {
    matches!(
        kind,
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
            | CompiledNodeKind::WaitEvent { .. }
    )
}

/// Push Choose successors: all branch targets + optional fallback.
fn push_expr_choose_successors(
    branches: &[crate::workflow::ExprBranch],
    otherwise: Option<StepIdx>,
    stack: &mut Vec<StepIdx>,
) {
    for branch in branches {
        stack.push(branch.target);
    }
    if let Some(fallback) = otherwise {
        stack.push(fallback);
    }
}

/// Push ChooseSlot successors: all slot branch targets + optional fallback.
fn push_slot_choose_successors(
    branches: &[crate::workflow::SlotBranch],
    otherwise: Option<StepIdx>,
    stack: &mut Vec<StepIdx>,
) {
    for branch in branches {
        stack.push(branch.target);
    }
    if let Some(fallback) = otherwise {
        stack.push(fallback);
    }
}

/// Push loop successors: body + done targets.
fn push_loop_successors(body: StepIdx, done: StepIdx, stack: &mut Vec<StepIdx>) {
    stack.push(body);
    stack.push(done);
}

/// Push RepeatCheck successor: done target only.
fn push_repeat_check_successors(done: StepIdx, stack: &mut Vec<StepIdx>) {
    stack.push(done);
}

/// Push TogetherStart successors: all branch targets + join.
fn push_together_start_successors(branches: &[StepIdx], join: StepIdx, stack: &mut Vec<StepIdx>) {
    for branch in branches {
        stack.push(*branch);
    }
    stack.push(join);
}

/// Push TogetherBranch successors: entry + join.
fn push_together_branch_successors(entry: StepIdx, join: StepIdx, stack: &mut Vec<StepIdx>) {
    stack.push(entry);
    stack.push(join);
}

/// Push ErrorHandler successors: body + handler.
fn push_error_handler_successors(body: StepIdx, handler: StepIdx, stack: &mut Vec<StepIdx>) {
    stack.push(body);
    stack.push(handler);
}

pub(super) fn branch_count_to_u16(count: usize) -> Result<u16, BudgetTraversalError> {
    match u16::try_from(count) {
        Ok(value) => Ok(value),
        Err(_) => Err(BudgetTraversalError::StepCountOverflow {
            actual: usize_to_u64_saturating(count),
        }),
    }
}

fn usize_to_u64_saturating(value: usize) -> u64 {
    u64::try_from(value).map_or(u64::MAX, core::convert::identity)
}

pub(super) fn bounded_tracking_vec<T>(node_count: usize) -> Vec<T> {
    Vec::with_capacity(node_count)
}

pub(super) fn tracked_steps_contain(steps: &[u16], step: u16) -> bool {
    steps.iter().copied().any(|candidate| candidate == step)
}

pub(super) fn insert_tracked_step(
    steps: &mut Vec<u16>,
    step: u16,
    limit: usize,
) -> Result<bool, BudgetTraversalError> {
    if tracked_steps_contain(steps, step) {
        return Ok(false);
    }
    if steps.len() >= limit {
        return Err(BudgetTraversalError::StepCountOverflow { actual: u64::MAX });
    }
    steps.push(step);
    Ok(true)
}

pub(super) fn remove_tracked_step(steps: &mut Vec<u16>, step: u16) {
    if let Some(position) = steps.iter().position(|candidate| *candidate == step) {
        steps.remove(position);
    }
}

pub(super) fn insert_tracked_jump_edge(
    edges: &mut Vec<(u16, u16)>,
    edge: (u16, u16),
    limit: usize,
) -> Result<bool, BudgetTraversalError> {
    if edges.iter().copied().any(|candidate| candidate == edge) {
        return Ok(false);
    }
    if edges.len() >= limit {
        return Err(BudgetTraversalError::StepCountOverflow { actual: u64::MAX });
    }
    edges.push(edge);
    Ok(true)
}
