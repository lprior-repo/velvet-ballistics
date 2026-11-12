//! Differential test for VB-REF-001: the reference engine model against the
//! production deterministic drive over the same workflow, frame, and budget.
//!
//! The two models agree exactly where both are defined: budget accounting,
//! budget exhaustion, and the step-state transition table. They diverge in
//! two pinned places, and this test asserts both sides of each divergence
//! rather than claiming an agreement that does not exist:
//!
//! - Completion: the reference model never executes nodes, so on a workflow
//!   the production drive finishes, the reference drive exhausts the budget.
//!   Only the budget-exhaustion signal agrees.
//! - Terminal-marked step zero: the reference model returns a finished
//!   signal for a frame whose first step is already terminal, while the
//!   production drive rejects the re-mark as an invalid transition.

use crate::engine_model::EngineModel;
use crate::step_state_model::is_valid_transition;
use vb_core::engine::{drive_deterministic, new_run_frame};
use vb_core::frame::{RunFrame, StepState, is_valid_step_state_transition};
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::workflow::{
    CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts, node,
};
use vb_core::{
    ConstIdx, CoreResult, EngineError, EngineSignal, RunId, SlotIdx, StepBudget, StepIdx,
    ValueStore, WorkflowDigest,
};

fn two_step_workflow(value: ConstValue) -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("engine_vs_reference"),
        digest: WorkflowDigest::from_bytes([0x42; 32]),
        nodes: vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![value].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|error| error.to_string())
}

fn drive_production(
    workflow: &CompiledWorkflow,
    frame: &mut RunFrame,
    budget: u64,
) -> Result<EngineSignal, EngineError> {
    let mut step_budget = StepBudget::new(budget);
    let mut store = ValueStore::new();
    drive_deterministic(workflow, frame, &mut step_budget, &mut store)
}

fn drive_reference(frame: &mut RunFrame, budget: u64) -> CoreResult<EngineSignal> {
    let mut store = ValueStore::new();
    EngineModel::drive_deterministic_ref(frame, &mut store, budget)
}

fn fresh_frame(workflow: &CompiledWorkflow) -> Result<RunFrame, String> {
    new_run_frame(RunId::new(1), workflow).map_err(|error| error.to_string())
}

fn terminal_frame(workflow: &CompiledWorkflow, state: StepState) -> Result<RunFrame, String> {
    let mut frame = fresh_frame(workflow)?;
    let marked = match state {
        StepState::Succeeded => frame.mark_succeeded(StepIdx::ZERO),
        StepState::Failed => frame.mark_failed(StepIdx::ZERO),
        StepState::Cancelled => frame.mark_cancelled(StepIdx::ZERO),
        StepState::Skipped => frame.mark_skipped(StepIdx::ZERO),
        other => return Err(format!("{other:?} is not a terminal state")),
    };
    marked.map_err(|error| error.to_string())?;
    Ok(frame)
}

fn assert_budget_zero_agrees(workflow: &CompiledWorkflow) -> Result<(), String> {
    let mut prod_frame = fresh_frame(workflow)?;
    let mut ref_frame = fresh_frame(workflow)?;
    let prod = drive_production(workflow, &mut prod_frame, 0);
    let reference = drive_reference(&mut ref_frame, 0);
    match prod {
        Ok(signal) => assert_eq!(signal, EngineSignal::StepBudgetExhausted),
        Err(error) => return Err(format!("production drive failed: {error}")),
    }
    match reference {
        Ok(signal) => assert_eq!(signal, EngineSignal::StepBudgetExhausted),
        Err(error) => return Err(format!("reference drive failed: {error}")),
    }
    assert_eq!(prod_frame.executed(), 0);
    assert_eq!(ref_frame.executed(), 0);
    assert_eq!(prod_frame.pc(), StepIdx::ZERO);
    assert_eq!(ref_frame.pc(), StepIdx::ZERO);
    Ok(())
}

fn assert_budget_exhaustion_agrees(workflow: &CompiledWorkflow) -> Result<(), String> {
    let mut prod_frame = fresh_frame(workflow)?;
    let mut ref_frame = fresh_frame(workflow)?;
    let prod = drive_production(workflow, &mut prod_frame, 1);
    let reference = drive_reference(&mut ref_frame, 1);
    match prod {
        Ok(signal) => assert_eq!(signal, EngineSignal::StepBudgetExhausted),
        Err(error) => return Err(format!("production drive failed: {error}")),
    }
    match reference {
        Ok(signal) => assert_eq!(signal, EngineSignal::StepBudgetExhausted),
        Err(error) => return Err(format!("reference drive failed: {error}")),
    }
    assert_eq!(prod_frame.executed(), 1);
    assert_eq!(prod_frame.pc(), StepIdx::new(1));
    assert_eq!(ref_frame.executed(), 0);
    assert_eq!(ref_frame.pc(), StepIdx::ZERO);
    Ok(())
}

fn assert_completion_divergence(workflow: &CompiledWorkflow) -> Result<(), String> {
    for budget in [2, 10] {
        let mut prod_frame = fresh_frame(workflow)?;
        let mut ref_frame = fresh_frame(workflow)?;
        let prod = drive_production(workflow, &mut prod_frame, budget);
        let reference = drive_reference(&mut ref_frame, budget);
        match prod {
            Ok(signal) => assert_eq!(
                signal,
                EngineSignal::Finished(SlotValue::I64(42), Taint::Clean)
            ),
            Err(error) => return Err(format!("production drive failed: {error}")),
        }
        match reference {
            Ok(signal) => assert_eq!(signal, EngineSignal::StepBudgetExhausted),
            Err(error) => return Err(format!("reference drive failed: {error}")),
        }
        assert_eq!(prod_frame.executed(), 2);
        assert_eq!(ref_frame.executed(), 0);
    }
    Ok(())
}

fn assert_terminal_frame_divergence(workflow: &CompiledWorkflow) -> Result<(), String> {
    for state in [
        StepState::Succeeded,
        StepState::Failed,
        StepState::Cancelled,
        StepState::Skipped,
    ] {
        let mut prod_frame = terminal_frame(workflow, state)?;
        let mut ref_frame = terminal_frame(workflow, state)?;
        let prod = drive_production(workflow, &mut prod_frame, 5);
        let reference = drive_reference(&mut ref_frame, 5);
        match reference {
            Ok(signal) => assert_eq!(
                signal,
                EngineSignal::Finished(SlotValue::Null, Taint::Clean)
            ),
            Err(error) => return Err(format!("reference drive failed: {error}")),
        }
        match prod {
            Err(EngineError::InternalInvariantViolation { reason }) => {
                assert_eq!(reason, vb_core::InvariantKind::from("invalid_state_transition"));
            }
            other => {
                return Err(format!(
                    "terminal {state:?}: expected invalid_state_transition, got {other:?}"
                ));
            }
        }
    }
    Ok(())
}

fn assert_budget_zero_terminal_agrees(workflow: &CompiledWorkflow) -> Result<(), String> {
    for state in [
        StepState::Succeeded,
        StepState::Failed,
        StepState::Cancelled,
        StepState::Skipped,
    ] {
        let mut prod_frame = terminal_frame(workflow, state)?;
        let mut ref_frame = terminal_frame(workflow, state)?;
        let prod = drive_production(workflow, &mut prod_frame, 0);
        let reference = drive_reference(&mut ref_frame, 0);
        match prod {
            Ok(signal) => assert_eq!(signal, EngineSignal::StepBudgetExhausted),
            Err(error) => return Err(format!("production drive failed: {error}")),
        }
        match reference {
            Ok(signal) => assert_eq!(signal, EngineSignal::StepBudgetExhausted),
            Err(error) => return Err(format!("reference drive failed: {error}")),
        }
    }
    Ok(())
}

#[test]
fn engine_vs_reference() -> Result<(), String> {
    let workflow = two_step_workflow(ConstValue::I64(42))?;
    assert_budget_zero_agrees(&workflow)?;
    assert_budget_exhaustion_agrees(&workflow)?;
    assert_completion_divergence(&workflow)?;
    assert_terminal_frame_divergence(&workflow)?;
    assert_budget_zero_terminal_agrees(&workflow)?;
    Ok(())
}

#[test]
fn engine_vs_reference_budget_accounting() -> Result<(), String> {
    for initial in 0..=5u64 {
        let mut prod_budget = StepBudget::new(initial);
        let mut ref_budget = StepBudget::new(initial);
        for _ in 0..=initial {
            let prod_take = prod_budget.try_take().map_err(|error| error.to_string())?;
            let (ref_take, ref_remaining) = EngineModel::step_budget_try_take(&mut ref_budget);
            assert_eq!(prod_take, ref_take, "initial budget {initial}");
            assert_eq!(
                prod_budget.remaining(),
                ref_remaining,
                "initial budget {initial}"
            );
            if !prod_take {
                break;
            }
        }
        assert_eq!(prod_budget.remaining(), 0, "initial budget {initial}");
        assert_eq!(ref_budget.remaining(), 0, "initial budget {initial}");
    }
    Ok(())
}

#[test]
fn engine_vs_reference_transition_table() -> Result<(), String> {
    let states = [
        StepState::Pending,
        StepState::Running,
        StepState::Waiting,
        StepState::Asking,
        StepState::Succeeded,
        StepState::Failed,
        StepState::Cancelled,
        StepState::Skipped,
    ];
    for from in states {
        for to in states {
            let reference = is_valid_transition(from, to);
            let production = is_valid_step_state_transition(from, to);
            if reference != production {
                return Err(format!(
                    "transition {from:?} -> {to:?}: reference {reference}, production {production}"
                ));
            }
        }
    }
    Ok(())
}
