#![forbid(unsafe_code)]
//! Integration tests for run_loop behavior: run_until_blocked, drive_deterministic,
//! signal propagation, state transitions, edge cases, and proptest invariants.

use crate::frame::StepState;
use crate::ids::{ActionId, ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use crate::limits::MAX_STEP_BUDGET;
use crate::value::{ConstValue, SlotValue, Taint};
use crate::value_store::ValueStore;
use crate::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowParts};

use crate::engine::{EngineSignal, StepBudget, drive_deterministic, new_run_frame, run_until_blocked, step_once};

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

fn test_store() -> ValueStore {
    ValueStore::new()
}

fn test_frame(run_id: RunId, workflow: &CompiledWorkflow) -> Result<crate::RunFrame, String> {
    new_run_frame(run_id, workflow).map_err(|error| error.to_string())
}

// =============================================================================
// Workflow helpers
// =============================================================================

fn two_step_workflow(value: ConstValue) -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("two_step"),
        digest: WorkflowDigest::from_bytes([0x01; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
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
        constants: vec![value].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())
}

fn single_step_finish_workflow() -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("single_finish"),
        digest: WorkflowDigest::from_bytes([0x02; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())
}

fn do_then_finish_workflow() -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("do_then_finish"),
        digest: WorkflowDigest::from_bytes([0x03; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(1),
                    input: SlotIdx::new(0),
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
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())
}

fn wait_workflow() -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("wait"),
        digest: WorkflowDigest::from_bytes([0x04; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())
}

fn ask_workflow() -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("ask"),
        digest: WorkflowDigest::from_bytes([0x05; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: None,
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())
}

fn four_step_set_const_chain_workflow(values: &[ConstValue]) -> Result<CompiledWorkflow, String> {
    let nodes: Vec<CompiledNode> = (0..values.len())
        .map(|i| {
            let is_last = i == values.len() - 1;
            CompiledNode {
                id: StepIdx::new(i as u16),
                output: Some(SlotIdx::new(i as u16)),
                next: if is_last {
                    Some(StepIdx::new(values.len() as u16))
                } else {
                    Some(StepIdx::new(i as u16 + 1))
                },
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(i as u16),
                },
            }
        })
        .chain(std::iter::once(CompiledNode {
            id: StepIdx::new(values.len() as u16),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(values.len() as u16 - 1),
            },
        }))
        .collect();

    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("set_chain"),
        digest: WorkflowDigest::from_bytes([0x06; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: values.to_vec().into_boxed_slice(),
        slot_count: values.len() as u16,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())
}

fn nop_then_finish_workflow() -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("nop_finish"),
        digest: WorkflowDigest::from_bytes([0x07; 32]),
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
        constants: vec![ConstValue::I64(99)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())
}

// =============================================================================
// A. run_until_blocked happy paths with exact value assertions
// =============================================================================

#[test]
fn run_until_blocked_completes_two_step_workflow_with_i64_value() -> Result<(), String> {
    let workflow = two_step_workflow(ConstValue::I64(42))?;
    let mut run = test_frame(RunId::new(1), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::Finished(SlotValue::I64(42), Taint::Clean))?;
    ensure_equal(run.executed(), 2)?;
    ensure_equal(run.pc(), StepIdx::new(1))
}

#[test]
fn run_until_blocked_completes_two_step_workflow_with_bool_true_value() -> Result<(), String> {
    let workflow = two_step_workflow(ConstValue::Bool(true))?;
    let mut run = test_frame(RunId::new(2), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|e| e.to_string())?;

    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::Bool(true), Taint::Clean),
    )?;
    ensure_equal(run.executed(), 2)
}

#[test]
fn run_until_blocked_completes_two_step_workflow_with_bool_false_value() -> Result<(), String> {
    let workflow = two_step_workflow(ConstValue::Bool(false))?;
    let mut run = test_frame(RunId::new(3), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|e| e.to_string())?;

    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::Bool(false), Taint::Clean),
    )
}

#[test]
fn run_until_blocked_completes_two_step_workflow_with_null_value() -> Result<(), String> {
    let workflow = two_step_workflow(ConstValue::Null)?;
    let mut run = test_frame(RunId::new(4), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|e| e.to_string())?;

    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::Null, Taint::Clean),
    )?;
    ensure_equal(run.executed(), 2)
}

#[test]
fn run_until_blocked_completes_single_step_finish_with_preseeded_slot() -> Result<(), String> {
    let workflow = single_step_finish_workflow()?;
    let mut run = test_frame(RunId::new(5), &workflow)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(55), Taint::Clean)
        .map_err(|e| e.to_string())?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|e| e.to_string())?;

    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::I64(55), Taint::Clean),
    )?;
    ensure_equal(run.executed(), 1)
}

#[test]
fn run_until_blocked_completes_single_step_finish_with_secret_taint() -> Result<(), String> {
    let workflow = single_step_finish_workflow()?;
    let mut run = test_frame(RunId::new(6), &workflow)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(88), Taint::Secret)
        .map_err(|e| e.to_string())?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|e| e.to_string())?;

    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::I64(88), Taint::Secret),
    )
}

// =============================================================================
// B. drive_deterministic happy paths and budget exhaustion
// =============================================================================

#[test]
fn drive_deterministic_exact_budget_two_completes_two_step_workflow() -> Result<(), String> {
    let workflow = two_step_workflow(ConstValue::I64(33))?;
    let mut run = test_frame(RunId::new(10), &workflow)?;
    let mut store = test_store();
    let mut budget = StepBudget::new(2);

    let result =
        drive_deterministic(&workflow, &mut run, &mut budget, &mut store)
            .map_err(|e| e.to_string())?;

    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::I64(33), Taint::Clean),
    )?;
    ensure_equal(run.executed(), 2)?;
    ensure_equal(budget.remaining(), 0)
}

#[test]
fn drive_deterministic_budget_one_exhausts_on_two_step_workflow() -> Result<(), String> {
    let workflow = two_step_workflow(ConstValue::I64(7))?;
    let mut run = test_frame(RunId::new(11), &workflow)?;
    let mut store = test_store();
    let mut budget = StepBudget::new(1);

    let result =
        drive_deterministic(&workflow, &mut run, &mut budget, &mut store)
            .map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::StepBudgetExhausted)?;
    ensure_equal(run.executed(), 1)?;
    ensure_equal(run.pc(), StepIdx::new(1))?;
    ensure_equal(budget.remaining(), 0)
}

#[test]
fn drive_deterministic_budget_zero_returns_step_budget_exhausted_frame_unchanged()
-> Result<(), String> {
    let workflow = two_step_workflow(ConstValue::I64(1))?;
    let mut run = test_frame(RunId::new(12), &workflow)?;
    let mut store = test_store();
    let initial_executed = run.executed();
    let initial_pc = run.pc();
    let mut budget = StepBudget::new(0);

    let result =
        drive_deterministic(&workflow, &mut run, &mut budget, &mut store)
            .map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::StepBudgetExhausted)?;
    ensure_equal(run.executed(), initial_executed)?;
    ensure_equal(run.pc(), initial_pc)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Pending))
}

#[test]
fn drive_deterministic_budget_three_completes_three_step_workflow() -> Result<(), String> {
    let workflow = four_step_set_const_chain_workflow(&[
        ConstValue::I64(10),
        ConstValue::I64(20),
        ConstValue::I64(30),
    ])?;
    let mut run = test_frame(RunId::new(13), &workflow)?;
    let mut store = test_store();
    let mut budget = StepBudget::new(4);

    let result =
        drive_deterministic(&workflow, &mut run, &mut budget, &mut store)
            .map_err(|e| e.to_string())?;

    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::I64(30), Taint::Clean),
    )?;
    ensure_equal(run.executed(), 4)?;
    ensure_equal(budget.remaining(), 0)
}

#[test]
fn drive_deterministic_budget_mid_exhaustion_keeps_leftover_remaining() -> Result<(), String> {
    let workflow = four_step_set_const_chain_workflow(&[
        ConstValue::I64(1),
        ConstValue::I64(2),
        ConstValue::I64(3),
    ])?;
    let mut run = test_frame(RunId::new(14), &workflow)?;
    let mut store = test_store();
    let mut budget = StepBudget::new(10);

    let result =
        drive_deterministic(&workflow, &mut run, &mut budget, &mut store)
            .map_err(|e| e.to_string())?;

    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::I64(3), Taint::Clean),
    )?;
    ensure_equal(budget.remaining(), 6)
}

// =============================================================================
// C. Signal propagation: AwaitingAction, AwaitingWait, AwaitingAsk, Finished
// =============================================================================

#[test]
fn drive_deterministic_stops_on_do_node_with_awaiting_action_signal() -> Result<(), String> {
    let workflow = do_then_finish_workflow()?;
    let mut run = test_frame(RunId::new(20), &workflow)?;
    let mut store = test_store();
    let mut budget = StepBudget::MAX;

    let result =
        drive_deterministic(&workflow, &mut run, &mut budget, &mut store)
            .map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::AwaitingAction)?;
    ensure_equal(run.executed(), 1)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Running))
}

#[test]
fn drive_deterministic_stops_on_wait_node_with_awaiting_wait_signal() -> Result<(), String> {
    let workflow = wait_workflow()?;
    let mut run = test_frame(RunId::new(21), &workflow)?;
    let mut store = test_store();
    let mut budget = StepBudget::MAX;

    let result =
        drive_deterministic(&workflow, &mut run, &mut budget, &mut store)
            .map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::AwaitingWait)?;
    ensure_equal(run.executed(), 1)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Waiting))
}

#[test]
fn drive_deterministic_stops_on_ask_node_with_awaiting_ask_signal() -> Result<(), String> {
    let workflow = ask_workflow()?;
    let mut run = test_frame(RunId::new(22), &workflow)?;
    let mut store = test_store();
    let mut budget = StepBudget::MAX;

    let result =
        drive_deterministic(&workflow, &mut run, &mut budget, &mut store)
            .map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::AwaitingAsk)?;
    ensure_equal(run.executed(), 1)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Asking))
}

#[test]
fn run_until_blocked_stops_on_do_node_awaiting_action_preserves_pc() -> Result<(), String> {
    let workflow = do_then_finish_workflow()?;
    let mut run = test_frame(RunId::new(23), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::AwaitingAction)?;
    ensure_equal(run.pc(), StepIdx::new(0))
}

#[test]
fn run_until_blocked_stops_on_wait_node_awaiting_wait_preserves_pc() -> Result<(), String> {
    let workflow = wait_workflow()?;
    let mut run = test_frame(RunId::new(24), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::AwaitingWait)?;
    ensure_equal(run.pc(), StepIdx::new(0))
}

#[test]
fn run_until_blocked_stops_on_ask_node_awaiting_ask_preserves_pc() -> Result<(), String> {
    let workflow = ask_workflow()?;
    let mut run = test_frame(RunId::new(25), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::AwaitingAsk)?;
    ensure_equal(run.pc(), StepIdx::new(0))
}

#[test]
fn drive_deterministic_do_then_finish_only_executes_do_not_advance_past_suspension()
-> Result<(), String> {
    let workflow = do_then_finish_workflow()?;
    let mut run = test_frame(RunId::new(26), &workflow)?;
    let mut store = test_store();
    let mut budget = StepBudget::MAX;

    let result =
        drive_deterministic(&workflow, &mut run, &mut budget, &mut store)
            .map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::AwaitingAction)?;
    ensure_equal(run.executed(), 1)?;
    ensure_equal(run.pc(), StepIdx::new(0))
}

// =============================================================================
// D. State transitions during execution
// =============================================================================

#[test]
fn run_until_blocked_marks_all_non_suspension_steps_as_succeeded() -> Result<(), String> {
    let workflow = two_step_workflow(ConstValue::I64(10))?;
    let mut run = test_frame(RunId::new(30), &workflow)?;
    let mut store = test_store();

    let _ = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|e| e.to_string())?;

    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
    ensure_equal(run.step_state(StepIdx::new(1)), Ok(StepState::Succeeded))
}

#[test]
fn run_until_blocked_step_budget_exhausted_leaves_pending_step_as_pending()
-> Result<(), String> {
    let workflow = two_step_workflow(ConstValue::I64(42))?;
    let mut run = test_frame(RunId::new(31), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::new(1), &mut store)
        .map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::StepBudgetExhausted)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
    ensure_equal(run.step_state(StepIdx::new(1)), Ok(StepState::Pending))
}

#[test]
fn drive_deterministic_wait_node_marks_step_as_waiting() -> Result<(), String> {
    let workflow = wait_workflow()?;
    let mut run = test_frame(RunId::new(32), &workflow)?;
    let mut store = test_store();
    let mut budget = StepBudget::MAX;

    let result =
        drive_deterministic(&workflow, &mut run, &mut budget, &mut store)
            .map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::AwaitingWait)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Waiting))
}

#[test]
fn drive_deterministic_ask_node_marks_step_as_asking() -> Result<(), String> {
    let workflow = ask_workflow()?;
    let mut run = test_frame(RunId::new(33), &workflow)?;
    let mut store = test_store();
    let mut budget = StepBudget::MAX;

    let result =
        drive_deterministic(&workflow, &mut run, &mut budget, &mut store)
            .map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::AwaitingAsk)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Asking))
}

#[test]
fn drive_deterministic_nop_then_finish_transitions_through_pending_to_succeeded()
-> Result<(), String> {
    let workflow = nop_then_finish_workflow()?;
    let mut run = test_frame(RunId::new(34), &workflow)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(99))
        .map_err(|e| e.to_string())?;
    let mut store = test_store();
    let mut budget = StepBudget::new(2);

    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Pending))?;
    ensure_equal(run.step_state(StepIdx::new(1)), Ok(StepState::Pending))?;

    let result =
        drive_deterministic(&workflow, &mut run, &mut budget, &mut store)
            .map_err(|e| e.to_string())?;

    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::I64(99), Taint::Clean),
    )?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
    ensure_equal(run.step_state(StepIdx::new(1)), Ok(StepState::Succeeded))
}

// =============================================================================
// E. Edge cases: single-step, max budget, zero budget, empty workflow
// =============================================================================

#[test]
fn run_until_blocked_single_step_changes_state_from_pending_to_succeeded() -> Result<(), String> {
    let workflow = single_step_finish_workflow()?;
    let mut run = test_frame(RunId::new(40), &workflow)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(true))
        .map_err(|e| e.to_string())?;
    let mut store = test_store();

    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Pending))?;

    let _ = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|e| e.to_string())?;

    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))
}

#[test]
fn run_until_blocked_max_budget_executes_all_10k_step_transitions() -> Result<(), String> {
    let budget = StepBudget::MAX;
    ensure_equal(budget.remaining(), MAX_STEP_BUDGET)?;

    let workflow = two_step_workflow(ConstValue::I64(99))?;
    let mut run = test_frame(RunId::new(41), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, budget, &mut store)
        .map_err(|e| e.to_string())?;

    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::I64(99), Taint::Clean),
    )
}

#[test]
fn run_until_blocked_zero_budget_exhausts_immediately_without_execution() -> Result<(), String> {
    let workflow = two_step_workflow(ConstValue::I64(1))?;
    let mut run = test_frame(RunId::new(42), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::new(0), &mut store)
        .map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::StepBudgetExhausted)?;
    ensure_equal(run.executed(), 0)?;
    ensure_equal(run.pc(), StepIdx::new(0))
}

#[test]
fn drive_deterministic_zero_budget_exhausts_immediately_no_state_change() -> Result<(), String> {
    let workflow = two_step_workflow(ConstValue::I64(1))?;
    let mut run = test_frame(RunId::new(43), &workflow)?;
    let mut store = test_store();
    let mut budget = StepBudget::new(0);

    let result =
        drive_deterministic(&workflow, &mut run, &mut budget, &mut store)
            .map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::StepBudgetExhausted)?;
    ensure_equal(run.executed(), 0)?;
    ensure_equal(run.pc(), StepIdx::new(0))?;
    ensure_equal(budget.remaining(), 0)
}

#[test]
fn run_until_blocked_budget_one_executes_only_first_step() -> Result<(), String> {
    let workflow = two_step_workflow(ConstValue::I64(5))?;
    let mut run = test_frame(RunId::new(44), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::new(1), &mut store)
        .map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::StepBudgetExhausted)?;
    ensure_equal(run.executed(), 1)?;
    ensure_equal(
        *run.read_slot(SlotIdx::new(0)).map_err(|e| e.to_string())?,
        SlotValue::I64(5),
    )?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))
}

#[test]
fn run_until_blocked_single_step_budget_clamped_does_not_overflow() -> Result<(), String> {
    let clamped = StepBudget::new(u64::MAX);
    ensure_equal(clamped.remaining(), MAX_STEP_BUDGET)
}

#[test]
fn run_until_blocked_budget_just_above_max_clamps_to_max() -> Result<(), String> {
    let just_above = StepBudget::new(MAX_STEP_BUDGET + 1);
    ensure_equal(just_above.remaining(), MAX_STEP_BUDGET)
}

// =============================================================================
// F. step_once driven tests
// =============================================================================

#[test]
fn step_once_do_node_returns_awaiting_action_and_preserves_pc() -> Result<(), String> {
    let workflow = do_then_finish_workflow()?;
    let mut run = test_frame(RunId::new(50), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::AwaitingAction)?;
    ensure_equal(run.pc(), StepIdx::new(0))
}

#[test]
fn step_once_wait_node_returns_awaiting_wait_and_marks_waiting() -> Result<(), String> {
    let workflow = wait_workflow()?;
    let mut run = test_frame(RunId::new(51), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::AwaitingWait)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Waiting))
}

#[test]
fn step_once_ask_node_returns_awaiting_ask_and_marks_asking() -> Result<(), String> {
    let workflow = ask_workflow()?;
    let mut run = test_frame(RunId::new(52), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::AwaitingAsk)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Asking))
}

#[test]
fn step_once_finish_node_returns_finished_with_exact_value_and_taint() -> Result<(), String> {
    let workflow = single_step_finish_workflow()?;
    let mut run = test_frame(RunId::new(53), &workflow)?;
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::I64(999),
        Taint::DerivedFromSecret,
    )
    .map_err(|e| e.to_string())?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::I64(999), Taint::DerivedFromSecret),
    )
}

#[test]
fn step_once_set_const_then_step_once_finish_produces_continue_then_finished()
-> Result<(), String> {
    let workflow = two_step_workflow(ConstValue::I64(77))?;
    let mut run = test_frame(RunId::new(54), &workflow)?;
    let mut store = test_store();

    let s0 = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
    ensure_equal(s0, EngineSignal::Continue)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;

    let s1 = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
    ensure_equal(
        s1,
        EngineSignal::Finished(SlotValue::I64(77), Taint::Clean),
    )?;
    ensure_equal(run.step_state(StepIdx::new(1)), Ok(StepState::Succeeded))
}

// =============================================================================
// G. Proptest: step-by-step = bulk execution, no_panic for any budget
// =============================================================================

proptest::proptest! {
    /// Step-by-step execution yields the same final signal and executed count
    /// as running with unlimited budget.
    #[test]
    fn prop_step_by_step_equals_bulk_execution(
        value in proptest::prelude::any::<i64>(),
    ) {
        let workflow = match two_step_workflow(ConstValue::I64(value)) {
            Ok(wf) => wf,
            Err(_) => { proptest::prop_assert!(false, "workflow creation failed"); return; }
        };

        let mut run_bulk = match test_frame(RunId::new(100), &workflow) {
            Ok(r) => r,
            Err(_) => { proptest::prop_assert!(false, "bulk frame creation failed"); return; }
        };
        let mut store_bulk = test_store();

        let (bulk_signal, bulk_executed) = match run_until_blocked(
            &workflow,
            &mut run_bulk,
            StepBudget::MAX,
            &mut store_bulk,
        ) {
            Ok(sig) => (sig, run_bulk.executed()),
            Err(_) => { proptest::prop_assert!(false, "bulk execution failed"); return; }
        };

        let mut run_step = match test_frame(RunId::new(101), &workflow) {
            Ok(r) => r,
            Err(_) => { proptest::prop_assert!(false, "step frame creation failed"); return; }
        };
        let mut store_step = test_store();

        let s0 = match step_once(&workflow, &mut run_step, &mut store_step) {
            Ok(sig) => sig,
            Err(_) => { proptest::prop_assert!(false, "step0 failed"); return; }
        };
        proptest::prop_assert_eq!(s0, EngineSignal::Continue);

        let s1 = match step_once(&workflow, &mut run_step, &mut store_step) {
            Ok(sig) => sig,
            Err(_) => { proptest::prop_assert!(false, "step1 failed"); return; }
        };
        proptest::prop_assert_eq!(s1, bulk_signal);
        proptest::prop_assert_eq!(run_step.executed(), bulk_executed);
    }

    /// drive_deterministic never panics for any budget value on a two-step workflow.
    #[test]
    fn prop_drive_deterministic_no_panic_for_any_budget(
        budget_value in 0u64..(MAX_STEP_BUDGET + 100),
    ) {
        let workflow = match two_step_workflow(ConstValue::I64(42)) {
            Ok(wf) => wf,
            Err(_) => { proptest::prop_assert!(false, "workflow creation failed"); return; }
        };
        let mut run = match test_frame(RunId::new(200), &workflow) {
            Ok(r) => r,
            Err(_) => { proptest::prop_assert!(false, "frame creation failed"); return; }
        };
        let mut store = test_store();
        let mut budget = StepBudget::new(budget_value);

        let _ = drive_deterministic(&workflow, &mut run, &mut budget, &mut store);
    }

    /// run_until_blocked never panics for any budget value on a two-step workflow.
    #[test]
    fn prop_run_until_blocked_no_panic_for_any_budget(
        budget_value in 0u64..(MAX_STEP_BUDGET + 100),
    ) {
        let workflow = match two_step_workflow(ConstValue::I64(42)) {
            Ok(wf) => wf,
            Err(_) => { proptest::prop_assert!(false, "workflow creation failed"); return; }
        };
        let mut run = match test_frame(RunId::new(300), &workflow) {
            Ok(r) => r,
            Err(_) => { proptest::prop_assert!(false, "frame creation failed"); return; }
        };
        let mut store = test_store();

        let _ = run_until_blocked(&workflow, &mut run, StepBudget::new(budget_value), &mut store);
    }

    /// Sequential step_once on a larger workflow matches bulk execution.
    #[test]
    fn prop_step_by_step_matches_bulk_on_four_step_chain(
        v0 in proptest::prelude::any::<i64>(),
        v1 in proptest::prelude::any::<i64>(),
        v2 in proptest::prelude::any::<i64>(),
    ) {
        let workflow = match four_step_set_const_chain_workflow(&[
            ConstValue::I64(v0),
            ConstValue::I64(v1),
            ConstValue::I64(v2),
        ]) {
            Ok(wf) => wf,
            Err(_) => { proptest::prop_assert!(false, "workflow creation failed"); return; }
        };

        let mut run_bulk = match test_frame(RunId::new(400), &workflow) {
            Ok(r) => r,
            Err(_) => { proptest::prop_assert!(false, "bulk frame creation failed"); return; }
        };
        let mut store_bulk = test_store();

        let bulk_result = match run_until_blocked(
            &workflow,
            &mut run_bulk,
            StepBudget::MAX,
            &mut store_bulk,
        ) {
            Ok(sig) => sig,
            Err(_) => { proptest::prop_assert!(false, "bulk execution failed"); return; }
        };

        let mut run_step = match test_frame(RunId::new(401), &workflow) {
            Ok(r) => r,
            Err(_) => { proptest::prop_assert!(false, "step frame creation failed"); return; }
        };
        let mut store_step = test_store();

        let mut terminal_found = false;
        for _ in 0..4 {
            let sig = match step_once(&workflow, &mut run_step, &mut store_step) {
                Ok(sig) => sig,
                Err(_) => { proptest::prop_assert!(false, "step_once failed"); return; }
            };
            match sig {
                EngineSignal::Continue => {
                    // no-op: continue stepping
                }
                terminal => {
                    proptest::prop_assert_eq!(terminal, bulk_result);
                    terminal_found = true;
                    break;
                }
            }
        }
        proptest::prop_assert!(terminal_found, "expected terminal signal within 4 steps");
    }
}
