use super::{assert_error, fixtures};
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx};
use vb_core::workflow::{
    CompiledNodeKind as K, CompiledWorkflow, InputSlotKind, SlotFlowError, SlotFlowFailure,
    WorkflowError, WorkflowParts, validate_slot_flow,
};

use fixtures::{declare, end, finish, join, n, parts, set};

fn union_graph() -> Result<WorkflowParts, SlotFlowError> {
    let mut p = fixtures::fixed(true, 0);
    let nodes = &mut p.nodes;

    let node3 = nodes
        .iter_mut()
        .find(|n| n.id == StepIdx::new(3))
        .ok_or_else(missing)?;
    node3.kind = K::SetConst {
        value: ConstIdx::new(1),
    };
    node3.output = Some(SlotIdx::new(1)).into();

    let node4 = nodes
        .iter_mut()
        .find(|n| n.id == StepIdx::new(4))
        .ok_or_else(missing)?;
    node4.kind = K::ConcurrentLaneEnd {
        owner: StepIdx::new(0),
        result: Some(SlotIdx::new(1)),
    };

    Ok(p)
}

fn missing() -> SlotFlowError {
    SlotFlowError {
        step: StepIdx::new(0),
        slot: None,
        reason: vb_core::workflow::SlotFlowFailure::InvalidReference,
    }
}

fn insert_wait(p: &mut WorkflowParts) -> Result<(), SlotFlowError> {
    let nodes = std::mem::replace(&mut p.nodes, Box::new([])).into_vec();
    let mut nodes = nodes;

    let node6 = nodes
        .iter_mut()
        .find(|n| n.id == StepIdx::new(6))
        .ok_or_else(missing)?;
    node6.kind = K::WaitUntil {
        deadline_slot: SlotIdx::new(3),
    };
    node6.routing.next = Some(StepIdx::new(7)).into();

    nodes.push(fixtures::finish(7, 3));

    p.nodes = nodes.into_boxed_slice();
    Ok(())
}

#[test]
fn successful_branch_types_union_without_writer_collision() -> Result<(), SlotFlowError> {
    validate_slot_flow(&union_graph()?)?;
    Ok(())
}

#[test]
fn union_keeps_known_invalid_kind() -> Result<(), SlotFlowError> {
    let mut p = union_graph()?;
    insert_wait(&mut p)?;
    assert_error(p, 6, None, SlotFlowFailure::IncompatibleType);
    Ok(())
}

#[test]
fn none_lane_result_is_null_not_dynamic() -> Result<(), SlotFlowError> {
    let mut p = fixtures::fixed(true, 0);
    for idx in [2usize, 4usize] {
        p.nodes.get_mut(idx).ok_or_else(missing)?.kind = K::ConcurrentLaneEnd {
            owner: StepIdx::new(0),
            result: None,
        };
    }
    insert_wait(&mut p)?;
    assert_error(p, 6, None, SlotFlowFailure::IncompatibleType);
    Ok(())
}

#[test]
fn dynamic_foreach_item_retains_runtime_check() -> Result<(), SlotFlowError> {
    let mut p = fixtures::foreach(3, 3);
    p.nodes.get_mut(1).ok_or_else(missing)?.kind = K::WaitUntil {
        deadline_slot: SlotIdx::new(1),
    };
    p.nodes.get_mut(1).ok_or_else(missing)?.output = None.into();
    p.nodes.get_mut(2).ok_or_else(missing)?.kind = K::ConcurrentLaneEnd {
        owner: StepIdx::new(0),
        result: Some(SlotIdx::new(1)),
    };
    validate_slot_flow(&p)?;
    Ok(())
}

#[test]
fn checked_constructor_rejects_dynamic_foreach_wait_fixture() -> Result<(), WorkflowError> {
    let mut p = fixtures::foreach(3, 3);
    p.nodes.get_mut(1).ok_or_else(missing)?.kind = K::WaitUntil {
        deadline_slot: SlotIdx::new(1),
    };
    p.nodes.get_mut(1).ok_or_else(missing)?.output = None.into();
    p.nodes.get_mut(2).ok_or_else(missing)?.kind = K::ConcurrentLaneEnd {
        owner: StepIdx::new(0),
        result: Some(SlotIdx::new(1)),
    };
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(p).map(|_| ()),
        Err(WorkflowError::ConcurrentLaneSuspension {
            owner: StepIdx::new(0),
            step: StepIdx::new(1),
        })
    );
    Ok(())
}

#[test]
fn same_frame_gate14_conflict_preserved() {
    let mut p = parts(
        vec![
            n(
                0,
                K::TogetherConcurrentStart {
                    branches: Box::new([StepIdx::new(1)]),
                    join: StepIdx::new(4),
                },
                None,
                None,
            ),
            set(1, 0, 1, 2),
            set(2, 1, 1, 3),
            end(3, 0, Some(1), 4),
            join(4, 0, 2, 5),
            finish(5, 2),
        ],
        3,
    );
    declare(&mut p, InputSlotKind::I64);
    assert_error(p, 2, Some(1), SlotFlowFailure::ConflictingConstantWrites);
}

#[test]
fn large_slot_count_uses_cell_budget_once() -> Result<(), SlotFlowError> {
    let mut p = fixtures::foreach(3, 3);
    p.slot_count = 8192;
    p.resource_contract.max_slots = 8192;
    validate_slot_flow(&p)?;
    Ok(())
}
