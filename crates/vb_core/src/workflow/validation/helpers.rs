//! Node validation helper functions.

use crate::errors::CoreError;
use crate::ids::{ConstIdx, ExprIdx, SlotIdx, StepIdx};

use super::super::types::{ExprBranch, SlotBranch, WorkflowError, WorkflowParts};

pub(crate) fn validate_slot(slot: SlotIdx, slot_count: u16) -> Result<(), WorkflowError> {
    if slot.as_usize() < usize::from(slot_count) {
        Ok(())
    } else {
        Err(WorkflowError::SlotOutOfBounds { slot })
    }
}

pub(crate) fn validate_optional_slot(slot: Option<SlotIdx>, slot_count: u16) -> Result<(), WorkflowError> {
    slot.map_or(Ok(()), |value| validate_slot(value, slot_count))
}

pub(crate) fn validate_slots(slots: &[SlotIdx], slot_count: u16) -> Result<(), WorkflowError> {
    for slot in slots {
        validate_slot(*slot, slot_count)?;
    }
    Ok(())
}

pub(crate) fn validate_build_list(items: &[SlotIdx], slot_count: u16) -> Result<(), WorkflowError> {
    if items.len() > crate::limits::MAX_LIST_ITEMS_PER_VALUE {
        return Err(WorkflowError::ResourceContractExceeded {
            resource: "list_items",
        });
    }
    validate_slots(items, slot_count)
}

pub(crate) fn validate_build_object(
    fields: &[(crate::ids::SymbolId, SlotIdx)],
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    if fields.len() > crate::limits::MAX_OBJECT_FIELDS_PER_VALUE {
        return Err(WorkflowError::ResourceContractExceeded {
            resource: "object_fields",
        });
    }
    for (_, slot) in fields {
        validate_slot(*slot, parts.slot_count)?;
    }
    Ok(())
}

pub(crate) fn validate_for_each_start(
    input: SlotIdx,
    item_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_slot(input, parts.slot_count)?;
    validate_slot(item_slot, parts.slot_count)?;
    validate_two_steps(body, done, parts)
}

pub(crate) fn validate_slot_and_steps(
    slot: SlotIdx,
    first: StepIdx,
    second: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_slot(slot, parts.slot_count)?;
    validate_two_steps(first, second, parts)
}

pub(crate) fn validate_two_steps(
    first: StepIdx,
    second: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_step(first, parts.nodes.len())?;
    validate_step(second, parts.nodes.len())
}

pub(crate) fn validate_together(
    branches: &[StepIdx],
    join: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_branch_route(branches.len(), Some(join))?;
    for branch in branches {
        validate_step(*branch, parts.nodes.len())?;
    }
    validate_step(join, parts.nodes.len())
}

pub(crate) fn validate_nonzero_u16(value: u16, resource: &'static str) -> Result<(), WorkflowError> {
    if value == 0 {
        Err(WorkflowError::ResourceContractExceeded { resource })
    } else {
        Ok(())
    }
}

pub(crate) fn validate_reduce_start(
    input: SlotIdx,
    accumulator: SlotIdx,
    initial: ConstIdx,
    body: StepIdx,
    done: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_slot(input, parts.slot_count)?;
    validate_slot(accumulator, parts.slot_count)?;
    validate_const(initial, parts.constants.len())?;
    validate_two_steps(body, done, parts)
}

pub(crate) fn validate_reduce_next(
    iterator_slot: SlotIdx,
    accumulator: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_slot(iterator_slot, parts.slot_count)?;
    validate_slot(accumulator, parts.slot_count)?;
    validate_two_steps(body, done, parts)
}

pub(crate) fn validate_repeat_start(
    max_attempts: u16,
    body: StepIdx,
    done: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_nonzero_u16(max_attempts, "max_retry_attempts")?;
    validate_two_steps(body, done, parts)
}

pub(crate) fn validate_slot_choose(
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_branch_route(branches.len(), otherwise)?;
    branches.iter().try_for_each(|branch| {
        validate_slot(branch.condition, parts.slot_count)?;
        validate_step(branch.target, parts.nodes.len())
    })?;
    validate_optional_step(otherwise, parts.nodes.len())
}

pub(crate) fn validate_expr_choose(
    branches: &[ExprBranch],
    otherwise: Option<StepIdx>,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_branch_route(branches.len(), otherwise)?;
    branches.iter().try_for_each(|branch| {
        validate_expr(branch.condition, parts.expressions.len())?;
        validate_step(branch.target, parts.nodes.len())
    })?;
    validate_optional_step(otherwise, parts.nodes.len())
}

pub(crate) fn validate_branch_route(
    branch_count: usize,
    otherwise: Option<StepIdx>,
) -> Result<(), WorkflowError> {
    if branch_count == 0 && otherwise.is_none() {
        Err(WorkflowError::EmptyBranchTable)
    } else {
        Ok(())
    }
}

pub(crate) fn validate_optional_step(step: Option<StepIdx>, node_count: usize) -> Result<(), WorkflowError> {
    step.map_or(Ok(()), |target| validate_step(target, node_count))
}

pub(crate) fn validate_expr(expr: ExprIdx, expression_count: usize) -> Result<(), WorkflowError> {
    if expr.as_usize() < expression_count {
        Ok(())
    } else {
        Err(WorkflowError::Expression(CoreError::ExprOutOfBounds {
            expr,
        }))
    }
}

pub(crate) fn validate_step(step: StepIdx, node_count: usize) -> Result<(), WorkflowError> {
    if step.as_usize() < node_count {
        Ok(())
    } else {
        Err(WorkflowError::StepOutOfBounds { step })
    }
}

pub(crate) fn validate_const(constant: ConstIdx, const_count: usize) -> Result<(), WorkflowError> {
    if constant.as_usize() < const_count {
        Ok(())
    } else {
        Err(WorkflowError::ConstOutOfBounds { constant })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ConstIdx, ExprIdx, SlotIdx, StepIdx};
    use crate::workflow::{ExprBranch, SlotBranch, WorkflowParts, CompiledNode, CompiledNodeKind, ResourceContract, WorkflowDigest};
    use crate::value::ConstValue;

    // -- validate_slot --

    #[test]
    fn validate_slot_accepts_in_bounds() {
        assert_eq!(validate_slot(SlotIdx::new(0), 1), Ok(()));
        assert_eq!(validate_slot(SlotIdx::new(4), 10), Ok(()));
    }

    #[test]
    fn validate_slot_rejects_at_boundary() {
        let result = validate_slot(SlotIdx::new(5), 5);
        assert!(matches!(result, Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(5)));
    }

    #[test]
    fn validate_slot_rejects_out_of_bounds() {
        let result = validate_slot(SlotIdx::new(100), 5);
        assert!(matches!(result, Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(100)));
    }

    #[test]
    fn validate_slot_accepts_zero_count_only_zero_index() {
        // slot 0 with slot_count=1 is in-bounds; slot 0 with slot_count=0 is not
        assert_eq!(validate_slot(SlotIdx::new(0), 1), Ok(()));
        let result = validate_slot(SlotIdx::new(0), 0);
        assert!(matches!(result, Err(WorkflowError::SlotOutOfBounds { .. })));
    }

    // -- validate_optional_slot --

    #[test]
    fn validate_optional_slot_accepts_none() {
        assert_eq!(validate_optional_slot(None, 5), Ok(()));
    }

    #[test]
    fn validate_optional_slot_accepts_some_in_bounds() {
        assert_eq!(validate_optional_slot(Some(SlotIdx::new(2)), 5), Ok(()));
    }

    #[test]
    fn validate_optional_slot_rejects_some_out_of_bounds() {
        let result = validate_optional_slot(Some(SlotIdx::new(10)), 5);
        assert!(matches!(result, Err(WorkflowError::SlotOutOfBounds { .. })));
    }

    // -- validate_step --

    #[test]
    fn validate_step_accepts_in_bounds() {
        assert_eq!(validate_step(StepIdx::new(0), 1), Ok(()));
        assert_eq!(validate_step(StepIdx::new(3), 10), Ok(()));
    }

    #[test]
    fn validate_step_rejects_at_boundary() {
        let result = validate_step(StepIdx::new(5), 5);
        assert!(matches!(result, Err(WorkflowError::StepOutOfBounds { step }) if step == StepIdx::new(5)));
    }

    #[test]
    fn validate_step_rejects_out_of_bounds() {
        let result = validate_step(StepIdx::new(99), 5);
        assert!(matches!(result, Err(WorkflowError::StepOutOfBounds { step }) if step == StepIdx::new(99)));
    }

    // -- validate_optional_step --

    #[test]
    fn validate_optional_step_accepts_none() {
        assert_eq!(validate_optional_step(None, 5), Ok(()));
    }

    #[test]
    fn validate_optional_step_accepts_some_in_bounds() {
        assert_eq!(validate_optional_step(Some(StepIdx::new(2)), 5), Ok(()));
    }

    #[test]
    fn validate_optional_step_rejects_some_out_of_bounds() {
        let result = validate_optional_step(Some(StepIdx::new(10)), 5);
        assert!(matches!(result, Err(WorkflowError::StepOutOfBounds { .. })));
    }

    // -- validate_expr --

    #[test]
    fn validate_expr_accepts_in_bounds() {
        assert_eq!(validate_expr(ExprIdx::new(0), 1), Ok(()));
        assert_eq!(validate_expr(ExprIdx::new(3), 10), Ok(()));
    }

    #[test]
    fn validate_expr_rejects_at_boundary() {
        let result = validate_expr(ExprIdx::new(5), 5);
        assert!(matches!(result, Err(WorkflowError::Expression(crate::errors::CoreError::ExprOutOfBounds { expr })) if expr == ExprIdx::new(5)));
    }

    // -- validate_const --

    #[test]
    fn validate_const_accepts_in_bounds() {
        assert_eq!(validate_const(ConstIdx::new(0), 1), Ok(()));
        assert_eq!(validate_const(ConstIdx::new(3), 10), Ok(()));
    }

    #[test]
    fn validate_const_rejects_at_boundary() {
        let result = validate_const(ConstIdx::new(5), 5);
        assert!(matches!(result, Err(WorkflowError::ConstOutOfBounds { constant }) if constant == ConstIdx::new(5)));
    }

    // -- validate_slots --

    #[test]
    fn validate_slots_accepts_empty() {
        assert_eq!(validate_slots(&[], 5), Ok(()));
    }

    #[test]
    fn validate_slots_accepts_all_in_bounds() {
        assert_eq!(
            validate_slots(&[SlotIdx::new(0), SlotIdx::new(2), SlotIdx::new(4)], 5),
            Ok(())
        );
    }

    #[test]
    fn validate_slots_rejects_first_out_of_bounds() {
        let result = validate_slots(&[SlotIdx::new(10), SlotIdx::new(0)], 5);
        assert!(matches!(result, Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(10)));
    }

    // -- validate_nonzero_u16 --

    #[test]
    fn validate_nonzero_u16_accepts_one() {
        assert_eq!(validate_nonzero_u16(1, "test_resource"), Ok(()));
    }

    #[test]
    fn validate_nonzero_u16_accepts_max() {
        assert_eq!(validate_nonzero_u16(u16::MAX, "test_resource"), Ok(()));
    }

    #[test]
    fn validate_nonzero_u16_rejects_zero() {
        let result = validate_nonzero_u16(0, "test_resource");
        assert!(matches!(result, Err(WorkflowError::ResourceContractExceeded { resource }) if resource == "test_resource"));
    }

    // -- validate_branch_route --

    #[test]
    fn validate_branch_route_accepts_branches_without_otherwise() {
        assert_eq!(validate_branch_route(1, None), Ok(()));
    }

    #[test]
    fn validate_branch_route_accepts_otherwise_without_branches() {
        assert_eq!(validate_branch_route(0, Some(StepIdx::new(0))), Ok(()));
    }

    #[test]
    fn validate_branch_route_accepts_both() {
        assert_eq!(validate_branch_route(2, Some(StepIdx::new(0))), Ok(()));
    }

    #[test]
    fn validate_branch_route_rejects_empty_no_otherwise() {
        let result = validate_branch_route(0, None);
        assert!(matches!(result, Err(WorkflowError::EmptyBranchTable)));
    }

    // -- validate_build_list --

    #[test]
    fn validate_build_list_accepts_empty() {
        assert_eq!(validate_build_list(&[], 5), Ok(()));
    }

    #[test]
    fn validate_build_list_accepts_in_bounds() {
        assert_eq!(
            validate_build_list(&[SlotIdx::new(0), SlotIdx::new(3)], 5),
            Ok(())
        );
    }

    #[test]
    fn validate_build_list_rejects_over_limit() {
        let items = vec![SlotIdx::new(0); crate::limits::MAX_LIST_ITEMS_PER_VALUE.saturating_add(1)];
        let result = validate_build_list(&items, 10);
        assert!(matches!(result, Err(WorkflowError::ResourceContractExceeded { resource }) if resource == "list_items"));
    }

    #[test]
    fn validate_build_list_rejects_slot_out_of_bounds() {
        let result = validate_build_list(&[SlotIdx::new(0), SlotIdx::new(99)], 5);
        assert!(matches!(result, Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(99)));
    }

    // -- validate_build_object --

    fn make_parts_with_slot_count(slot_count: u16) -> WorkflowParts {
        WorkflowParts {
            name: Box::<str>::from("test"),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: Box::new([]),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count,
            symbols_count: 100,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        }
    }

    #[test]
    fn validate_build_object_accepts_empty_fields() {
        let parts = make_parts_with_slot_count(5);
        assert_eq!(validate_build_object(&[], &parts), Ok(()));
    }

    #[test]
    fn validate_build_object_accepts_in_bounds() {
        let parts = make_parts_with_slot_count(5);
        let fields = vec![
            (crate::ids::SymbolId::new(0), SlotIdx::new(0)),
            (crate::ids::SymbolId::new(1), SlotIdx::new(4)),
        ];
        assert_eq!(validate_build_object(&fields, &parts), Ok(()));
    }

    #[test]
    fn validate_build_object_rejects_over_field_limit() {
        let parts = make_parts_with_slot_count(100);
        let fields: Vec<_> = (0..=crate::limits::MAX_OBJECT_FIELDS_PER_VALUE)
            .map(|i| (crate::ids::SymbolId::new(i as u32), SlotIdx::new(0)))
            .collect();
        let result = validate_build_object(&fields, &parts);
        assert!(matches!(result, Err(WorkflowError::ResourceContractExceeded { resource }) if resource == "object_fields"));
    }

    #[test]
    fn validate_build_object_rejects_slot_out_of_bounds() {
        let parts = make_parts_with_slot_count(2);
        let fields = vec![
            (crate::ids::SymbolId::new(0), SlotIdx::new(0)),
            (crate::ids::SymbolId::new(1), SlotIdx::new(5)),
        ];
        let result = validate_build_object(&fields, &parts);
        assert!(matches!(result, Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(5)));
    }

    // -- validate_together --

    fn make_parts_with_node_count(count: usize) -> WorkflowParts {
        let nodes: Vec<CompiledNode> = (0..count)
            .map(|i| CompiledNode {
                id: StepIdx::new(i as u16),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            })
            .collect();
        WorkflowParts {
            name: Box::<str>::from("test"),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: 0,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        }
    }

    #[test]
    fn validate_together_accepts_valid_branches() {
        let parts = make_parts_with_node_count(5);
        assert_eq!(
            validate_together(&[StepIdx::new(1), StepIdx::new(2)], StepIdx::new(4), &parts),
            Ok(())
        );
    }

    #[test]
    fn validate_together_rejects_branch_out_of_bounds() {
        let parts = make_parts_with_node_count(5);
        let result = validate_together(&[StepIdx::new(99)], StepIdx::new(3), &parts);
        assert!(matches!(result, Err(WorkflowError::StepOutOfBounds { step }) if step == StepIdx::new(99)));
    }

    #[test]
    fn validate_together_rejects_join_out_of_bounds() {
        let parts = make_parts_with_node_count(5);
        let result = validate_together(&[StepIdx::new(1)], StepIdx::new(99), &parts);
        assert!(matches!(result, Err(WorkflowError::StepOutOfBounds { step }) if step == StepIdx::new(99)));
    }

    // -- validate_two_steps --

    #[test]
    fn validate_two_steps_accepts_both_in_bounds() {
        let parts = make_parts_with_node_count(10);
        assert_eq!(validate_two_steps(StepIdx::new(1), StepIdx::new(5), &parts), Ok(()));
    }

    #[test]
    fn validate_two_steps_rejects_first_out_of_bounds() {
        let parts = make_parts_with_node_count(5);
        let result = validate_two_steps(StepIdx::new(99), StepIdx::new(1), &parts);
        assert!(matches!(result, Err(WorkflowError::StepOutOfBounds { step }) if step == StepIdx::new(99)));
    }

    #[test]
    fn validate_two_steps_rejects_second_out_of_bounds() {
        let parts = make_parts_with_node_count(5);
        let result = validate_two_steps(StepIdx::new(1), StepIdx::new(99), &parts);
        assert!(matches!(result, Err(WorkflowError::StepOutOfBounds { step }) if step == StepIdx::new(99)));
    }

    // -- validate_slot_and_steps --

    #[test]
    fn validate_slot_and_steps_accepts_all_valid() {
        let parts = make_parts_with_node_count(10);
        assert_eq!(
            validate_slot_and_steps(SlotIdx::new(2), StepIdx::new(1), StepIdx::new(5), &parts),
            Ok(())
        );
    }

    #[test]
    fn validate_slot_and_steps_rejects_bad_slot() {
        let mut parts = make_parts_with_node_count(10);
        parts.slot_count = 2;
        let result = validate_slot_and_steps(SlotIdx::new(5), StepIdx::new(1), StepIdx::new(5), &parts);
        assert!(matches!(result, Err(WorkflowError::SlotOutOfBounds { .. })));
    }

    // -- validate_repeat_start --

    #[test]
    fn validate_repeat_start_accepts_valid() {
        let parts = make_parts_with_node_count(10);
        assert_eq!(
            validate_repeat_start(3, StepIdx::new(1), StepIdx::new(5), &parts),
            Ok(())
        );
    }

    #[test]
    fn validate_repeat_start_rejects_zero_attempts() {
        let parts = make_parts_with_node_count(10);
        let result = validate_repeat_start(0, StepIdx::new(1), StepIdx::new(5), &parts);
        assert!(matches!(result, Err(WorkflowError::ResourceContractExceeded { resource }) if resource == "max_retry_attempts"));
    }

    // -- validate_slot_choose --

    #[test]
    fn validate_slot_choose_accepts_valid() {
        let mut parts = make_parts_with_node_count(5);
        parts.slot_count = 3;
        let branches = vec![SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(1),
        }];
        assert_eq!(validate_slot_choose(&branches, Some(StepIdx::new(2)), &parts), Ok(()));
    }

    #[test]
    fn validate_slot_choose_rejects_empty_no_otherwise() {
        let parts = make_parts_with_node_count(5);
        let result = validate_slot_choose(&[], None, &parts);
        assert!(matches!(result, Err(WorkflowError::EmptyBranchTable)));
    }

    // -- validate_expr_choose --

    #[test]
    fn validate_expr_choose_accepts_valid() {
        let mut parts = make_parts_with_node_count(5);
        parts.expressions = vec![crate::workflow::ExprProgram {
            ops: Box::new([]),
            max_stack: 0,
        }].into_boxed_slice();
        let branches = vec![ExprBranch {
            condition: ExprIdx::new(0),
            target: StepIdx::new(1),
        }];
        assert_eq!(validate_expr_choose(&branches, Some(StepIdx::new(2)), &parts), Ok(()));
    }

    #[test]
    fn validate_expr_choose_rejects_empty_no_otherwise() {
        let parts = make_parts_with_node_count(5);
        let result = validate_expr_choose(&[], None, &parts);
        assert!(matches!(result, Err(WorkflowError::EmptyBranchTable)));
    }

    // -- validate_for_each_start --

    #[test]
    fn validate_for_each_start_accepts_valid() {
        let mut parts = make_parts_with_node_count(10);
        parts.slot_count = 5;
        assert_eq!(
            validate_for_each_start(
                SlotIdx::new(0), SlotIdx::new(1),
                StepIdx::new(1), StepIdx::new(5), &parts
            ),
            Ok(())
        );
    }

    #[test]
    fn validate_for_each_start_rejects_bad_slot() {
        let mut parts = make_parts_with_node_count(10);
        parts.slot_count = 2;
        let result = validate_for_each_start(
            SlotIdx::new(0), SlotIdx::new(5),
            StepIdx::new(1), StepIdx::new(5), &parts
        );
        assert!(matches!(result, Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(5)));
    }

    // -- validate_reduce_start --

    #[test]
    fn validate_reduce_start_accepts_valid() {
        let mut parts = make_parts_with_node_count(10);
        parts.slot_count = 5;
        parts.constants = vec![ConstValue::Null].into_boxed_slice();
        assert_eq!(
            validate_reduce_start(
                SlotIdx::new(0), SlotIdx::new(1), ConstIdx::new(0),
                StepIdx::new(1), StepIdx::new(5), &parts
            ),
            Ok(())
        );
    }

    #[test]
    fn validate_reduce_start_rejects_bad_const() {
        let mut parts = make_parts_with_node_count(10);
        parts.slot_count = 5;
        parts.constants = vec![ConstValue::Null].into_boxed_slice();
        let result = validate_reduce_start(
            SlotIdx::new(0), SlotIdx::new(1), ConstIdx::new(5),
            StepIdx::new(1), StepIdx::new(5), &parts
        );
        assert!(matches!(result, Err(WorkflowError::ConstOutOfBounds { .. })));
    }

    // -- validate_reduce_next --

    #[test]
    fn validate_reduce_next_accepts_valid() {
        let mut parts = make_parts_with_node_count(10);
        parts.slot_count = 5;
        assert_eq!(
            validate_reduce_next(
                SlotIdx::new(0), SlotIdx::new(1),
                StepIdx::new(1), StepIdx::new(5), &parts
            ),
            Ok(())
        );
    }

    #[test]
    fn validate_reduce_next_rejects_bad_accumulator() {
        let mut parts = make_parts_with_node_count(10);
        parts.slot_count = 2;
        let result = validate_reduce_next(
            SlotIdx::new(0), SlotIdx::new(5),
            StepIdx::new(1), StepIdx::new(5), &parts
        );
        assert!(matches!(result, Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(5)));
    }
}
