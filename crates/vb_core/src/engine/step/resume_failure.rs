//! Action-failure resumption error-path tests.
//!
//! Exercises the error branches in `resume_action_failure`:
//! - Error handler routing (GAP-ERROR-001)
//! - Out-of-bounds step index (GAP-ERROR-002)

use crate::action::{ActionFailureCode, ActionTicket, RetryPolicy};
use crate::engine::step::{EngineSignal, resume_action_failure, step_once};
use crate::errors::EngineError;
use crate::frame::{RunFrame, StepState};
use crate::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

// ---------------------------------------------------------------------------
// Helpers (module-local)
// ---------------------------------------------------------------------------

fn ensure_equal<T>(actual: T, expected: T) -> Result<(), String>
where
    T: core::fmt::Debug + PartialEq,
{
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, found {actual:?}"))
    }
}

fn test_frame(workflow: &CompiledWorkflow) -> Result<RunFrame, String> {
    RunFrame::new(
        RunId::new(1),
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// 1. resume_action_failure with error handler — routed path
// ---------------------------------------------------------------------------

/// GAP-ERROR-001: When a Do node has `on_error` set, `resume_action_failure`
/// routes to the handler step and returns `EngineSignal::Continue`.
///
/// The Do node declares `on_error: Some(StepIdx::new(2))`. After the action
/// fails, `route_error_handler` in `resume_action_failure` detects the
/// handler and advances the PC to step 2.
#[test]
fn resume_action_failure_with_handler_routes_to_handler_and_returns_continue() -> Result<(), String>
{
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("resume_fail_with_handler"),
        digest: WorkflowDigest::from_bytes([0xF1; 32]),
        nodes: vec![
            // Step 0: Do node with error handler at step 2, normal path to step 1
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: Some(StepIdx::new(2)),
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(1),
                    input: SlotIdx::new(0),
                },
            },
            // Step 1: success path — finish the run (reachable via next)
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
            // Step 2: error handler body (reachable via on_error)
            CompiledNode {
                id: StepIdx::new(2),
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
    .map_err(|e| e.to_string())?;

    let mut run = test_frame(&workflow)?;
    let mut store = crate::value_store::ValueStore::new();

    // Suspend at the Do node
    let suspend = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
    ensure_equal(
        suspend,
        EngineSignal::AwaitingAction {
            step: StepIdx::new(0),
            seq: SeqNo::ZERO,
            action: ActionId::new(1),
        },
    )?;

    // Fail the action — should route to handler (step 2)
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(1),
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
        ..Default::default()
    };

    let (signal, _journal) = resume_action_failure(
        &workflow,
        &mut run,
        ticket,
        ActionFailureCode::Timeout,
        RetryPolicy::NonRetryable,
    )
    .map_err(|e| e.to_string())?;

    ensure_equal(signal, EngineSignal::Continue)?;
    // PC should be at the handler step (step 2)
    ensure_equal(run.pc(), StepIdx::new(2))?;
    // The Do step should be in Failed state
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))
}

// ---------------------------------------------------------------------------
// 2. resume_action_failure — mark_failed error (StepStateOutOfBounds)
// ---------------------------------------------------------------------------

/// GAP-ERROR-002: When `resume_action_failure` is called with a step index
/// that exceeds the workflow's node count, `mark_failed` returns
/// `EngineError::StepStateOutOfBounds`.
///
/// This tests the error path on line 218 of `engine/step.rs`:
///   run.mark_failed(step)?
/// where `step` comes from the ticket and could theoretically be out of bounds
/// if the ticket was fabricated or corrupted.
#[test]
fn resume_action_failure_out_of_bounds_step_returns_step_state_out_of_bounds() -> Result<(), String>
{
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("resume_fail_oob_step"),
        digest: WorkflowDigest::from_bytes([0xF2; 32]),
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
    })
    .map_err(|e| e.to_string())?;

    let mut run = test_frame(&workflow)?;
    let mut store = crate::value_store::ValueStore::new();

    // Suspend at the Do node to establish a Running step
    let _suspend = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    // Create a ticket with an out-of-bounds step index (step 99 in a 1-step workflow)
    let ticket = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(99),
        seq: SeqNo::new(1),
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
        ..Default::default()
    };

    let result = resume_action_failure(
        &workflow,
        &mut run,
        ticket,
        ActionFailureCode::Timeout,
        RetryPolicy::NonRetryable,
    );

    match result {
        Err(EngineError::StepStateOutOfBounds { step }) if step == StepIdx::new(99) => Ok(()),
        Err(other) => Err(format!("expected StepStateOutOfBounds(99), got {other:?}"))?,
        Ok(_) => Err("expected Err(StepStateOutOfBounds), got Ok".to_string())?,
    }
}
