#![forbid(unsafe_code)]

use super::gate_11_spans::{
    check_loop_span, check_next_step_in_range, check_step_in_range, check_together_span,
    has_prior_matching_start, is_collect_start_done, is_foreach_start_done,
    is_matching_collect_start, is_matching_for_each_start, is_matching_reduce_start,
    is_matching_repeat_start, is_reduce_start_done, is_repeat_start_done, require_pairing,
    step_in_loop_body,
};
use crate::ValidationResult;
use vb_core::ids::StepIdx;
use vb_core::workflow::{CompiledNodeKind, WorkflowParts};

// ---------------------------------------------------------------------------
// Gate 11: ForEach/Together body graph is well-formed
// ---------------------------------------------------------------------------

/// Validates that ForEach and Together body subgraphs are structurally
/// well-formed: body targets exist, done targets exist, inner loops nest
/// correctly, and no dangling references.
///
/// Gate 11 (idempotency / structural): loop bodies must point to valid steps
/// within the node array, done targets must be valid, and inner loop spans
/// must be contained within their outer loop spans.
pub fn validate_gate_11_loop_body_graph(parts: &WorkflowParts) -> ValidationResult<()> {
    let node_count = parts.nodes.len();
    if node_count == 0 {
        return Ok(());
    }
    check_step_in_range(parts.entry, node_count, 0, "entry")?;
    for (index, node) in parts.nodes.iter().enumerate() {
        if let Some(next) = node.routing.next.step() {
            check_next_step_in_range(next, node_count, index)?;
        }
        if let Some(on_error) = node.routing.error.handler() {
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
            require_matching_together_branch(parts, index, branch.get(), *join)
        }
        CompiledNodeKind::TogetherJoin { branch_count, .. } => {
            require_matching_together_join(parts, index, branch_count.get())
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
