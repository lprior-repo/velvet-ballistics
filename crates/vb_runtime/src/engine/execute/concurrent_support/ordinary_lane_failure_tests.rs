#![forbid(unsafe_code)]

use super::ordinary_lane_failure;
use crate::engine::types::RuntimeEngineError;
use vb_core::action::ActionError;
use vb_core::errors::EngineError;
use vb_core::ids::{ConstIdx, ExprIdx, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    CompiledNodeKind as Kind, CompiledWorkflow, ExprOp, ExprProgram, ResourceContract,
    WorkflowParts, node,
};

fn checked_first_success_eval_plan() -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    let first_success = node(
        StepIdx::new(0),
        Kind::FirstSuccessStart {
            branches: Box::new([StepIdx::new(1), StepIdx::new(3)]),
            join: StepIdx::new(5),
        },
        None,
        None,
    );
    let first_eval = node(
        StepIdx::new(1),
        Kind::EvalExpr {
            expr: ExprIdx::new(0),
        },
        Some(SlotIdx::new(0)),
        Some(StepIdx::new(2)),
    );
    let first_lane_end = node(
        StepIdx::new(2),
        Kind::ConcurrentLaneEnd {
            owner: StepIdx::new(0),
            result: Some(SlotIdx::new(0)),
        },
        None,
        Some(StepIdx::new(5)),
    );
    let second_eval = node(
        StepIdx::new(3),
        Kind::EvalExpr {
            expr: ExprIdx::new(0),
        },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(4)),
    );
    let second_lane_end = node(
        StepIdx::new(4),
        Kind::ConcurrentLaneEnd {
            owner: StepIdx::new(0),
            result: Some(SlotIdx::new(1)),
        },
        None,
        Some(StepIdx::new(5)),
    );
    let join = node(
        StepIdx::new(5),
        Kind::ConcurrentGroupJoin {
            owner: StepIdx::new(0),
        },
        Some(SlotIdx::new(2)),
        Some(StepIdx::new(6)),
    );
    let finish = node(
        StepIdx::new(6),
        Kind::Finish {
            result: SlotIdx::new(2),
        },
        None,
        None,
    );
    let expression = ExprProgram::try_from_ops(Box::new([
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Div,
    ]))?;
    let parts = WorkflowParts {
        name: "first_success_eval".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: Box::new([
            first_success,
            first_eval,
            first_lane_end,
            second_eval,
            second_lane_end,
            join,
            finish,
        ]),
        expressions: Box::new([expression]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(1), ConstValue::I64(0)]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),
    };

    Ok(CompiledWorkflow::try_from_concurrent_parts(parts)?)
}

#[test]
fn division_by_zero_at_checked_first_success_eval_is_an_ordinary_lane_failure()
-> Result<(), Box<dyn std::error::Error>> {
    let plan = checked_first_success_eval_plan()?;
    assert_eq!(
        plan.ir_profile(),
        vb_core::workflow::WorkflowIrProfile::ConcurrentV3
    );
    assert!(ordinary_lane_failure(
        &plan,
        StepIdx::new(1),
        &RuntimeEngineError::Core(EngineError::DivisionByZero),
    ));
    Ok(())
}

#[test]
fn action_error_at_checked_first_success_eval_is_fatal() -> Result<(), Box<dyn std::error::Error>> {
    let plan = checked_first_success_eval_plan()?;
    assert!(!ordinary_lane_failure(
        &plan,
        StepIdx::new(1),
        &RuntimeEngineError::Action(ActionError::DispatchFailed),
    ));
    Ok(())
}

#[test]
fn taint_violation_at_checked_first_success_eval_is_fatal() -> Result<(), Box<dyn std::error::Error>>
{
    let plan = checked_first_success_eval_plan()?;
    assert!(!ordinary_lane_failure(
        &plan,
        StepIdx::new(1),
        &RuntimeEngineError::TaintViolation {
            step: StepIdx::new(1),
        },
    ));
    Ok(())
}

#[test]
fn non_ordinary_core_error_at_checked_first_success_eval_is_fatal()
-> Result<(), Box<dyn std::error::Error>> {
    let plan = checked_first_success_eval_plan()?;
    assert!(!ordinary_lane_failure(
        &plan,
        StepIdx::new(1),
        &RuntimeEngineError::Core(EngineError::StepBudgetExhausted),
    ));
    Ok(())
}

#[test]
fn division_by_zero_outside_eval_node_is_fatal() -> Result<(), Box<dyn std::error::Error>> {
    let plan = checked_first_success_eval_plan()?;
    assert!(!ordinary_lane_failure(
        &plan,
        StepIdx::new(0),
        &RuntimeEngineError::Core(EngineError::DivisionByZero),
    ));
    Ok(())
}
