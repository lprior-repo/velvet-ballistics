#![forbid(unsafe_code)]
//! Step budget suspension tests.
//!
//! These tests convert fallible helpers into `expect`-style failures so the
//! function signatures do not need to return `Result`. Per repository policy
//! (AGENTS.md: "Tests must compile and run, but test clippy is not strict"),
//! `clippy::expect_used` is allowed in this test target.
#![allow(clippy::expect_used)]

use proptest::prelude::*;
use vb_core::engine::{EngineSignal, StepBudget, run_until_blocked};
use vb_core::frame::{RunFrame, StepState};
use vb_core::ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::limits::MAX_STEP_BUDGET;
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowParts};

fn workflow_with_const_then_finish(value: ConstValue) -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("vb_5m8w_step_budget_suspension"),
        digest: WorkflowDigest::from_bytes([0x5d; 32]),
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
                output: Some(SlotIdx::new(0)),
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
        resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|error| error.to_string())
}

fn new_frame(workflow: &CompiledWorkflow) -> Result<RunFrame, String> {
    RunFrame::new(
        RunId::new(508),
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|error| error.to_string())
}

#[test]
fn given_budget_above_max_when_constructed_then_clamped_to_max_step_budget() {
    let requested = MAX_STEP_BUDGET.saturating_add(1);

    let budget = StepBudget::new(requested);

    assert_eq!(budget.remaining(), MAX_STEP_BUDGET);
}

#[test]
fn given_u64_max_budget_when_constructed_then_clamped_to_max_step_budget() {
    let budget = StepBudget::new(u64::MAX);

    assert_eq!(budget.remaining(), MAX_STEP_BUDGET);
}

#[test]
fn given_zero_budget_when_try_take_called_then_returns_false_without_mutation() {
    let mut budget = StepBudget::new(0);
    let before_remaining = budget.remaining();

    let taken = budget
        .try_take()
        .expect("try_take on a zero budget should not error");

    assert!(!taken);
    assert_eq!(budget.remaining(), before_remaining);
}

#[test]
fn given_positive_budget_when_try_take_called_then_remaining_decrements_by_one() {
    let mut budget = StepBudget::new(MAX_STEP_BUDGET);

    let taken = budget
        .try_take()
        .expect("try_take on a positive budget should not error");

    assert!(taken);
    assert_eq!(budget.remaining(), MAX_STEP_BUDGET - 1);
}

#[test]
fn given_try_take_repeated_after_zero_then_budget_does_not_underflow() {
    let mut budget = StepBudget::new(0);

    for attempt in 0..=1024u16 {
        let taken = budget
            .try_take()
            .expect("try_take on a zero budget should not error");
        assert!(!taken, "attempt {attempt} must report exhausted");
        assert_eq!(
            budget.remaining(),
            0,
            "attempt {attempt} must preserve zero"
        );
    }
}

#[test]
fn given_zero_budget_when_run_until_blocked_then_signal_is_step_budget_exhausted_not_finished_or_error()
 {
    let workflow = workflow_with_const_then_finish(ConstValue::I64(99))
        .expect("WorkflowParts must construct successfully");
    let mut run = new_frame(&workflow).expect("RunFrame must construct successfully");
    let mut store = ValueStore::new();

    let signal = run_until_blocked(&workflow, &mut run, StepBudget::new(0), &mut store)
        .expect("run_until_blocked must not error on a zero budget");

    assert_eq!(signal, EngineSignal::StepBudgetExhausted);
    assert_eq!(run.pc(), StepIdx::new(0));
    assert_eq!(run.executed(), 0);
    assert_eq!(run.step_state(StepIdx::new(0)), Ok(StepState::Pending));
    assert_eq!(
        run.read_slot(SlotIdx::new(0)).copied(),
        Err(vb_core::CoreError::SlotUninitialized {
            slot: SlotIdx::new(0)
        })
    );
}

#[test]
fn given_one_step_completed_when_next_budget_exhausts_then_completed_step_remains_succeeded() {
    let workflow = workflow_with_const_then_finish(ConstValue::I64(42))
        .expect("WorkflowParts must construct successfully");
    let mut run = new_frame(&workflow).expect("RunFrame must construct successfully");
    let mut store = ValueStore::new();

    let first_signal = run_until_blocked(&workflow, &mut run, StepBudget::new(1), &mut store)
        .expect("run_until_blocked must not error on a budget of 1");
    assert_eq!(first_signal, EngineSignal::StepBudgetExhausted);
    assert_eq!(run.pc(), StepIdx::new(1));
    assert_eq!(run.executed(), 1);
    assert_eq!(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded));
    assert_eq!(
        run.read_slot(SlotIdx::new(0)).copied(),
        Ok(SlotValue::I64(42))
    );

    let second_signal = run_until_blocked(&workflow, &mut run, StepBudget::new(0), &mut store)
        .expect("run_until_blocked must not error on a zero budget");

    assert_eq!(second_signal, EngineSignal::StepBudgetExhausted);
    assert_eq!(run.pc(), StepIdx::new(1));
    assert_eq!(run.executed(), 1);
    assert_eq!(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded));
    assert_eq!(run.step_state(StepIdx::new(1)), Ok(StepState::Pending));
    assert_eq!(
        run.read_slot(SlotIdx::new(0)).copied(),
        Ok(SlotValue::I64(42))
    );
}

#[test]
fn given_budget_suspended_run_when_fresh_budget_scheduled_then_run_resumes_from_same_pc() {
    let workflow = workflow_with_const_then_finish(ConstValue::I64(7))
        .expect("WorkflowParts must construct successfully");
    let mut run = new_frame(&workflow).expect("RunFrame must construct successfully");
    let mut store = ValueStore::new();

    let first_signal = run_until_blocked(&workflow, &mut run, StepBudget::new(1), &mut store)
        .expect("run_until_blocked must not error on a budget of 1");
    assert_eq!(first_signal, EngineSignal::StepBudgetExhausted);
    assert_eq!(run.pc(), StepIdx::new(1));

    let resumed_signal = run_until_blocked(&workflow, &mut run, StepBudget::new(1), &mut store)
        .expect("run_until_blocked must not error on a budget of 1");

    assert_eq!(
        resumed_signal,
        EngineSignal::Finished(SlotValue::I64(7), Taint::Clean)
    );
    assert_eq!(run.pc(), StepIdx::new(1));
    assert_eq!(run.executed(), 2);
    assert_eq!(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded));
    assert_eq!(run.step_state(StepIdx::new(1)), Ok(StepState::Succeeded));
}

proptest! {
    #[test]
    fn proptest_step_budget_new_clamps_any_u64_to_max_step_budget(requested in any::<u64>()) {
        let budget = StepBudget::new(requested);
        let expected = requested.min(MAX_STEP_BUDGET);

        prop_assert_eq!(budget.remaining(), expected);
    }

    #[test]
    fn proptest_positive_try_take_decrements_exactly_once(requested in 1u64..=MAX_STEP_BUDGET) {
        let mut budget = StepBudget::new(requested);

        let taken = budget.try_take().map_err(|error| TestCaseError::fail(error.to_string()))?;

        prop_assert!(taken);
        // requested is in 1u64..=MAX_STEP_BUDGET, so saturating_sub yields requested-1.
        prop_assert_eq!(budget.remaining(), requested.saturating_sub(1));
    }

    #[test]
    fn proptest_zero_try_take_repetition_preserves_zero(repetitions in 0u16..=1024u16) {
        let mut budget = StepBudget::new(0);

        for _ in 0..repetitions {
            let taken = budget.try_take().map_err(|error| TestCaseError::fail(error.to_string()))?;
            prop_assert!(!taken);
            prop_assert_eq!(budget.remaining(), 0);
        }
        prop_assert_eq!(budget.remaining(), 0);
    }
}
