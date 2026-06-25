use super::test_support::{ensure_equal, test_frame};
use super::{
    journal_action_suspended, resume_action_completion, resume_action_failure, step_once,
};
use crate::EngineSignal;
use crate::action::{ActionFailureCode, ActionJournalEvent, ActionTicket, RetryPolicy};
use crate::frame::StepState;
use crate::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use crate::value::{SlotValue, Taint};
use crate::value_store::ValueStore;
use crate::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts};

#[test]
fn resume_action_completion_writes_output_and_advances_pc() -> Result<(), String> {
    let workflow = resume_completion_workflow()?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();

    let suspend = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
    ensure_equal(suspend, EngineSignal::AwaitingAction)?;

    let ticket = action_ticket(1, 0, 1, 1);
    let (signal, _journal) = resume_action_completion(
        &workflow,
        &mut run,
        ticket,
        SlotIdx::new(0),
        SlotValue::I64(99),
        Taint::Clean,
    )
    .map_err(|e| e.to_string())?;

    ensure_equal(signal, EngineSignal::Continue)?;
    ensure_equal(
        *run.read_slot(SlotIdx::new(0)).map_err(|e| e.to_string())?,
        SlotValue::I64(99),
    )?;
    ensure_equal(run.pc(), StepIdx::new(1))?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))
}

#[test]
fn resume_action_failure_marks_step_failed() -> Result<(), String> {
    let workflow = single_do_workflow("resume_fail", [0xA2; 32])?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();

    let _suspend = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
    let ticket = action_ticket(1, 0, 1, 1);
    let (signal, _journal) = resume_action_failure(
        &workflow,
        &mut run,
        ticket,
        ActionFailureCode::Timeout,
        RetryPolicy::NonRetryable,
    )
    .map_err(|e| e.to_string())?;

    ensure_equal(signal, EngineSignal::AwaitingAction)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))
}

#[test]
fn journal_action_suspended_captures_all_fields() -> Result<(), String> {
    let ticket = action_ticket(1, 0, 1, 5);
    let event = journal_action_suspended(
        ticket,
        ActionId::new(5),
        SlotIdx::new(0),
        SlotIdx::new(1),
        StepIdx::new(0),
    );

    match event {
        ActionJournalEvent::Suspended {
            ticket: t,
            attempt,
            action,
            input_slot,
            output_slot,
            step,
        } => {
            ensure_equal(t.run, RunId::new(1))?;
            ensure_equal(attempt, 1)?;
            ensure_equal(action, ActionId::new(5))?;
            ensure_equal(input_slot, SlotIdx::new(0))?;
            ensure_equal(output_slot, SlotIdx::new(1))?;
            ensure_equal(step, StepIdx::new(0))
        }
        other => Err(format!("unexpected event: {other:?}")),
    }
}

fn action_ticket(run: u64, step: u16, seq: u64, action: u16) -> ActionTicket {
    ActionTicket {
        run: RunId::new(run),
        step: StepIdx::new(step),
        seq: SeqNo::new(seq),
        action: ActionId::new(action),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
    }
}

fn resume_completion_workflow() -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("resume_ok"),
        digest: WorkflowDigest::from_bytes([0xA1; 32]),
        nodes: vec![
            do_node(Some(StepIdx::new(1))),
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
    })
    .map_err(|e| e.to_string())
}

fn single_do_workflow(name: &str, digest: [u8; 32]) -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from(name),
        digest: WorkflowDigest::from_bytes(digest),
        nodes: vec![do_node(None)].into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())
}

fn do_node(next: Option<StepIdx>) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(1),
            input: SlotIdx::new(0),
        },
    }
}
