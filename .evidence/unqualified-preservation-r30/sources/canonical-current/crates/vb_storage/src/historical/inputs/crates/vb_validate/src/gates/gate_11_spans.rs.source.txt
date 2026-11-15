#![forbid(unsafe_code)]

use crate::{ValidationError, ValidationResult};
use vb_core::ids::StepIdx;
use vb_core::workflow::{CompiledNodeKind, WorkflowParts};

pub(crate) fn has_prior_matching_start(
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

pub(crate) fn step_in_loop_body(index: usize, body: StepIdx, done: StepIdx) -> bool {
    let body_index = body.as_usize();
    let done_index = done.as_usize();
    index >= body_index && index < done_index
}

pub(crate) fn require_pairing(
    matches: bool,
    index: usize,
    detail: impl Into<String>,
) -> ValidationResult<()> {
    if matches {
        return Ok(());
    }
    Err(ValidationError::NodeKindConstraintViolation {
        node_index: index,
        detail: detail.into(),
    })
}

pub(crate) fn is_matching_for_each_start(
    kind: &CompiledNodeKind,
    body: StepIdx,
    done: StepIdx,
) -> bool {
    matches!(
        kind,
        CompiledNodeKind::ForEachStart {
            body: start_body,
            done: start_done,
            ..
        } if *start_body == body && *start_done == done
    )
}

pub(crate) fn is_matching_collect_start(
    kind: &CompiledNodeKind,
    body: StepIdx,
    done: StepIdx,
) -> bool {
    matches!(
        kind,
        CompiledNodeKind::CollectStart {
            body: start_body,
            done: start_done,
            ..
        } if *start_body == body && *start_done == done
    )
}

pub(crate) fn is_matching_reduce_start(
    kind: &CompiledNodeKind,
    body: StepIdx,
    done: StepIdx,
) -> bool {
    matches!(
        kind,
        CompiledNodeKind::ReduceStart {
            body: start_body,
            done: start_done,
            ..
        } if *start_body == body && *start_done == done
    )
}

pub(crate) fn is_matching_repeat_start(
    kind: &CompiledNodeKind,
    body: StepIdx,
    done: StepIdx,
) -> bool {
    matches!(
        kind,
        CompiledNodeKind::RepeatStart {
            body: start_body,
            done: start_done,
            ..
        } if *start_body == body && *start_done == done
    )
}

pub(crate) fn is_foreach_start_done(kind: &CompiledNodeKind, index: usize) -> bool {
    matches!(
        kind,
        CompiledNodeKind::ForEachStart { done, .. } if done.as_usize() == index
    )
}

pub(crate) fn is_collect_start_done(kind: &CompiledNodeKind, index: usize) -> bool {
    matches!(
        kind,
        CompiledNodeKind::CollectStart { done, .. } if done.as_usize() == index
    )
}

pub(crate) fn is_reduce_start_done(kind: &CompiledNodeKind, index: usize) -> bool {
    matches!(
        kind,
        CompiledNodeKind::ReduceStart { done, .. } if done.as_usize() == index
    )
}

pub(crate) fn is_repeat_start_done(kind: &CompiledNodeKind, index: usize) -> bool {
    matches!(
        kind,
        CompiledNodeKind::RepeatStart { done, .. } if done.as_usize() == index
    )
}

pub(crate) fn check_step_in_range(
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

pub(crate) fn check_next_step_in_range(
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

/// Checks that the loop body start is after the loop start and before the loop
/// done step, ensuring the loop span is well-formed.
pub(crate) fn check_loop_span(
    start_index: usize,
    body: StepIdx,
    done: StepIdx,
    node_count: usize,
) -> ValidationResult<()> {
    let body_usize = body.as_usize();
    let done_usize = done.as_usize();
    // Body must be after start (forward edge).
    if body_usize <= start_index {
        return Err(ValidationError::LoopBodyStepOutOfRange {
            step: body_usize,
            node_count,
            source_node: start_index,
            label: "loop body must be after loop start".to_owned(),
        });
    }
    // Done must be after body.
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

pub(crate) fn check_together_span(
    start_index: usize,
    branches: &[StepIdx],
    join: StepIdx,
    node_count: usize,
) -> ValidationResult<()> {
    let join_usize = join.as_usize();
    for (branch_index, branch) in branches.iter().enumerate() {
        let branch_usize = branch.as_usize();
        // Branch must be after start.
        if branch_usize <= start_index {
            return Err(ValidationError::LoopBodyStepOutOfRange {
                step: branch_usize,
                node_count,
                source_node: start_index,
                label: format!("together branch {branch_index} must be after start"),
            });
        }
        // Join must be after branch.
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
