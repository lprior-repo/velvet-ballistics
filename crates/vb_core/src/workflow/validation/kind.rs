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

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::node::{CompiledNode, CompiledNodeKind};
    use super::super::super::types::{WorkflowParts, ResourceContract, WorkflowError};
    use crate::ids::{ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
    use crate::value::ConstValue;

    fn make_parts(
        nodes: Vec<CompiledNode>,
        slot_count: u16,
    ) -> WorkflowParts {
        let max_steps = u16::try_from(nodes.len()).map_or(u16::MAX, |v| v);
        WorkflowParts {
            name: Box::<str>::from("kind_test"),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count,
            symbols_count: 10,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract {
                max_steps,
                max_slots: slot_count.max(1),
                max_constants: 10,
                max_accessors: 10,
                max_expressions: 10,
                max_expr_stack: 64,
                max_step_budget_per_tick: 10_000,
                max_input_bytes: 1_048_576,
                max_output_bytes: 1_048_576,
                max_blob_bytes: 16_777_216,
                max_ipc_payload_bytes: 1_048_576,
                max_retry_attempts: 3,
                max_fanout: 64,
                max_collect_items: 1_024,
                max_queue_depth: 1_024,
                max_journal_batch_bytes: 1_048_576,
            },
            step_names: Box::new([]),
        }
    }

    fn node(id: u16, kind: CompiledNodeKind) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind,
        }
    }

    // -- Nop --

    #[test]
    fn validate_node_nop_accepts_valid() {
        let parts = make_parts(vec![node(0, CompiledNodeKind::Nop)], 1);
        assert_eq!(validate_node(&parts.nodes[0], &parts), Ok(()));
    }

    // -- SetConst --

    #[test]
    fn validate_node_set_const_accepts_in_bounds() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::SetConst { value: ConstIdx::new(0) })],
            1,
        );
        assert_eq!(validate_node(&parts.nodes[0], &parts), Ok(()));
    }

    #[test]
    fn validate_node_set_const_rejects_out_of_bounds() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::SetConst { value: ConstIdx::new(5) })],
            1,
        );
        let result = validate_node(&parts.nodes[0], &parts);
        assert!(matches!(result, Err(WorkflowError::ConstOutOfBounds { constant }) if constant == ConstIdx::new(5)));
    }

    // -- Copy --

    #[test]
    fn validate_node_copy_accepts_in_bounds() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::Copy { source: SlotIdx::new(0) })],
            2,
        );
        assert_eq!(validate_node(&parts.nodes[0], &parts), Ok(()));
    }

    #[test]
    fn validate_node_copy_rejects_out_of_bounds() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::Copy { source: SlotIdx::new(5) })],
            2,
        );
        let result = validate_node(&parts.nodes[0], &parts);
        assert!(matches!(result, Err(WorkflowError::SlotOutOfBounds { .. })));
    }

    // -- EvalExpr --

    #[test]
    fn validate_node_eval_expr_rejects_out_of_bounds() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::EvalExpr { expr: ExprIdx::new(5) })],
            1,
        );
        let result = validate_node(&parts.nodes[0], &parts);
        assert!(matches!(result, Err(WorkflowError::Expression(crate::errors::CoreError::ExprOutOfBounds { .. }))));
    }

    // -- BuildObject --

    #[test]
    fn validate_node_build_object_accepts_in_bounds() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::BuildObject {
                fields: vec![(SymbolId::new(0), SlotIdx::new(0))].into_boxed_slice(),
            })],
            2,
        );
        assert_eq!(validate_node(&parts.nodes[0], &parts), Ok(()));
    }

    #[test]
    fn validate_node_build_object_rejects_slot_out_of_bounds() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::BuildObject {
                fields: vec![(SymbolId::new(0), SlotIdx::new(99))].into_boxed_slice(),
            })],
            2,
        );
        let result = validate_node(&parts.nodes[0], &parts);
        assert!(matches!(result, Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(99)));
    }

    // -- BuildList --

    #[test]
    fn validate_node_build_list_accepts_in_bounds() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::BuildList {
                items: vec![SlotIdx::new(0), SlotIdx::new(1)].into_boxed_slice(),
            })],
            3,
        );
        assert_eq!(validate_node(&parts.nodes[0], &parts), Ok(()));
    }

    #[test]
    fn validate_node_build_list_rejects_slot_out_of_bounds() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::BuildList {
                items: vec![SlotIdx::new(0), SlotIdx::new(50)].into_boxed_slice(),
            })],
            3,
        );
        let result = validate_node(&parts.nodes[0], &parts);
        assert!(matches!(result, Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(50)));
    }

    // -- Do --

    #[test]
    fn validate_node_do_accepts_in_bounds() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            })],
            2,
        );
        assert_eq!(validate_node(&parts.nodes[0], &parts), Ok(()));
    }

    #[test]
    fn validate_node_do_rejects_input_out_of_bounds() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(10),
            })],
            2,
        );
        let result = validate_node(&parts.nodes[0], &parts);
        assert!(matches!(result, Err(WorkflowError::SlotOutOfBounds { .. })));
    }

    // -- ForEachJoin --

    #[test]
    fn validate_node_for_each_join_accepts_in_bounds() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::ForEachJoin { output: SlotIdx::new(0) })],
            2,
        );
        assert_eq!(validate_node(&parts.nodes[0], &parts), Ok(()));
    }

    // -- TogetherJoin --

    #[test]
    fn validate_node_together_join_rejects_zero_branch_count() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::TogetherJoin {
                branch_count: 0,
                accumulator: SlotIdx::new(0),
            })],
            2,
        );
        let result = validate_node(&parts.nodes[0], &parts);
        assert!(matches!(
            result,
            Err(WorkflowError::ResourceContractExceeded { resource }) if resource == "branch_count"
        ));
    }

    // -- WaitEvent --

    #[test]
    fn validate_node_wait_event_accepts_with_optional_timeout() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(0),
                timeout_slot: Some(SlotIdx::new(1)),
            })],
            3,
        );
        assert_eq!(validate_node(&parts.nodes[0], &parts), Ok(()));
    }

    #[test]
    fn validate_node_wait_event_accepts_without_timeout() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(0),
                timeout_slot: None,
            })],
            3,
        );
        assert_eq!(validate_node(&parts.nodes[0], &parts), Ok(()));
    }

    #[test]
    fn validate_node_wait_event_rejects_event_out_of_bounds() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(10),
                timeout_slot: None,
            })],
            3,
        );
        let result = validate_node(&parts.nodes[0], &parts);
        assert!(matches!(result, Err(WorkflowError::SlotOutOfBounds { .. })));
    }

    // -- Ask --

    #[test]
    fn validate_node_ask_accepts_valid() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: None,
            })],
            2,
        );
        assert_eq!(validate_node(&parts.nodes[0], &parts), Ok(()));
    }

    #[test]
    fn validate_node_ask_rejects_prompt_out_of_bounds() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::Ask {
                prompt: SlotIdx::new(10),
                timeout_slot: None,
            })],
            2,
        );
        let result = validate_node(&parts.nodes[0], &parts);
        assert!(matches!(result, Err(WorkflowError::SlotOutOfBounds { .. })));
    }

    // -- AskResume --

    #[test]
    fn validate_node_ask_resume_rejects_out_of_bounds() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::AskResume { answer: SlotIdx::new(10) })],
            2,
        );
        let result = validate_node(&parts.nodes[0], &parts);
        assert!(matches!(result, Err(WorkflowError::SlotOutOfBounds { .. })));
    }

    // -- RepeatStart --

    #[test]
    fn validate_node_repeat_start_rejects_zero_attempts() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::RepeatStart {
                max_attempts: 0,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            })],
            2,
        );
        let result = validate_node(&parts.nodes[0], &parts);
        assert!(matches!(
            result,
            Err(WorkflowError::ResourceContractExceeded { resource }) if resource == "max_retry_attempts"
        ));
    }

    // -- RepeatFinish --

    #[test]
    fn validate_node_repeat_finish_accepts_in_bounds() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::RepeatFinish { result: SlotIdx::new(0) })],
            2,
        );
        assert_eq!(validate_node(&parts.nodes[0], &parts), Ok(()));
    }

    // -- WaitUntil --

    #[test]
    fn validate_node_wait_until_rejects_out_of_bounds() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::WaitUntil { deadline_slot: SlotIdx::new(99) })],
            2,
        );
        let result = validate_node(&parts.nodes[0], &parts);
        assert!(matches!(result, Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(99)));
    }

    // -- Jump --

    #[test]
    fn validate_node_jump_rejects_out_of_bounds() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::Jump { target: StepIdx::new(50) })],
            2,
        );
        let result = validate_node(&parts.nodes[0], &parts);
        assert!(matches!(result, Err(WorkflowError::StepOutOfBounds { step }) if step == StepIdx::new(50)));
    }

    // -- Finish --

    #[test]
    fn validate_node_finish_rejects_out_of_bounds() {
        let parts = make_parts(
            vec![node(0, CompiledNodeKind::Finish { result: SlotIdx::new(99) })],
            2,
        );
        let result = validate_node(&parts.nodes[0], &parts);
        assert!(matches!(result, Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(99)));
    }

    // -- Output slot validation (shared across all kinds) --

    #[test]
    fn validate_node_output_slot_out_of_bounds() {
        let mut n = node(0, CompiledNodeKind::Nop);
        n.output = Some(SlotIdx::new(99));
        let parts = make_parts(vec![n], 2);
        let result = validate_node(&parts.nodes[0], &parts);
        assert!(matches!(result, Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(99)));
    }

    // -- Next step validation (shared across all kinds) --

    #[test]
    fn validate_node_next_step_out_of_bounds() {
        let mut n = node(0, CompiledNodeKind::Nop);
        n.next = Some(StepIdx::new(50));
        let parts = make_parts(vec![n], 2);
        let result = validate_node(&parts.nodes[0], &parts);
        assert!(matches!(result, Err(WorkflowError::StepOutOfBounds { step }) if step == StepIdx::new(50)));
    }
}
