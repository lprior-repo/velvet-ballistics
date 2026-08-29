use super::step_once;
use super::test_support::{ensure_equal, nop_then_finish_workflow, test_frame};
use crate::EngineSignal;
use crate::frame::StepState;
use crate::ids::{ActionId, SlotIdx, StepIdx, WorkflowDigest};
use crate::value::{SlotValue, Taint};
use crate::value_store::ValueStore;
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

#[test]
fn step_once_nop_advances_pc_and_returns_continue() -> Result<(), String> {
    let workflow = nop_then_finish_workflow()?;
    let mut run = test_frame(&workflow)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
        .map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(run.pc(), StepIdx::new(1))?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))
}

#[test]
fn step_once_finish_returns_finished_with_value_and_taint() -> Result<(), String> {
    let workflow = nop_then_finish_workflow()?;
    let mut run = test_frame(&workflow)?;
    run.set_pc(StepIdx::new(1)).map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(42), Taint::Clean)
        .map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::I64(42), Taint::Clean),
    )?;
    ensure_equal(run.step_state(StepIdx::new(1)), Ok(StepState::Succeeded))
}

#[test]
fn step_once_do_returns_awaiting_action() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("do_test"),
        digest: WorkflowDigest::from_bytes([0x22; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(5),
                input: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),    })
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::AwaitingAction)
}

#[test]
fn step_once_wait_returns_awaiting_wait() -> Result<(), String> {
    let workflow = wait_workflow("wait_test", [0x33; 32])?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::AwaitingWait)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Waiting))
}

#[test]
fn step_once_ask_returns_awaiting_ask() -> Result<(), String> {
    let workflow = ask_workflow("ask_test", [0x44; 32])?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::AwaitingAsk)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Asking))
}

#[test]
fn step_once_jump_advances_pc_to_target() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("jump_test"),
        digest: WorkflowDigest::from_bytes([0x55; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Jump {
                    target: StepIdx::new(1),
                },
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
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),    })
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(run.pc(), StepIdx::new(1))
}

pub(super) fn wait_workflow(name: &str, digest: [u8; 32]) -> Result<CompiledWorkflow, String> {
    single_node_workflow(
        name,
        digest,
        CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::new(0),
        },
    )
}

pub(super) fn ask_workflow(name: &str, digest: [u8; 32]) -> Result<CompiledWorkflow, String> {
    single_node_workflow(
        name,
        digest,
        CompiledNodeKind::Ask {
            prompt: SlotIdx::new(0),
            timeout_slot: None,
        },
    )
}

fn single_node_workflow(
    name: &str,
    digest: [u8; 32],
    kind: CompiledNodeKind,
) -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from(name),
        digest: WorkflowDigest::from_bytes(digest),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind,
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),    })
    .map_err(|e| e.to_string())
}
