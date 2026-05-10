#![forbid(unsafe_code)]
//! Deterministic run loop and step budget execution.

use crate::EngineSignal;
use crate::StepBudget;
use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::value_store::ValueStore;
use crate::workflow::CompiledWorkflow;

/// Executes deterministic nodes until finish or budget exhaustion.
pub fn run_until_blocked(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    mut budget: StepBudget,
    store: &mut ValueStore,
) -> Result<EngineSignal, EngineError> {
    drive_deterministic(plan, run, &mut budget, store)
}

/// Executes deterministic nodes until finish, suspension, or budget exhaustion.
pub fn drive_deterministic(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    budget: &mut StepBudget,
    store: &mut ValueStore,
) -> Result<EngineSignal, EngineError> {
    while budget.try_take()? {
        let signal = super::step::step_once(plan, run, store)?;
        if !matches!(signal, EngineSignal::Continue) {
            return Ok(signal);
        }
    }
    Ok(EngineSignal::StepBudgetExhausted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EngineSignal;
    use crate::ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
    use crate::value::{ConstValue, SlotValue, Taint};
    use crate::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowParts};

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

    fn two_step_workflow(value: ConstValue) -> Result<CompiledWorkflow, String> {
        CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("run_loop_test"),
            digest: WorkflowDigest::from_bytes([0x99; 32]),
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

    fn test_frame(workflow: &CompiledWorkflow) -> Result<RunFrame, String> {
        RunFrame::new(
            RunId::new(1),
            workflow.entry(),
            workflow.node_count(),
            workflow.slot_count(),
        )
        .map_err(|e| e.to_string())
    }

    #[test]
    fn run_until_blocked_completes_two_step_workflow() -> Result<(), String> {
        let workflow = two_step_workflow(ConstValue::I64(42))?;
        let mut run = test_frame(&workflow)?;
        let mut store = ValueStore::new();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
            .map_err(|e| e.to_string())?;

        ensure_equal(
            result,
            EngineSignal::Finished(SlotValue::I64(42), Taint::Clean),
        )?;
        ensure_equal(run.executed(), 2)
    }

    #[test]
    fn run_until_blocked_exhausts_zero_budget() -> Result<(), String> {
        let workflow = two_step_workflow(ConstValue::I64(1))?;
        let mut run = test_frame(&workflow)?;
        let mut store = ValueStore::new();

        let result = run_until_blocked(&workflow, &mut run, StepBudget::new(0), &mut store)
            .map_err(|e| e.to_string())?;

        ensure_equal(result, EngineSignal::StepBudgetExhausted)?;
        ensure_equal(run.executed(), 0)
    }

    #[test]
    fn drive_deterministic_with_one_budget_stops_after_first_step() -> Result<(), String> {
        let workflow = two_step_workflow(ConstValue::I64(7))?;
        let mut run = test_frame(&workflow)?;
        let mut store = ValueStore::new();
        let mut budget = StepBudget::new(1);

        let result = drive_deterministic(&workflow, &mut run, &mut budget, &mut store)
            .map_err(|e| e.to_string())?;

        ensure_equal(result, EngineSignal::StepBudgetExhausted)?;
        ensure_equal(run.executed(), 1)?;
        ensure_equal(run.pc(), StepIdx::new(1))
    }

    #[test]
    fn drive_deterministic_exact_budget_completes() -> Result<(), String> {
        let workflow = two_step_workflow(ConstValue::I64(33))?;
        let mut run = test_frame(&workflow)?;
        let mut store = ValueStore::new();
        let mut budget = StepBudget::new(2);

        let result = drive_deterministic(&workflow, &mut run, &mut budget, &mut store)
            .map_err(|e| e.to_string())?;

        ensure_equal(
            result,
            EngineSignal::Finished(SlotValue::I64(33), Taint::Clean),
        )?;
        ensure_equal(run.executed(), 2)
    }

    #[test]
    fn drive_deterministic_stops_on_do_node_suspension() -> Result<(), String> {
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("do_suspend"),
            digest: WorkflowDigest::from_bytes([0xDD; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: crate::ids::ActionId::new(1),
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
            resource_contract: crate::ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .map_err(|e| e.to_string())?;
        let mut run = test_frame(&workflow)?;
        let mut store = ValueStore::new();

        let mut budget = StepBudget::MAX;
        let result = drive_deterministic(&workflow, &mut run, &mut budget, &mut store)
            .map_err(|e| e.to_string())?;

        ensure_equal(result, EngineSignal::AwaitingAction)
    }
}
