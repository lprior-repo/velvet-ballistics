use super::{assert_error, fixtures as f};
use vb_core::ids::{ConstIdx, ExprIdx, SlotIdx, StepIdx};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind as K, CompiledWorkflow, ErrorRouting, ExprOp as Op, ExprProgram,
    InputSlotKind, SlotFlowError, SlotFlowFailure, WorkflowError, WorkflowIrProfile, WorkflowParts,
    validate_slot_flow,
};
fn owner() -> CompiledNode {
    f::n(
        0,
        K::TogetherConcurrentStart {
            branches: Box::new([StepIdx::new(1)]),
            join: StepIdx::new(5),
        },
        None,
        None,
    )
}
fn partial() -> Result<WorkflowParts, WorkflowError> {
    let write = partial_write();
    let wait = recovery_wait();
    let mut p = f::parts(
        vec![
            owner(),
            write,
            f::end(2, 0, Some(0), 5),
            wait,
            f::end(4, 0, Some(0), 5),
            f::join(5, 0, 1, 6),
            f::finish(6, 1),
        ],
        2,
    );
    f::declare(&mut p, InputSlotKind::I64);
    p.expressions = Box::new([ExprProgram::try_from_ops(Box::new([Op::LoadConst(
        ConstIdx::new(1),
    )]))?]);
    Ok(p)
}
#[test]
fn handler_preserves_possible_partial_overwrite_types() -> Result<(), WorkflowError> {
    assert_error(partial()?, 3, None, SlotFlowFailure::IncompatibleType);
    Ok(())
}
#[test]
fn child_overwrite_preserves_original_parent_kind() -> Result<(), SlotFlowError> {
    let mut p = f::fixed(false, 0);
    let node = p.nodes.get_mut(1).ok_or_else(missing)?;
    node.kind = K::SetConst {
        value: ConstIdx::new(1),
    };
    node.output = Some(SlotIdx::new(0)).into();
    p.nodes.get_mut(2).ok_or_else(missing)?.kind = K::ConcurrentLaneEnd {
        owner: StepIdx::new(0),
        result: Some(SlotIdx::new(0)),
    };
    let mut nodes = p.nodes.into_vec();
    let parent = nodes.get_mut(6).ok_or_else(missing)?;
    parent.kind = K::WaitUntil {
        deadline_slot: SlotIdx::new(0),
    };
    parent.routing.next = Some(StepIdx::new(7)).into();
    nodes.push(f::finish(7, 3));
    p.nodes = nodes.into_boxed_slice();
    validate_slot_flow(&p)
}
fn missing() -> SlotFlowError {
    SlotFlowError {
        step: StepIdx::new(0),
        slot: None,
        reason: SlotFlowFailure::InvalidReference,
    }
}
// Historical case identity retained; raw flow success alone is not the acceptance oracle.
#[test]
fn raw_flow_does_not_open_constructor() -> Result<(), WorkflowError> {
    let original = f::foreach(3, 3);
    validate_slot_flow(&original)?;
    let workflow = CompiledWorkflow::try_from_concurrent_parts(original.clone())?;
    assert_eq!(workflow.ir_profile(), WorkflowIrProfile::ConcurrentV3);
    assert_eq!(
        workflow
            .concurrent_requirements()
            .map(|r| (r.frames(), r.groups())),
        Some((3, 1))
    );
    assert_eq!(workflow.to_parts(), original);
    Ok(())
}

#[test]
fn checked_foreach_constructor_rejects_non_list_input() -> Result<(), SlotFlowError> {
    let mut malformed = f::foreach(3, 3);
    malformed.input_slots.first_mut().ok_or_else(missing)?.kind = InputSlotKind::I64;
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(malformed).map(|_| ()),
        Err(WorkflowError::SlotFlow(SlotFlowError {
            step: StepIdx::new(0),
            slot: None,
            reason: SlotFlowFailure::IncompatibleType,
        }))
    );
    Ok(())
}

#[test]
fn foreach_fixture_remains_closed_under_legacy_constructor() {
    assert_eq!(
        CompiledWorkflow::try_from_parts(f::foreach(3, 3)).map(|_| ()),
        Err(WorkflowError::ConcurrentProfileRequired {
            step: StepIdx::new(0)
        })
    );
}
#[test]
fn oversized_analysis_rejects_before_structural_allocation()
-> Result<(), Box<dyn std::error::Error>> {
    let mut p = f::foreach(3, 3);
    let node = p.nodes.first().ok_or("missing owner")?.clone();
    let count = vb_core::limits::MAX_STEPS_PER_WORKFLOW
        .checked_add(1)
        .ok_or("count overflow")?;
    p.nodes = vec![node; count].into_boxed_slice();
    assert_error(p, 0, None, SlotFlowFailure::AnalysisCapacity);
    Ok(())
}

fn partial_write() -> CompiledNode {
    let mut write = f::n(
        1,
        K::EvalExpr {
            expr: ExprIdx::new(0),
        },
        Some(0),
        Some(2),
    );
    write.routing.error = ErrorRouting::Handler {
        handler: StepIdx::new(3),
        error_slot: None,
    };
    write
}

fn recovery_wait() -> CompiledNode {
    f::n(
        3,
        K::WaitUntil {
            deadline_slot: SlotIdx::new(0),
        },
        None,
        Some(4),
    )
}
