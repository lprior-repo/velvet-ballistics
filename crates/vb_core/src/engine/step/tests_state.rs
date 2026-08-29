use super::step_once;
use super::test_support::{ensure_equal, nop_then_finish_workflow, test_frame};
use super::tests_basic::{ask_workflow, wait_workflow};
use crate::EngineSignal;
use crate::frame::StepState;
use crate::ids::{ActionId, SlotIdx, StepIdx, WorkflowDigest};
use crate::value::{ConstValue, SlotValue};
use crate::value_store::ValueStore;
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

#[test]
fn step_once_error_handler_jumps_to_body() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("error_handler_test"),
        digest: WorkflowDigest::from_bytes([0xB1; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ErrorHandler {
                    body: StepIdx::new(1),
                    handler: StepIdx::new(2),
                    error_slot: None,
                },
            },
            finish_node(StepIdx::new(1)),
            finish_node(StepIdx::new(2)),
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(1)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),    })
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(&workflow)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(1))
        .map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(run.pc(), StepIdx::new(1))
}

#[test]
fn step_once_awaiting_action_preserves_pc() -> Result<(), String> {
    let workflow = single_do_workflow("await_action_preserves_pc", [0x55; 32])?;
    let mut run = test_frame(&workflow)?;
    ensure_equal(run.pc(), StepIdx::new(0))?;
    let mut store = ValueStore::new();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::AwaitingAction)?;
    ensure_equal(run.pc(), StepIdx::new(0))?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Running))
}

#[test]
fn step_once_signal_maps_to_correct_state() -> Result<(), String> {
    assert_continue_maps_to_succeeded()?;
    assert_awaiting_action_maps_to_running()?;
    assert_wait_maps_to_waiting()?;
    assert_ask_maps_to_asking()?;
    Ok(())
}

fn assert_continue_maps_to_succeeded() -> Result<(), String> {
    let workflow = nop_then_finish_workflow()?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();
    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))
}

fn assert_awaiting_action_maps_to_running() -> Result<(), String> {
    let workflow = single_do_workflow("signal_map_awaiting_action", [0x66; 32])?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();
    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
    ensure_equal(result, EngineSignal::AwaitingAction)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Running))
}

fn assert_wait_maps_to_waiting() -> Result<(), String> {
    let workflow = wait_workflow("signal_map_awaiting_wait", [0x77; 32])?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();
    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
    ensure_equal(result, EngineSignal::AwaitingWait)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Waiting))
}

fn assert_ask_maps_to_asking() -> Result<(), String> {
    let workflow = ask_workflow("signal_map_awaiting_ask", [0x88; 32])?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();
    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
    ensure_equal(result, EngineSignal::AwaitingAsk)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Asking))
}

fn single_do_workflow(name: &str, digest: [u8; 32]) -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from(name),
        digest: WorkflowDigest::from_bytes(digest),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(1),
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
    .map_err(|e| e.to_string())
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
