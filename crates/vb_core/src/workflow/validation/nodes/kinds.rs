#![forbid(unsafe_code)]
//! Exhaustive kind dispatch for [`CompiledNodeKind`] validation.
//!
//! Each arm calls the appropriate shared validator from `common` and
//! `branch_tables` modules.

use super::bounds::{
    validate_const, validate_expr, validate_optional_slot, validate_slot, validate_step,
};
use super::common::{
    validate_build_list, validate_build_object, validate_collect_start, validate_expr_choose,
    validate_for_each_start, validate_node_common, validate_reduce_next, validate_reduce_start,
    validate_repeat_start, validate_slot_and_steps, validate_slot_choose, validate_together,
    validate_together_join, validate_two_steps,
};
use crate::workflow::{CompiledNode, CompiledNodeKind, WorkflowError, WorkflowParts};

/// Validates a single node: common fields first, then kind-specific fields.
pub(crate) fn validate_node(
    node: &CompiledNode,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_node_common(node, parts)?;
    validate_node_kind(&node.kind, parts)
}

/// Dispatches kind-specific validation for each [`CompiledNodeKind`] variant.
pub(crate) fn validate_node_kind(
    kind: &CompiledNodeKind,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    match kind {
        CompiledNodeKind::Nop => Ok(()),
        CompiledNodeKind::SetConst { value } => validate_const(*value, parts.constants.len()),
        CompiledNodeKind::Copy { source } => validate_slot(*source, parts.slot_count),
        CompiledNodeKind::EvalExpr { expr } => validate_expr(*expr, parts.expressions.len()),
        CompiledNodeKind::BuildObject { fields } => validate_build_object(fields, parts),
        CompiledNodeKind::BuildList { items } => validate_build_list(items, parts.slot_count),
        CompiledNodeKind::Do { action: _, input } => validate_slot(*input, parts.slot_count),
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => validate_slot_choose(branches, *otherwise, parts),
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => validate_expr_choose(branches, *otherwise, parts),
        CompiledNodeKind::ForEachStart {
            input,
            item_slot,
            limit,
            body,
            done,
        } => validate_for_each_start(*input, *item_slot, *limit, *body, *done, parts),
        CompiledNodeKind::ForEachNext {
            iterator_slot,
            body,
            done,
        } => validate_slot_and_steps(*iterator_slot, *body, *done, parts),
        CompiledNodeKind::ForEachJoin { output } => validate_slot(*output, parts.slot_count),
        CompiledNodeKind::TogetherStart { branches, join } => {
            validate_together(branches, *join, parts)
        }
        CompiledNodeKind::TogetherBranch {
            branch: _,
            entry,
            join,
            accumulator,
        } => {
            validate_two_steps(*entry, *join, parts)?;
            validate_slot(*accumulator, parts.slot_count)
        }
        CompiledNodeKind::TogetherJoin {
            branch_count,
            accumulator,
        } => validate_together_join(*branch_count, *accumulator, parts),
        CompiledNodeKind::CollectStart {
            source,
            limit,
            page_size,
            body,
            done,
        } => validate_collect_start(*source, *limit, *page_size, *body, *done, parts),
        CompiledNodeKind::CollectPage {
            collector_slot,
            body,
            done,
        }
        | CompiledNodeKind::CollectNext {
            collector_slot,
            body,
            done,
        } => validate_slot_and_steps(*collector_slot, *body, *done, parts),
        CompiledNodeKind::CollectFinish { collector_slot } => {
            validate_slot(*collector_slot, parts.slot_count)
        }
        CompiledNodeKind::ReduceStart {
            input,
            accumulator,
            initial,
            body,
            done,
        } => validate_reduce_start(*input, *accumulator, *initial, *body, *done, parts),
        CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator,
            body,
            done,
        } => validate_reduce_next(*iterator_slot, *accumulator, *body, *done, parts),
        CompiledNodeKind::ReduceFinish { accumulator } => {
            validate_slot(*accumulator, parts.slot_count)
        }
        CompiledNodeKind::RepeatStart {
            max_attempts,
            body,
            done,
        } => validate_repeat_start(*max_attempts, *body, *done, parts),
        CompiledNodeKind::RepeatAttempt {
            attempt_slot,
            body,
            done,
        } => validate_slot_and_steps(*attempt_slot, *body, *done, parts),
        CompiledNodeKind::RepeatCheck { attempt_slot, done } => {
            validate_slot(*attempt_slot, parts.slot_count)?;
            validate_step(*done, parts.nodes.len())
        }
        CompiledNodeKind::RepeatFinish { result } => validate_slot(*result, parts.slot_count),
        CompiledNodeKind::WaitUntil { deadline_slot } => {
            validate_slot(*deadline_slot, parts.slot_count)
        }
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => {
            validate_slot(*event, parts.slot_count)?;
            validate_optional_slot(*timeout_slot, parts.slot_count)
        }
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => {
            validate_slot(*prompt, parts.slot_count)?;
            validate_optional_slot(*timeout_slot, parts.slot_count)
        }
        CompiledNodeKind::AskResume { answer } => validate_slot(*answer, parts.slot_count),
        CompiledNodeKind::RetryCheck {
            policy_slot,
            body,
            exhausted,
        } => validate_slot_and_steps(*policy_slot, *body, *exhausted, parts),
        CompiledNodeKind::ErrorHandler {
            body,
            handler,
            error_slot,
        } => {
            validate_two_steps(*body, *handler, parts)?;
            validate_optional_slot(*error_slot, parts.slot_count)
        }
        CompiledNodeKind::Jump { target } => validate_step(*target, parts.nodes.len()),
        CompiledNodeKind::Finish { result } => validate_slot(*result, parts.slot_count),
    }
}
