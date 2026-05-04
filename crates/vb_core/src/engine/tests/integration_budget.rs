//! Integration tests for step budget handling.

use crate::errors::EngineError;
use crate::frame::StepState;
use crate::ids::{RunId, SlotIdx, StepIdx, WorkflowDigest};
use crate::value::{ConstValue, SlotValue, Taint};
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

use crate::engine::{EngineSignal, StepBudget, new_run_frame, run_until_blocked, step_once};

fn test_store() -> crate::value_store::ValueStore {
    crate::value_store::ValueStore::new()
}

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

fn test_frame(run_id: RunId, workflow: &CompiledWorkflow) -> Result<crate::RunFrame, String> {
    new_run_frame(run_id, workflow).map_err(|error| error.to_string())
}

fn tiny_workflow(value: ConstValue) -> Result<CompiledWorkflow, crate::WorkflowError> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("tiny"),
        digest: WorkflowDigest::from_bytes([1; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: crate::ids::ConstIdx::new(0),
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
}

#[test]
fn zero_budget_exhausts_without_execution() -> Result<(), String> {
    let workflow = tiny_workflow(ConstValue::I64(42)).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(7), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::new(0), &mut store);

    ensure_equal(result, Ok(EngineSignal::StepBudgetExhausted))?;
    ensure_equal(run.executed(), 0)?;
    ensure_equal(run.pc(), StepIdx::new(0))?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Pending))?;
    Ok(())
}

#[test]
fn step_budget_try_take_consumes_exactly_one_transition() -> Result<(), String> {
    let mut budget = StepBudget::new(1);

    ensure_equal(budget.try_take().map_err(|error| error.to_string())?, true)?;
    ensure_equal(budget.remaining(), 0)?;
    ensure_equal(budget.try_take().map_err(|error| error.to_string())?, false)?;
    ensure_equal(budget.remaining(), 0)?;
    Ok(())
}

#[test]
fn one_budget_executes_one_transition_and_exhausts() -> Result<(), String> {
    let workflow = tiny_workflow(ConstValue::I64(42)).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(17), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::new(1), &mut store);

    ensure_equal(result, Ok(EngineSignal::StepBudgetExhausted))?;
    ensure_equal(run.executed(), 1)?;
    ensure_equal(run.pc(), StepIdx::new(1))?;
    ensure_equal(run.read_slot(SlotIdx::new(0)), Ok(&SlotValue::I64(42)))?;
    Ok(())
}

#[test]
fn budget_zero_drive_deterministic_returns_step_budget_exhausted_without_touching_frame()
-> Result<(), String> {
    let workflow = tiny_workflow(ConstValue::I64(1)).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(100), &workflow)?;
    let mut store = test_store();
    let initial_executed = run.executed();
    let initial_pc = run.pc();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::new(0), &mut store);

    ensure_equal(result, Ok(EngineSignal::StepBudgetExhausted))?;
    ensure_equal(run.executed(), initial_executed)?;
    ensure_equal(run.pc(), initial_pc)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Pending))?;
    Ok(())
}

#[test]
fn budget_one_executes_exactly_one_transition_then_exhausts() -> Result<(), String> {
    let workflow = tiny_workflow(ConstValue::I64(7)).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(101), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::new(1), &mut store);

    ensure_equal(result, Ok(EngineSignal::StepBudgetExhausted))?;
    ensure_equal(run.executed(), 1)?;
    ensure_equal(run.pc(), StepIdx::new(1))?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
    ensure_equal(run.step_state(StepIdx::new(1)), Ok(StepState::Pending))?;
    Ok(())
}

#[test]
fn budget_two_completes_two_step_workflow_with_finish() -> Result<(), String> {
    let workflow = tiny_workflow(ConstValue::I64(55)).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(102), &workflow)?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::new(2), &mut store);

    ensure_equal(
        result,
        Ok(EngineSignal::Finished(SlotValue::I64(55), Taint::Clean)),
    )?;
    ensure_equal(run.executed(), 2)?;
    Ok(())
}

#[test]
fn step_budget_try_take_returns_false_after_depletion_without_error() -> Result<(), String> {
    let mut budget = StepBudget::new(0);
    let first = budget.try_take().map_err(|error| error.to_string())?;
    ensure_equal(first, false)?;
    ensure_equal(budget.remaining(), 0)?;

    let mut budget_one = StepBudget::new(1);
    let take1 = budget_one.try_take().map_err(|error| error.to_string())?;
    ensure_equal(take1, true)?;
    ensure_equal(budget_one.remaining(), 0)?;
    let take2 = budget_one.try_take().map_err(|error| error.to_string())?;
    ensure_equal(take2, false)?;
    ensure_equal(budget_one.remaining(), 0)?;
    Ok(())
}

#[test]
fn step_budget_max_does_not_overflow_on_consecutive_takes() -> Result<(), String> {
    let mut budget = StepBudget::MAX;
    ensure_equal(budget.remaining(), crate::limits::MAX_STEP_BUDGET)?;
    let take = budget.try_take().map_err(|error| error.to_string())?;
    ensure_equal(take, true)?;
    ensure_equal(budget.remaining(), crate::limits::MAX_STEP_BUDGET - 1)?;
    Ok(())
}
