//! Verified error-path tests for the step engine.
//!
//! Exercises error branches that are not covered by the main `tests.rs`
//! or `edge_cases.rs` modules.  Each test asserts a specific error variant
//! or return value — no `unwrap()` in test logic.

use crate::action::{ActionFailureCode, ActionTicket, RetryPolicy};
use crate::engine::choose::{choose_expr_branch, choose_slot_branch};
use crate::engine::node_helpers;
use crate::engine::step::{EngineSignal, resume_action_failure, step_once};
use crate::errors::EngineError;
use crate::frame::{RunFrame, StepState};
use crate::ids::{ActionId, ConstIdx, ExprIdx, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use crate::value::ConstValue;
use crate::value_store::ValueStore;
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprBranch, ExprOp, ExprProgram,
    ResourceContract, SlotBranch, WorkflowParts,
};

// ---------------------------------------------------------------------------
// Helpers
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

fn test_frame(workflow: &CompiledWorkflow) -> Result<crate::frame::RunFrame, String> {
    crate::frame::RunFrame::new(
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
    let mut store = ValueStore::new();

    // Suspend at the Do node
    let suspend = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
    ensure_equal(suspend, EngineSignal::AwaitingAction)?;

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
    let mut store = ValueStore::new();

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

// ---------------------------------------------------------------------------
// 3. choose_expr_branch — non-boolean result (I64 → TypeMismatch)
// ---------------------------------------------------------------------------

/// GAP-ERROR-003: When a Choose node's expression evaluates to I64 instead
/// of Bool, `choose_expr_branch` returns `EngineError::TypeMismatch`.
///
/// The expression `LoadConst(I64(42))` pushes `SlotValue::I64(42)` onto the
/// stack. The branch target checker sees a non-boolean and returns
/// `TypeMismatch { expected: "boolean", found: "number" }`.
fn i64_expr_plan() -> Result<CompiledWorkflow, String> {
    let expr =
        ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(0))].into_boxed_slice())
            .map_err(crate::WorkflowError::Expression)
            .map_err(|e| e.to_string())?;

    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("choose_i64_expr"),
        digest: WorkflowDigest::from_bytes([0xF3; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: vec![ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    }]
                    .into_boxed_slice(),
                    otherwise: None,
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
        expressions: vec![expr].into_boxed_slice(),
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

#[test]
fn choose_expr_branch_non_bool_i64_returns_type_mismatch() -> Result<(), String> {
    let plan = i64_expr_plan()?;
    let mut run = test_frame(&plan)?;
    let mut store = ValueStore::new();

    let branches = vec![ExprBranch {
        condition: ExprIdx::new(0),
        target: StepIdx::new(1),
    }];

    let result = choose_expr_branch(
        &plan,
        &mut run,
        &mut store,
        &branches,
        Some(StepIdx::new(2)),
    );

    match result {
        Err(EngineError::TypeMismatch {
            expected: "boolean",
            found: "number",
        }) => Ok(()),
        other => Err(format!(
            "expected TypeMismatch(boolean, number), got {other:?}"
        ))?,
    }
}

// ---------------------------------------------------------------------------
// 4. choose_slot_branch — empty branches + no otherwise → MissingNextStep
// ---------------------------------------------------------------------------

/// GAP-ERROR-004: When a ChooseSlot node has zero branches and no `otherwise`
/// slot, `choose_slot_branch` returns `EngineError::MissingNextStep`.
///
/// The empty branches loop terminates immediately, falling through to
/// `otherwise.ok_or(EngineError::MissingNextStep { step: run.pc() })`.
#[test]
fn choose_slot_branch_empty_no_otherwise_returns_missing_next_step() -> Result<(), String> {
    let branches: Vec<SlotBranch> = vec![];

    let result = choose_slot_branch(
        &mut RunFrame::new(RunId::new(1), StepIdx::new(0), 10, 1).unwrap(),
        &branches,
        None,
    );

    match result {
        Err(EngineError::MissingNextStep { step: _ }) => Ok(()),
        other => Err(format!("expected MissingNextStep, got {other:?}"))?,
    }
}

// ---------------------------------------------------------------------------
// 5. choose_expr_branch — all false + no otherwise → MissingNextStep
// ---------------------------------------------------------------------------

/// GAP-ERROR-005: When every branch in a Choose node evaluates to false and
/// there is no `otherwise`, `choose_expr_branch` returns
/// `EngineError::MissingNextStep`.
fn all_false_expr_plan() -> Result<CompiledWorkflow, String> {
    // Two expressions, both loading Bool(false)
    let expr0 =
        ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(0))].into_boxed_slice())
            .map_err(crate::WorkflowError::Expression)
            .map_err(|e| e.to_string())?;
    let expr1 =
        ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(1))].into_boxed_slice())
            .map_err(crate::WorkflowError::Expression)
            .map_err(|e| e.to_string())?;

    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("choose_all_false"),
        digest: WorkflowDigest::from_bytes([0xF5; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: vec![
                        ExprBranch {
                            condition: ExprIdx::new(0),
                            target: StepIdx::new(1),
                        },
                        ExprBranch {
                            condition: ExprIdx::new(1),
                            target: StepIdx::new(2),
                        },
                    ]
                    .into_boxed_slice(),
                    otherwise: None, // No fallback
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
        ]
        .into_boxed_slice(),
        expressions: vec![expr0, expr1].into_boxed_slice(),
        accessors: Box::new([]),
        constants: vec![ConstValue::Bool(false), ConstValue::Bool(false)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())
}

#[test]
fn choose_expr_branch_all_false_no_otherwise_returns_missing_next_step() -> Result<(), String> {
    let plan = all_false_expr_plan()?;
    let mut run = test_frame(&plan)?;
    let mut store = ValueStore::new();

    let branches = vec![
        ExprBranch {
            condition: ExprIdx::new(0),
            target: StepIdx::new(1),
        },
        ExprBranch {
            condition: ExprIdx::new(1),
            target: StepIdx::new(2),
        },
    ];

    let result = choose_expr_branch(&plan, &mut run, &mut store, &branches, None);

    match result {
        Err(EngineError::MissingNextStep { step: _ }) => Ok(()),
        other => Err(format!("expected MissingNextStep, got {other:?}"))?,
    }
}

// ---------------------------------------------------------------------------
// 6. node_helpers::set_const — ConstOutOfBounds
// ---------------------------------------------------------------------------

/// GAP-ERROR-006: When `set_const` is called with a `ConstIdx` that exceeds
/// the constant pool, it returns `EngineError::ConstOutOfBounds`.
///
/// We build a valid workflow (1 constant) and then directly invoke
/// `node_helpers::set_const` with a `ConstIdx` beyond the pool, bypassing
/// the workflow validator.
#[test]
fn set_const_invalid_const_index_returns_const_out_of_bounds() -> Result<(), String> {
    // Valid workflow with 1 constant — only index 0 is valid.
    let plan = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("set_const_oob"),
        digest: WorkflowDigest::from_bytes([0xF6; 32]),
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
                kind: CompiledNodeKind::Nop,
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(42)].into_boxed_slice(), // Only 1 constant
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())?;

    let mut run = test_frame(&plan)?;
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(99),
        },
    };

    let result = node_helpers::set_const(&plan, &mut run, &node, ConstIdx::new(99));

    match result {
        Err(EngineError::ConstOutOfBounds { index }) if index == ConstIdx::new(99) => Ok(()),
        Err(other) => Err(format!("expected ConstOutOfBounds(99), got {other:?}"))?,
        Ok(_) => Err("expected Err(ConstOutOfBounds), got Ok".to_string())?,
    }
}

// ---------------------------------------------------------------------------
// 7. node_helpers::copy_slot — SlotUninitialized
// ---------------------------------------------------------------------------

/// GAP-ERROR-007: When `copy_slot` is called with a source slot that has
/// never been written to, it returns `EngineError::SlotUninitialized`.
#[test]
fn copy_slot_uninitialized_source_returns_slot_uninitialized() -> Result<(), String> {
    let mut run = crate::frame::RunFrame::new(RunId::new(1), StepIdx::new(0), 3, 3)
        .map_err(|e| e.to_string())?;
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(1)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Copy {
            source: SlotIdx::new(2),
        },
    };

    let result = node_helpers::copy_slot(&mut run, &node, SlotIdx::new(2));

    match result {
        Err(EngineError::SlotUninitialized { slot }) if slot == SlotIdx::new(2) => Ok(()),
        Err(other) => Err(format!("expected SlotUninitialized(2), got {other:?}"))?,
        Ok(_) => Err("expected Err(SlotUninitialized), got Ok".to_string())?,
    }
}

// ---------------------------------------------------------------------------
// 8. node_helpers::finish_run — SlotOutOfBounds
// ---------------------------------------------------------------------------

/// GAP-ERROR-008: When `finish_run` is called with a `SlotIdx` that exceeds
/// the run frame's slot count, it returns `EngineError::SlotOutOfBounds`.
///
/// We create a RunFrame with 10 slots and directly call `finish_run` with
/// `SlotIdx(99)`, which is out of bounds.
#[test]
fn finish_run_slot_out_of_bounds_returns_slot_out_of_bounds() -> Result<(), String> {
    // Create a RunFrame with 10 slots.
    let mut run =
        RunFrame::new(RunId::new(1), StepIdx::new(0), 10, 10).map_err(|e| e.to_string())?;

    // Call finish_run with a slot index beyond the frame's 10 slots.
    let result = node_helpers::finish_run(&mut run, SlotIdx::new(99));

    match result {
        Err(EngineError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(99) => Ok(()),
        Err(other) => Err(format!("expected SlotOutOfBounds(99), got {other:?}"))?,
        Ok(_) => Err("expected Err(SlotOutOfBounds), got Ok".to_string())?,
    }
}
