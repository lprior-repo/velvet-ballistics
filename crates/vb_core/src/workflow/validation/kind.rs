//! Node kind-specific validation.

use super::helpers::{
    validate_branch_route, validate_build_list, validate_build_object, validate_const,
    validate_expr, validate_expr_choose, validate_for_each_start, validate_nonzero_u16,
    validate_optional_slot, validate_optional_step, validate_reduce_next, validate_reduce_start,
    validate_repeat_start, validate_slot, validate_slot_and_steps, validate_slots, validate_step,
    validate_together, validate_two_steps, validate_slot_choose,
};
use super::super::types::{WorkflowError, WorkflowParts};

/// Validates a single node against its kind-specific constraints.
#[allow(clippy::match_same_arms)]
pub(crate) fn validate_node(node: &super::super::node::CompiledNode, parts: &WorkflowParts) -> Result<(), WorkflowError> {
    validate_optional_slot(node.output, parts.slot_count)?;
    validate_optional_step(node.next, parts.nodes.len())?;
    match &node.kind {
        super::super::node::CompiledNodeKind::Nop => Ok(()),
        super::super::node::CompiledNodeKind::SetConst { value } => validate_const(*value, parts.constants.len()),
        super::super::node::CompiledNodeKind::Copy { source } => validate_slot(*source, parts.slot_count),
        super::super::node::CompiledNodeKind::EvalExpr { expr } => validate_expr(*expr, parts.expressions.len()),
        super::super::node::CompiledNodeKind::BuildObject { fields } => validate_build_object(fields, parts),
        super::super::node::CompiledNodeKind::BuildList { items } => validate_build_list(items, parts.slot_count),
        super::super::node::CompiledNodeKind::Do { action: _, input } => validate_slot(*input, parts.slot_count),
        super::super::node::CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => validate_slot_choose(branches, *otherwise, parts),
        super::super::node::CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => validate_expr_choose(branches, *otherwise, parts),
        super::super::node::CompiledNodeKind::ForEachStart {
            input,
            item_slot,
            limit: _,
            body,
            done,
        } => validate_for_each_start(*input, *item_slot, *body, *done, parts),
        super::super::node::CompiledNodeKind::ForEachNext {
            iterator_slot,
            body,
            done,
        } => validate_slot_and_steps(*iterator_slot, *body, *done, parts),
        super::super::node::CompiledNodeKind::ForEachJoin { output } => validate_slot(*output, parts.slot_count),
        super::super::node::CompiledNodeKind::TogetherStart { branches, join } => validate_together(branches, *join, parts),
        super::super::node::CompiledNodeKind::TogetherBranch {
            branch: _,
            entry,
            join,
            accumulator,
        } => {
            validate_two_steps(*entry, *join, parts)?;
            validate_slot(*accumulator, parts.slot_count)
        }
        super::super::node::CompiledNodeKind::TogetherJoin {
            branch_count,
            accumulator,
        } => {
            validate_nonzero_u16(*branch_count, "branch_count")?;
            validate_slot(*accumulator, parts.slot_count)
        }
        super::super::node::CompiledNodeKind::CollectStart {
            source,
            limit: _,
            page_size: _,
            body,
            done,
        } => validate_slot_and_steps(*source, *body, *done, parts),
        super::super::node::CompiledNodeKind::CollectPage {
            collector_slot,
            body,
            done,
        }
        | super::super::node::CompiledNodeKind::CollectNext {
            collector_slot,
            body,
            done,
        } => validate_slot_and_steps(*collector_slot, *body, *done, parts),
        super::super::node::CompiledNodeKind::CollectFinish { collector_slot } => {
            validate_slot(*collector_slot, parts.slot_count)
        }
        super::super::node::CompiledNodeKind::ReduceStart {
            input,
            accumulator,
            initial,
            body,
            done,
        } => validate_reduce_start(*input, *accumulator, *initial, *body, *done, parts),
        super::super::node::CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator,
            body,
            done,
        } => validate_reduce_next(*iterator_slot, *accumulator, *body, *done, parts),
        super::super::node::CompiledNodeKind::ReduceFinish { accumulator } => {
            validate_slot(*accumulator, parts.slot_count)
        }
        super::super::node::CompiledNodeKind::RepeatStart {
            max_attempts,
            body,
            done,
        } => validate_repeat_start(*max_attempts, *body, *done, parts),
        super::super::node::CompiledNodeKind::RepeatAttempt {
            attempt_slot,
            body,
            done,
        } => validate_slot_and_steps(*attempt_slot, *body, *done, parts),
        super::super::node::CompiledNodeKind::RepeatCheck { attempt_slot, done } => {
            validate_slot(*attempt_slot, parts.slot_count)?;
            validate_step(*done, parts.nodes.len())
        }
        super::super::node::CompiledNodeKind::RepeatFinish { result } => validate_slot(*result, parts.slot_count),
        super::super::node::CompiledNodeKind::WaitUntil { deadline_slot } => validate_slot(*deadline_slot, parts.slot_count),
        super::super::node::CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => {
            validate_slot(*event, parts.slot_count)?;
            validate_optional_slot(*timeout_slot, parts.slot_count)
        }
        super::super::node::CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => {
            validate_slot(*prompt, parts.slot_count)?;
            validate_optional_slot(*timeout_slot, parts.slot_count)
        }
        super::super::node::CompiledNodeKind::AskResume { answer } => validate_slot(*answer, parts.slot_count),
        super::super::node::CompiledNodeKind::RetryCheck {
            policy_slot,
            body,
            exhausted,
        } => validate_slot_and_steps(*policy_slot, *body, *exhausted, parts),
        super::super::node::CompiledNodeKind::ErrorHandler { body, handler } => validate_two_steps(*body, *handler, parts),
        super::super::node::CompiledNodeKind::Jump { target } => validate_step(*target, parts.nodes.len()),
        super::super::node::CompiledNodeKind::Finish { result } => validate_slot(*result, parts.slot_count),
    }
}
