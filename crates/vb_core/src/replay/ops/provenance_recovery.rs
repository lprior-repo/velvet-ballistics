use super::*;
use crate::ids::{RunId, WorkflowDigest};
use crate::workflow::{ResourceContract, WorkflowParts, node};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn plan() -> Result<CompiledWorkflow, crate::WorkflowError> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: "replay_error_pc".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::new([
            node(
                StepIdx::ZERO,
                crate::CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            node(StepIdx::new(1), crate::CompiledNodeKind::Nop, None, None),
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
}

fn stack() -> Result<ReplayExprStack, String> {
    ReplayExprStack::new(4).map_err(|error| format!("{error:?}"))
}

#[test]
fn numeric_failures_report_actual_nonzero_and_zero_frame_pc() -> TestResult {
    let plan = plan()?;
    for pc in [StepIdx::new(1), StepIdx::ZERO] {
        for (op, operands) in [
            (ExprOp::Div, [SlotValue::I64(1), SlotValue::I64(0)]),
            (ExprOp::Add, [SlotValue::I64(i64::MAX), SlotValue::I64(1)]),
            (ExprOp::Add, [SlotValue::I64(1), SlotValue::Bool(true)]),
        ] {
            let run = RunFrame::new(RunId::new(1), pc, 2, 2)?;
            let mut store = ValueStore::new();
            let mut stack = stack()?;
            for value in operands {
                stack.push(value).map_err(|error| format!("{error:?}"))?;
            }
            let mut taint = Taint::Secret;
            assert_eq!(
                eval_replay_op(&plan, &run, &mut store, op, &mut stack, &mut taint),
                Err(ReplayError::ExpressionEvalFailed { step: pc })
            );
            assert_eq!(taint, Taint::Secret);
        }
    }
    Ok(())
}

#[test]
fn empty_stack_failure_uses_current_step() -> TestResult {
    let plan = plan()?;
    let run = RunFrame::new(RunId::new(1), StepIdx::new(1), 2, 2)?;
    let mut store = ValueStore::new();
    let mut stack = stack()?;
    let mut taint = Taint::Clean;
    assert_eq!(
        eval_replay_op(&plan, &run, &mut store, ExprOp::Not, &mut stack, &mut taint),
        Err(ReplayError::ExpressionEvalFailed {
            step: StepIdx::new(1)
        })
    );
    Ok(())
}

#[test]
fn load_failures_keep_their_slot_and_internal_reason() -> TestResult {
    let plan = plan()?;
    let run = RunFrame::new(RunId::new(1), StepIdx::new(1), 2, 2)?;
    let mut store = ValueStore::new();
    let mut stack = stack()?;
    let mut taint = Taint::Clean;
    for slot in [SlotIdx::ZERO, SlotIdx::new(u16::MAX)] {
        assert_eq!(
            eval_replay_op(
                &plan,
                &run,
                &mut store,
                ExprOp::LoadSlot(slot),
                &mut stack,
                &mut taint
            ),
            Err(ReplayError::SlotNotAvailable { slot })
        );
    }
    assert_eq!(
        eval_replay_op(
            &plan,
            &run,
            &mut store,
            ExprOp::LoadConst(ConstIdx::new(0)),
            &mut stack,
            &mut taint
        ),
        Err(ReplayError::Internal {
            reason: "constant out of bounds"
        })
    );
    assert_eq!(stack.len, 0);
    assert_eq!(taint, Taint::Clean);
    Ok(())
}

#[test]
fn successful_arithmetic_preserves_value_and_operand_taint() -> TestResult {
    let plan = plan()?;
    let run = RunFrame::new(RunId::new(1), StepIdx::new(1), 2, 2)?;
    let mut store = ValueStore::new();
    let mut stack = stack()?;
    stack
        .push_tainted(SlotValue::I64(2), Taint::Secret)
        .map_err(|e| format!("{e:?}"))?;
    stack
        .push(SlotValue::I64(3))
        .map_err(|e| format!("{e:?}"))?;
    let mut taint = Taint::Clean;
    assert_eq!(
        eval_replay_op(&plan, &run, &mut store, ExprOp::Add, &mut stack, &mut taint),
        Ok(())
    );
    assert_eq!(stack.pop_tainted(), Ok((SlotValue::I64(5), Taint::Secret)));
    assert_eq!(taint, Taint::Secret);
    Ok(())
}

#[test]
fn failed_operation_cleans_pending_taint_before_following_evaluation() -> TestResult {
    let plan = plan()?;
    let run = RunFrame::new(RunId::new(1), StepIdx::new(1), 2, 2)?;
    let mut store = ValueStore::new();
    let mut stack = stack()?;
    stack
        .push_tainted(SlotValue::I64(1), Taint::Secret)
        .map_err(|e| format!("{e:?}"))?;
    stack
        .push(SlotValue::Bool(false))
        .map_err(|e| format!("{e:?}"))?;
    let mut taint = Taint::Clean;
    assert_eq!(
        eval_replay_op(&plan, &run, &mut store, ExprOp::Add, &mut stack, &mut taint),
        Err(ReplayError::ExpressionEvalFailed {
            step: StepIdx::new(1)
        })
    );
    assert_eq!(stack.len, 0);
    stack
        .push(SlotValue::Bool(true))
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        eval_replay_op(&plan, &run, &mut store, ExprOp::Not, &mut stack, &mut taint),
        Ok(())
    );
    assert_eq!(
        stack.pop_tainted(),
        Ok((SlotValue::Bool(false), Taint::Clean))
    );
    assert_eq!(taint, Taint::Clean);
    Ok(())
}
