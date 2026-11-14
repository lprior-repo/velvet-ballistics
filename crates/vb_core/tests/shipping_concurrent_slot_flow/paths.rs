use super::{assert_error, fixtures as f};
use vb_core::ids::{ActionId, SlotIdx, StepIdx};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind as K, CompiledWorkflow, ErrorRouting, InputSlotKind, SlotBranch,
    SlotFlowFailure, WorkflowParts, validate_slot_flow,
};

fn group(join: u16) -> CompiledNode {
    f::n(
        0,
        K::TogetherConcurrentStart {
            branches: Box::new([StepIdx::new(1)]),
            join: StepIdx::new(join),
        },
        None,
        None,
    )
}

fn conditional(both: bool) -> WorkflowParts {
    let mut nodes = vec![group(5)];
    nodes.push(f::n(
        1,
        K::ChooseSlot {
            branches: Box::new([SlotBranch {
                condition: SlotIdx::new(0),
                target: StepIdx::new(2),
            }]),
            otherwise: Some(StepIdx::new(3)).into(),
        },
        None,
        None,
    ));
    nodes.push(f::set(2, 0, 1, 4));
    if both {
        nodes.push(f::set(3, 0, 1, 4));
    } else {
        nodes.push(f::n(3, K::Nop, None, Some(4)));
    }
    nodes.push(f::end(4, 0, Some(1), 5));
    nodes.push(f::join(5, 0, 2, 6));
    nodes.push(f::finish(6, 2));
    let mut parts = f::parts(nodes, 3);
    f::declare(&mut parts, InputSlotKind::Bool);
    parts
}

fn action(handler_read: u16) -> WorkflowParts {
    let mut nodes = vec![group(5)];
    nodes.push(f::n(
        1,
        K::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        },
        Some(1),
        Some(2),
    ));
    if let Some(do_node) = nodes.last_mut() {
        do_node.routing.error = ErrorRouting::Handler {
            handler: StepIdx::new(3),
            error_slot: Some(SlotIdx::new(2)),
        };
    }
    nodes.push(f::end(2, 0, Some(1), 5));
    nodes.push(f::copy(3, handler_read, 3, 4));
    nodes.push(f::end(4, 0, Some(3), 5));
    nodes.push(f::join(5, 0, 4, 6));
    nodes.push(f::finish(6, 4));
    let mut parts = f::parts(nodes, 5);
    f::declare(&mut parts, InputSlotKind::I64);
    parts
}

fn nested(parent_read: u16) -> WorkflowParts {
    let mut nodes = vec![nested_owner()];
    nodes.push(f::n(
        1,
        K::FirstSuccessStart {
            branches: Box::new([StepIdx::new(2)]),
            join: StepIdx::new(4),
        },
        None,
        None,
    ));
    nodes.push(f::copy(2, 1, 2, 3));
    nodes.push(f::end(3, 1, Some(2), 4));
    nodes.push(f::join(4, 1, 3, 5));
    nodes.push(f::end(5, 0, Some(3), 6));
    nodes.push(f::join(6, 0, 4, 7));
    nodes.push(f::finish(7, parent_read));
    let mut parts = f::parts(nodes, 5);
    f::declare(&mut parts, InputSlotKind::List);
    parts
}

#[test]
fn conditional_result_definite() {
    let parts = conditional(true);
    let result = validate_slot_flow(&parts);
    assert_eq!(result, Ok(()));
}

#[test]
fn conditional_missing_result_rejected() {
    let parts = conditional(false);
    assert_error(parts, 4, Some(1), SlotFlowFailure::UninitializedRead);
}

#[test]
fn action_error_value_is_assigned() {
    let parts = action(2);
    let result = validate_slot_flow(&parts);
    assert_eq!(result, Ok(()));
}

#[test]
fn action_error_does_not_initialize_success_output() {
    let parts = action(1);
    assert_error(parts, 3, Some(1), SlotFlowFailure::UninitializedRead);
}

#[test]
fn checked_constructor_rejects_action_fixture() {
    let parts = action(1);
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(parts).map(|_| ()),
        Err(vb_core::workflow::WorkflowError::ConcurrentLaneSuspension {
            owner: StepIdx::new(0),
            step: StepIdx::new(1),
        })
    );
}

#[test]
fn nested_race_sees_outer_item() {
    let parts = nested(4);
    let result = validate_slot_flow(&parts);
    assert_eq!(result, Ok(()));
}

#[test]
fn nested_item_does_not_escape_to_root() {
    let parts = nested(1);
    assert_error(parts, 7, Some(1), SlotFlowFailure::UninitializedRead);
}

#[test]
fn nested_join_result_must_be_definite() {
    let mut parts = nested(4);
    if let Some(node) = parts.nodes.iter_mut().find(|n| n.id == StepIdx::new(2)) {
        node.kind = K::Nop;
        node.output = None.into();
    }
    assert_error(parts, 3, Some(2), SlotFlowFailure::UninitializedRead);
}

#[test]
fn zero_nested_for_each_still_returns_list() {
    let mut parts = nested(4);
    if let Some(node) = parts.nodes.iter_mut().find(|n| n.id == StepIdx::new(0)) {
        if let K::ForEachConcurrentStart { limit, .. } = &mut node.kind {
            *limit = 0u32.into();
        }
    }
    let result = validate_slot_flow(&parts);
    assert_eq!(result, Ok(()));
}

fn nested_owner() -> CompiledNode {
    f::n(
        0,
        K::ForEachConcurrentStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(1),
            limit: 3u32.into(),
            at_once: 2u16.into(),
            body: StepIdx::new(1),
            join: StepIdx::new(6),
        },
        None,
        None,
    )
}
