#![forbid(unsafe_code)]
//! Gate 11: Loop body graph validation

use crate::{ValidationError, ValidationResult};
use vb_core::ids::{AccessorIdx, ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId};
use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, PathSegment,
    WorkflowParts,
};

pub fn validate_gate_11_loop_body_graph(parts: &WorkflowParts) -> ValidationResult<()> {
    let node_count = parts.nodes.len();
    if node_count == 0 {
        return Ok(());
    }
    check_step_in_range(parts.entry, node_count, 0, "entry")?;
    for (index, node) in parts.nodes.iter().enumerate() {
        if let Some(next) = node.next {
            check_next_step_in_range(next, node_count, index)?;
        }
        if let Some(on_error) = node.on_error {
            check_step_in_range(on_error, node_count, index, "on_error")?;
        }
        match &node.kind {
            CompiledNodeKind::ForEachStart { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "for_each body")?;
                check_step_in_range(*done, node_count, index, "for_each done")?;
                check_loop_span(index, *body, *done, node_count)?;
            }
            CompiledNodeKind::ForEachNext { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "for_each_next body")?;
                check_step_in_range(*done, node_count, index, "for_each_next done")?;
            }
            CompiledNodeKind::TogetherStart { branches, join } => {
                for (branch_index, branch) in branches.iter().enumerate() {
                    check_step_in_range(
                        *branch,
                        node_count,
                        index,
                        &format!("together branch {branch_index}"),
                    )?;
                }
                check_step_in_range(*join, node_count, index, "together join")?;
                check_together_span(index, branches, *join, node_count)?;
            }
            CompiledNodeKind::TogetherBranch { entry, join, .. } => {
                check_step_in_range(*entry, node_count, index, "together_branch entry")?;
                check_step_in_range(*join, node_count, index, "together_branch join")?;
            }
            CompiledNodeKind::TogetherJoin { .. } => {}
            CompiledNodeKind::CollectStart { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "collect body")?;
                check_step_in_range(*done, node_count, index, "collect done")?;
                check_loop_span(index, *body, *done, node_count)?;
            }
            CompiledNodeKind::CollectPage { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "collect_page body")?;
                check_step_in_range(*done, node_count, index, "collect_page done")?;
            }
            CompiledNodeKind::CollectNext { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "collect_next body")?;
                check_step_in_range(*done, node_count, index, "collect_next done")?;
            }
            CompiledNodeKind::ReduceStart { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "reduce body")?;
                check_step_in_range(*done, node_count, index, "reduce done")?;
                check_loop_span(index, *body, *done, node_count)?;
            }
            CompiledNodeKind::ReduceNext { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "reduce_next body")?;
                check_step_in_range(*done, node_count, index, "reduce_next done")?;
            }
            CompiledNodeKind::RepeatStart { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "repeat body")?;
                check_step_in_range(*done, node_count, index, "repeat done")?;
                check_loop_span(index, *body, *done, node_count)?;
            }
            CompiledNodeKind::RepeatAttempt { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "repeat_attempt body")?;
                check_step_in_range(*done, node_count, index, "repeat_attempt done")?;
            }
            CompiledNodeKind::RepeatCheck { done, .. } => {
                check_step_in_range(*done, node_count, index, "repeat_check done")?;
            }
            CompiledNodeKind::RetryCheck {
                body, exhausted, ..
            } => {
                check_step_in_range(*body, node_count, index, "retry_check body")?;
                check_step_in_range(*exhausted, node_count, index, "retry_check exhausted")?;
            }
            CompiledNodeKind::ErrorHandler { body, handler, .. } => {
                check_step_in_range(*body, node_count, index, "error_handler body")?;
                check_step_in_range(*handler, node_count, index, "error_handler handler")?;
            }
            _ => {}
        }
    }
    validate_loop_pairings(parts)?;
    Ok(())
}

fn validate_loop_pairings(parts: &WorkflowParts) -> ValidationResult<()> {
    parts
        .nodes
        .iter()
        .enumerate()
        .try_for_each(|(index, node)| validate_node_pairing(parts, index, &node.kind))
}

fn validate_node_pairing(
    parts: &WorkflowParts,
    index: usize,
    kind: &CompiledNodeKind,
) -> ValidationResult<()> {
    match kind {
        CompiledNodeKind::ForEachNext { body, done, .. } => require_matching_body_start(
            parts,
            index,
            *body,
            *done,
            "ForEachNext",
            is_matching_for_each_start,
        ),
        CompiledNodeKind::ForEachJoin { .. } => {
            require_matching_done_start(parts, index, "ForEachJoin", is_foreach_start_done)
        }
        CompiledNodeKind::TogetherBranch { branch, join, .. } => {
            require_matching_together_branch(parts, index, *branch, *join)
        }
        CompiledNodeKind::TogetherJoin { branch_count, .. } => {
            require_matching_together_join(parts, index, *branch_count)
        }
        CompiledNodeKind::CollectPage { body, done, .. }
        | CompiledNodeKind::CollectNext { body, done, .. } => require_matching_body_start(
            parts,
            index,
            *body,
            *done,
            "Collect continuation",
            is_matching_collect_start,
        ),
        CompiledNodeKind::CollectFinish { .. } => {
            require_matching_done_start(parts, index, "CollectFinish", is_collect_start_done)
        }
        CompiledNodeKind::ReduceNext { body, done, .. } => require_matching_body_start(
            parts,
            index,
            *body,
            *done,
            "ReduceNext",
            is_matching_reduce_start,
        ),
        CompiledNodeKind::ReduceFinish { .. } => {
            require_matching_done_start(parts, index, "ReduceFinish", is_reduce_start_done)
        }
        CompiledNodeKind::RepeatAttempt { body, done, .. } => require_matching_body_start(
            parts,
            index,
            *body,
            *done,
            "RepeatAttempt",
            is_matching_repeat_start,
        ),
        CompiledNodeKind::RepeatCheck { done, .. } => {
            require_matching_repeat_check(parts, index, *done)
        }
        CompiledNodeKind::RepeatFinish { .. } => {
            require_matching_done_start(parts, index, "RepeatFinish", is_repeat_start_done)
        }
        _ => Ok(()),
    }
}

fn require_matching_body_start(
    parts: &WorkflowParts,
    index: usize,
    body: StepIdx,
    done: StepIdx,
    label: &str,
    start_matches: fn(&CompiledNodeKind, StepIdx, StepIdx) -> bool,
) -> ValidationResult<()> {
    let has_match = step_in_loop_body(index, body, done)
        && has_prior_matching_start(parts, index, |kind| start_matches(kind, body, done));
    require_pairing(has_match, index, format!("{label} has no matching start"))
}

fn require_matching_done_start(
    parts: &WorkflowParts,
    index: usize,
    label: &str,
    start_done_matches: fn(&CompiledNodeKind, usize) -> bool,
) -> ValidationResult<()> {
    let has_match = has_prior_matching_start(parts, index, |kind| start_done_matches(kind, index));
    require_pairing(has_match, index, format!("{label} has no matching start"))
}

fn require_matching_repeat_check(
    parts: &WorkflowParts,
    index: usize,
    done: StepIdx,
) -> ValidationResult<()> {
    let has_match = has_prior_matching_start(parts, index, |kind| match kind {
        CompiledNodeKind::RepeatStart {
            body,
            done: start_done,
            ..
        } => *start_done == done && step_in_loop_body(index, *body, *start_done),
        _ => false,
    });
    require_pairing(has_match, index, "RepeatCheck has no matching RepeatStart")
}

fn require_matching_together_branch(
    parts: &WorkflowParts,
    index: usize,
    branch: u16,
    join: StepIdx,
) -> ValidationResult<()> {
    let has_match = has_prior_matching_start(parts, index, |kind| match kind {
        CompiledNodeKind::TogetherStart {
            branches,
            join: start_join,
        } => {
            *start_join == join
                && branches.iter().enumerate().any(|(branch_index, target)| {
                    branch_index == usize::from(branch) && target.as_usize() == index
                })
        }
        _ => false,
    });
    require_pairing(
        has_match,
        index,
        "TogetherBranch has no matching TogetherStart branch target",
    )
}

fn require_matching_together_join(
    parts: &WorkflowParts,
    index: usize,
    branch_count: u16,
) -> ValidationResult<()> {
    let has_match = has_prior_matching_start(parts, index, |kind| match kind {
        CompiledNodeKind::TogetherStart { branches, join } => {
            join.as_usize() == index && branches.len() == usize::from(branch_count)
        }
        _ => false,
    });
    require_pairing(
        has_match,
        index,
        "TogetherJoin has no matching TogetherStart branch count",
    )
}

fn has_prior_matching_start(
    parts: &WorkflowParts,
    index: usize,
    predicate: impl Fn(&CompiledNodeKind) -> bool,
) -> bool {
    parts
        .nodes
        .iter()
        .take(index)
        .any(|node| predicate(&node.kind))
}

fn step_in_loop_body(index: usize, body: StepIdx, done: StepIdx) -> bool {
    let body_index = body.as_usize();
    let done_index = done.as_usize();
    index >= body_index && index < done_index
}

fn require_pairing(matches: bool, index: usize, detail: impl Into<String>) -> ValidationResult<()> {
    if matches {
        return Ok(());
    }
    Err(ValidationError::NodeKindConstraintViolation {
        node_index: index,
        detail: detail.into(),
    })
}

fn is_matching_for_each_start(kind: &CompiledNodeKind, body: StepIdx, done: StepIdx) -> bool {
    matches!(
        kind,
        CompiledNodeKind::ForEachStart {
            body: start_body,
            done: start_done,
            ..
        } if *start_body == body && *start_done == done
    )
}

fn is_matching_collect_start(kind: &CompiledNodeKind, body: StepIdx, done: StepIdx) -> bool {
    matches!(
        kind,
        CompiledNodeKind::CollectStart {
            body: start_body,
            done: start_done,
            ..
        } if *start_body == body && *start_done == done
    )
}

fn is_matching_reduce_start(kind: &CompiledNodeKind, body: StepIdx, done: StepIdx) -> bool {
    matches!(
        kind,
        CompiledNodeKind::ReduceStart {
            body: start_body,
            done: start_done,
            ..
        } if *start_body == body && *start_done == done
    )
}

fn is_matching_repeat_start(kind: &CompiledNodeKind, body: StepIdx, done: StepIdx) -> bool {
    matches!(
        kind,
        CompiledNodeKind::RepeatStart {
            body: start_body,
            done: start_done,
            ..
        } if *start_body == body && *start_done == done
    )
}

fn is_foreach_start_done(kind: &CompiledNodeKind, index: usize) -> bool {
    matches!(
        kind,
        CompiledNodeKind::ForEachStart { done, .. } if done.as_usize() == index
    )
}

fn is_collect_start_done(kind: &CompiledNodeKind, index: usize) -> bool {
    matches!(
        kind,
        CompiledNodeKind::CollectStart { done, .. } if done.as_usize() == index
    )
}

fn is_reduce_start_done(kind: &CompiledNodeKind, index: usize) -> bool {
    matches!(
        kind,
        CompiledNodeKind::ReduceStart { done, .. } if done.as_usize() == index
    )
}

fn is_repeat_start_done(kind: &CompiledNodeKind, index: usize) -> bool {
    matches!(
        kind,
        CompiledNodeKind::RepeatStart { done, .. } if done.as_usize() == index
    )
}

fn check_step_in_range(
    step: StepIdx,
    node_count: usize,
    source_index: usize,
    label: &str,
) -> ValidationResult<()> {
    if step.as_usize() >= node_count {
        return Err(ValidationError::LoopBodyStepOutOfRange {
            step: step.as_usize(),
            node_count,
            source_node: source_index,
            label: label.to_owned(),
        });
    }
    Ok(())
}

fn check_next_step_in_range(
    step: StepIdx,
    node_count: usize,
    source_index: usize,
) -> ValidationResult<()> {
    if step.as_usize() >= node_count {
        return Err(ValidationError::LoopBodyStepOutOfRange {
            step: step.as_usize(),
            node_count,
            source_node: source_index,
            label: "next".to_owned(),
        });
    }
    Ok(())
}

fn check_loop_span(
    start_index: usize,
    body: StepIdx,
    done: StepIdx,
    node_count: usize,
) -> ValidationResult<()> {
    let body_usize = body.as_usize();
    let done_usize = done.as_usize();
    if body_usize <= start_index {
        return Err(ValidationError::LoopBodyStepOutOfRange {
            step: body_usize,
            node_count,
            source_node: start_index,
            label: "loop body must be after loop start".to_owned(),
        });
    }
    if done_usize <= body_usize {
        return Err(ValidationError::LoopBodyStepOutOfRange {
            step: done_usize,
            node_count,
            source_node: start_index,
            label: "loop done must be after loop body".to_owned(),
        });
    }
    Ok(())
}

fn check_together_span(
    start_index: usize,
    branches: &[StepIdx],
    join: StepIdx,
    node_count: usize,
) -> ValidationResult<()> {
    let join_usize = join.as_usize();
    for (branch_index, branch) in branches.iter().enumerate() {
        let branch_usize = branch.as_usize();
        if branch_usize <= start_index {
            return Err(ValidationError::LoopBodyStepOutOfRange {
                step: branch_usize,
                node_count,
                source_node: start_index,
                label: format!("together branch {branch_index} must be after start"),
            });
        }
        if join_usize <= branch_usize {
            return Err(ValidationError::LoopBodyStepOutOfRange {
                step: join_usize,
                node_count,
                source_node: start_index,
                label: format!("together join must be after branch {branch_index}"),
            });
        }
    }
    Ok(())
}
