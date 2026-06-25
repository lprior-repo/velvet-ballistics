use crate::action::{ActionTicket, compute_action_idempotency_key};
use crate::frame::RunFrame;
use crate::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use crate::value::ConstValue;
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

pub(super) fn ensure_equal<T>(actual: T, expected: T) -> Result<(), String>
where
    T: core::fmt::Debug + PartialEq,
{
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, found {actual:?}"))
    }
}

pub(super) fn test_frame(workflow: &CompiledWorkflow) -> Result<RunFrame, String> {
    RunFrame::new(
        RunId::new(1),
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|e| e.to_string())
}

pub(super) fn nop_then_finish_workflow() -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("nop_finish"),
        digest: WorkflowDigest::from_bytes([0x11; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(42)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())
}

pub(super) fn action_ticket(run: u64, step: u16, action: u16, capacity: u16) -> ActionTicket {
    let run = RunId::new(run);
    let seq = SeqNo::new(1);
    let action = ActionId::new(action);
    ActionTicket {
        run,
        step: StepIdx::new(step),
        seq,
        action,
        attempt: 1,
        idempotency_key: compute_action_idempotency_key(run, seq, action),
        capacity,
    }
}

pub(super) fn single_do_workflow(
    name: &str,
    digest: [u8; 32],
    slot_count: u16,
) -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from(name),
        digest: WorkflowDigest::from_bytes(digest),
        nodes: vec![do_node(StepIdx::new(0), ActionId::new(1), None, None, None)]
            .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())
}

pub(super) fn do_then_finish_workflow(
    name: &str,
    digest: [u8; 32],
    slot_count: u16,
) -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from(name),
        digest: WorkflowDigest::from_bytes(digest),
        nodes: vec![
            do_node(
                StepIdx::new(0),
                ActionId::new(1),
                Some(StepIdx::new(1)),
                None,
                None,
            ),
            finish_node(StepIdx::new(1)),
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())
}

pub(super) fn do_with_error_handler_workflow(
    name: &str,
    digest: [u8; 32],
) -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from(name),
        digest: WorkflowDigest::from_bytes(digest),
        nodes: vec![
            do_node(
                StepIdx::new(0),
                ActionId::new(9),
                Some(StepIdx::new(1)),
                Some(StepIdx::new(2)),
                Some(SlotIdx::new(1)),
            ),
            finish_node(StepIdx::new(1)),
            finish_node(StepIdx::new(2)),
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())
}

fn do_node(
    id: StepIdx,
    action: ActionId,
    next: Option<StepIdx>,
    on_error: Option<StepIdx>,
    error_slot: Option<SlotIdx>,
) -> CompiledNode {
    CompiledNode {
        id,
        output: Some(SlotIdx::new(0)),
        next,
        on_error,
        error_slot,
        kind: CompiledNodeKind::Do {
            action,
            input: SlotIdx::new(0),
        },
    }
}

fn finish_node(id: StepIdx) -> CompiledNode {
    CompiledNode {
        id,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }
}
