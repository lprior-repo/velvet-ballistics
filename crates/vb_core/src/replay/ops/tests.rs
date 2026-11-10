#[cfg(test)]
#[allow(clippy::panic_in_result_fn)]
use crate::errors::CoreError;
use crate::frame::RunFrame;
use crate::ids::{
    AccessorIdx, ConstIdx, ExprIdx, ListId, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest,
};
use crate::limits::MAX_EXPRESSION_STACK;
use crate::replay::{ReplayError, ReplayExprStack, eval_expr_for_replay};
use crate::value::{ConstValue, FiniteF64, SlotValue, Taint};
use crate::value_store::ValueStore;
use crate::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, PathSegment,
    ResourceContract, WorkflowParts, check_expr_stack_bound, node,
};

use super::{eval_replay_op, pop_i64_pair, pop_pair};

fn make_plan(
    nodes: Vec<CompiledNode>,
    constants: Vec<ConstValue>,
    expressions: Vec<ExprProgram>,
    accessors: Vec<AccessorProgram>,
) -> Result<crate::workflow::CompiledWorkflow, CoreError> {
    crate::workflow::CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: "test_ops".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: nodes.into(),
        expressions: expressions.into(),
        accessors: accessors.into(),
        constants: constants.into(),
        slot_count: 8,
        symbols_count: 1,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|_| CoreError::InvalidCompiledWorkflow {
        reason: "test workflow validation failed",
    })
}

fn make_expr_program(ops: Vec<ExprOp>) -> Result<ExprProgram, CoreError> {
    let max_stack = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK)?;
    ExprProgram::try_from_parts(ops.into(), max_stack)
}

fn make_frame(slot_count: u16) -> Result<RunFrame, CoreError> {
    RunFrame::new(RunId::new(0), StepIdx::new(0), 2, slot_count)
}

fn make_minimal_plan_with_constants(
    constants: Vec<ConstValue>,
) -> Result<crate::workflow::CompiledWorkflow, CoreError> {
    make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
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
        ],
        constants,
        vec![],
        vec![],
    )
}

fn replay_err_to_core(e: ReplayError) -> CoreError {
    match e {
        ReplayError::StepNotFound { step } => CoreError::InvalidProgramCounter { step },
        ReplayError::SlotNotAvailable { slot } => CoreError::SlotOutOfBounds { slot },
        ReplayError::ExpressionEvalFailed { step } => CoreError::InvalidProgramCounter { step },
        ReplayError::NonDeterministicStep { step, .. } => CoreError::InvalidProgramCounter { step },
        ReplayError::Internal { reason } => CoreError::InternalInvariantViolation { reason },
    }
}

// ---- ReplayExprStack lifecycle ----

#[test]
fn expr_stack_push_pop_roundtrip() -> Result<(), CoreError> {
    let mut stack = ReplayExprStack::new(3).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(42)).map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Bool(true))
        .map_err(replay_err_to_core)?;

    let second = stack.pop().map_err(replay_err_to_core)?;
    let first = stack.pop().map_err(replay_err_to_core)?;

    if second != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "second pop should be Bool(true)",
        });
    }
    if first != SlotValue::I64(42) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "first pop should be I64(42)",
        });
    }
    Ok(())
}

#[test]
fn expr_stack_pop_empty_returns_error() -> Result<(), CoreError> {
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let result = stack.pop();
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { step }) if step == StepIdx::ZERO),
        "popping from empty stack must fail with ExpressionEvalFailed"
    );
    Ok(())
}

#[test]
fn expr_stack_push_overflow_returns_error() -> Result<(), CoreError> {
    let mut stack = ReplayExprStack::new(1).map_err(replay_err_to_core)?;
    stack.push(SlotValue::Null).map_err(replay_err_to_core)?;
    let overflow_result = stack.push(SlotValue::Null);
    assert!(
        matches!(overflow_result, Err(ReplayError::ExpressionEvalFailed { step }) if step == StepIdx::ZERO),
        "push overflow must fail with ExpressionEvalFailed"
    );
    Ok(())
}

#[test]
fn expr_stack_max_capacity_boundary() -> Result<(), CoreError> {
    let mut stack = ReplayExprStack::new(MAX_EXPRESSION_STACK).map_err(replay_err_to_core)?;
    let values: [i64; 64] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
        48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
    ];
    values
        .iter()
        .try_for_each(|&i| stack.push(SlotValue::I64(i)).map_err(replay_err_to_core))?;
    let overflow_result = stack.push(SlotValue::Null);
    assert!(
        matches!(overflow_result, Err(ReplayError::ExpressionEvalFailed { step }) if step == StepIdx::ZERO),
        "push beyond capacity must fail with ExpressionEvalFailed"
    );
    Ok(())
}

// ---- pop_pair / pop_i64_pair ----

#[test]
fn pop_pair_returns_left_then_right() -> Result<(), CoreError> {
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(10)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(20)).map_err(replay_err_to_core)?;

    let (left, right) = pop_pair(&mut stack).map_err(replay_err_to_core)?;
    if left != SlotValue::I64(10) || right != SlotValue::I64(20) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "pop_pair order wrong",
        });
    }
    Ok(())
}

#[test]
fn pop_i64_pair_succeeds_for_i64_values() -> Result<(), CoreError> {
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(5)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(3)).map_err(replay_err_to_core)?;

    let (left, right) = pop_i64_pair(&mut stack).map_err(replay_err_to_core)?;
    if left != 5 || right != 3 {
        return Err(CoreError::InternalInvariantViolation {
            reason: "pop_i64_pair values wrong",
        });
    }
    Ok(())
}

#[test]
fn pop_i64_pair_rejects_non_i64_left() -> Result<(), CoreError> {
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Bool(true))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(3)).map_err(replay_err_to_core)?;

    let result = pop_i64_pair(&mut stack);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "non-i64 left operand must fail"
    );
    Ok(())
}

#[test]
fn pop_i64_pair_rejects_non_i64_right() -> Result<(), CoreError> {
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(3)).map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Bool(false))
        .map_err(replay_err_to_core)?;

    let result = pop_i64_pair(&mut stack);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "non-i64 right operand must fail"
    );
    Ok(())
}

#[test]
fn pop_pair_underflow_returns_error() -> Result<(), CoreError> {
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(1)).map_err(replay_err_to_core)?;
    let result = pop_pair(&mut stack);
    assert_eq!(
        result,
        Err(ReplayError::ExpressionEvalFailed {
            step: StepIdx::ZERO
        })
    );
    Ok(())
}

// ---- Arithmetic ops ----

#[test]
fn eval_add_succeeds() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(10)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(7)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Add, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(17) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "10 + 7 should be 17",
        });
    }
    Ok(())
}

#[test]
fn eval_add_overflow_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(i64::MAX))
        .map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::I64(i64::MAX))
        .map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Add, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "i64::MAX + i64::MAX must overflow"
    );
    Ok(())
}

#[test]
fn eval_sub_succeeds() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(100))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(37)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Sub, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(63) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "100 - 37 should be 63",
        });
    }
    Ok(())
}

#[test]
fn eval_sub_underflow_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(i64::MIN))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(1)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Sub, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "i64::MIN - 1 must underflow"
    );
    Ok(())
}

#[test]
fn eval_mul_succeeds() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(6)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(7)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Mul, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(42) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "6 * 7 should be 42",
        });
    }
    Ok(())
}

#[test]
fn eval_mul_overflow_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(i64::MAX))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(2)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Mul, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "i64::MAX * 2 must overflow"
    );
    Ok(())
}

#[test]
fn eval_div_succeeds() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(42)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(6)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Div, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(7) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "42 / 6 should be 7",
        });
    }
    Ok(())
}

#[test]
fn eval_div_by_zero_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(10)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(0)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Div, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "division by zero must fail"
    );
    Ok(())
}

#[test]
fn eval_add_underflow_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(i64::MIN))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(-1)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Add, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "i64::MIN + (-1) must overflow"
    );
    Ok(())
}

#[test]
fn eval_sub_negative_result() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(3)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(10)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Sub, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(-7) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "3 - 10 should be -7",
        });
    }
    Ok(())
}

#[test]
fn eval_mul_zero() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(i64::MAX))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(0)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Mul, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(0) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "i64::MAX * 0 should be 0",
        });
    }
    Ok(())
}

#[test]
fn eval_div_negative() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(-10))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(3)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Div, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(-3) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "-10 / 3 should be -3 (truncating toward zero)",
        });
    }
    Ok(())
}

// ---- Comparison ops ----

#[test]
fn eval_gt_true() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(10)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(5)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Gt, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "10 > 5 should be true",
        });
    }
    Ok(())
}

#[test]
fn eval_gt_false_when_equal() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(5)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(5)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Gt, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "5 > 5 should be false",
        });
    }
    Ok(())
}

#[test]
fn eval_lt_true() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(3)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(8)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Lt, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "3 < 8 should be true",
        });
    }
    Ok(())
}

#[test]
fn eval_lt_false_when_greater() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(10)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(5)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Lt, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "10 < 5 should be false",
        });
    }
    Ok(())
}

#[test]
fn eval_gte_true_when_equal() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(5)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(5)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Gte, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "5 >= 5 should be true",
        });
    }
    Ok(())
}

#[test]
fn eval_gte_false_when_less() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(3)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(10)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Gte, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "3 >= 10 should be false",
        });
    }
    Ok(())
}

#[test]
fn eval_lte_true_when_equal() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(5)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(5)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Lte, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "5 <= 5 should be true",
        });
    }
    Ok(())
}

#[test]
fn eval_lte_false_when_greater() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(10)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(3)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Lte, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "10 <= 3 should be false",
        });
    }
    Ok(())
}

// ---- Boolean ops ----

#[test]
fn eval_eq_same_i64() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(42)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(42)).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Eq, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "42 == 42 should be true",
        });
    }
    Ok(())
}

#[test]
fn eval_eq_different_types_returns_false() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(0)).map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Bool(false))
        .map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Eq, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "I64(0) == Bool(false) should be false",
        });
    }
    Ok(())
}

#[test]
fn eval_eq_null_null_returns_true() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::Null).map_err(replay_err_to_core)?;
    stack.push(SlotValue::Null).map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Eq, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "Null == Null should be true",
        });
    }
    Ok(())
}

#[test]
fn eval_not_eq_true() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(1)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(2)).map_err(replay_err_to_core)?;
    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::NotEq,
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "1 != 2 should be true",
        });
    }
    Ok(())
}

#[test]
fn eval_and_true_true() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::Bool(true))
        .map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Bool(true))
        .map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::And, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "true && true should be true",
        });
    }
    Ok(())
}

#[test]
fn eval_and_true_false() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::Bool(true))
        .map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Bool(false))
        .map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::And, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "true && false should be false",
        });
    }
    Ok(())
}

#[test]
fn eval_or_false_true() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::Bool(false))
        .map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Bool(true))
        .map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Or, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "false || true should be true",
        });
    }
    Ok(())
}

#[test]
fn eval_or_false_false() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::Bool(false))
        .map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Bool(false))
        .map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Or, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "false || false should be false",
        });
    }
    Ok(())
}

#[test]
fn eval_not_true() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::Bool(true))
        .map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Not, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "!true should be false",
        });
    }
    Ok(())
}

#[test]
fn eval_not_false() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::Bool(false))
        .map_err(replay_err_to_core)?;
    eval_replay_op(&plan, &run, &mut store, ExprOp::Not, &mut stack, &mut taint)
        .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "!false should be true",
        });
    }
    Ok(())
}

#[test]
fn eval_and_rejects_non_bool() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(1)).map_err(replay_err_to_core)?;
    stack
        .push(SlotValue::Bool(true))
        .map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::And, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "And with non-bool left must fail"
    );
    Ok(())
}

#[test]
fn eval_or_rejects_non_bool() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::Bool(true))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(1)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Or, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "Or with non-bool right must fail"
    );
    Ok(())
}

#[test]
fn eval_not_rejects_non_bool() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(1)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Not, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "Not on non-bool must fail"
    );
    Ok(())
}

// ---- LoadSlot / LoadConst ----

#[test]
fn eval_load_slot_succeeds() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let mut run = make_frame(4)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(99))?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadSlot(SlotIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(99) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "loaded slot should be I64(99)",
        });
    }
    Ok(())
}

#[test]
fn eval_load_slot_out_of_bounds_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(2)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    let result = eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadSlot(SlotIdx::new(50)),
        &mut stack,
        &mut taint,
    );
    assert!(
        matches!(result, Err(ReplayError::SlotNotAvailable { .. })),
        "loading out-of-bounds slot must fail"
    );
    Ok(())
}

#[test]
fn eval_load_slot_propagates_taint() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let mut run = make_frame(4)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadSlot(SlotIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    if taint != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "taint should be Secret after loading tainted slot",
        });
    }
    Ok(())
}

#[test]
fn eval_load_const_i64() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![ConstValue::I64(77)])?;
    let run = make_frame(2)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadConst(ConstIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(77) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "loaded const should be I64(77)",
        });
    }
    Ok(())
}

#[test]
fn eval_load_const_bool() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![ConstValue::Bool(true)])?;
    let run = make_frame(2)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadConst(ConstIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "loaded const should be Bool(true)",
        });
    }
    Ok(())
}

#[test]
fn eval_load_const_null() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![ConstValue::Null])?;
    let run = make_frame(2)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadConst(ConstIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::Null {
        return Err(CoreError::InternalInvariantViolation {
            reason: "loaded const should be Null",
        });
    }
    Ok(())
}

#[test]
fn eval_load_const_out_of_bounds_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![ConstValue::I64(1)])?;
    let run = make_frame(2)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    let result = eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadConst(ConstIdx::new(5)),
        &mut stack,
        &mut taint,
    );
    assert!(
        matches!(result, Err(ReplayError::Internal { .. })),
        "loading out-of-bounds constant must fail"
    );
    Ok(())
}

// ---- LoadAccessor ----

#[test]
fn eval_load_accessor_object_field() -> Result<(), CoreError> {
    let field_sym = SymbolId::new(0);
    let mut store = ValueStore::new();
    let fields = vec![crate::value_store::ObjectField::clean(
        field_sym,
        SlotValue::I64(42),
    )];
    let obj_handle = store.insert_object(fields.into_boxed_slice())?;

    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
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
        ],
        vec![],
        vec![],
        vec![AccessorProgram {
            root: SlotIdx::new(0),
            path: vec![PathSegment::Field(field_sym)].into(),
        }],
    )?;

    let mut run = make_frame(4)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Object(obj_handle))?;
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadAccessor(AccessorIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(42) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "accessor should resolve object field to I64(42)",
        });
    }
    Ok(())
}

#[test]
fn eval_load_accessor_list_index() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let items = vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)];
    let list_handle = store.insert_list(items.into_boxed_slice())?;

    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
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
        ],
        vec![],
        vec![],
        vec![AccessorProgram {
            root: SlotIdx::new(0),
            path: vec![PathSegment::Index(1)].into(),
        }],
    )?;

    let mut run = make_frame(4)?;
    run.write_slot(SlotIdx::new(0), SlotValue::List(list_handle))?;
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadAccessor(AccessorIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(20) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "accessor should resolve list[1] to I64(20)",
        });
    }
    Ok(())
}

#[test]
fn eval_load_accessor_empty_path_returns_root() -> Result<(), CoreError> {
    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
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
        ],
        vec![],
        vec![],
        vec![AccessorProgram {
            root: SlotIdx::new(0),
            path: vec![].into(),
        }],
    )?;

    let mut run = make_frame(4)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(99))?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadAccessor(AccessorIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(99) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "empty accessor path should return root value",
        });
    }
    Ok(())
}

#[test]
fn eval_load_accessor_out_of_bounds_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    let result = eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadAccessor(AccessorIdx::new(99)),
        &mut stack,
        &mut taint,
    );
    assert!(
        matches!(result, Err(ReplayError::Internal { .. })),
        "out-of-bounds accessor must fail"
    );
    Ok(())
}

// ---- Helper ops with wrong operand types ----

#[test]
fn eval_helper_op_with_wrong_types_returns_eval_failed() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(2)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack.push(SlotValue::I64(1)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(2)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::Contains,
        &mut stack,
        &mut taint,
    );
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "Contains on non-symbols must fail evaluation, not panic"
    );
    Ok(())
}

// ---- eval_expr_for_replay integration ----

#[test]
fn eval_expr_for_replay_addition() -> Result<(), CoreError> {
    let expr = make_expr_program(vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Add,
    ])?;

    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
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
        ],
        vec![],
        vec![expr],
        vec![],
    )?;

    let mut run = make_frame(4)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(15))?;
    run.write_slot(SlotIdx::new(1), SlotValue::I64(27))?;
    let mut store = ValueStore::new();

    let (value, _taint) = eval_expr_for_replay(&plan, &run, &mut store, ExprIdx::new(0))
        .map_err(replay_err_to_core)?;

    if value != SlotValue::I64(42) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "15 + 27 should be 42",
        });
    }
    Ok(())
}

#[test]
fn eval_expr_for_replay_comparison_chain() -> Result<(), CoreError> {
    let expr = make_expr_program(vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Gt,
        ExprOp::LoadConst(ConstIdx::new(2)),
        ExprOp::LoadConst(ConstIdx::new(3)),
        ExprOp::Lt,
        ExprOp::And,
    ])?;

    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
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
        ],
        vec![
            ConstValue::I64(10),
            ConstValue::I64(5),
            ConstValue::I64(3),
            ConstValue::I64(8),
        ],
        vec![expr],
        vec![],
    )?;

    let run = make_frame(2)?;
    let mut store = ValueStore::new();

    let (value, _taint) = eval_expr_for_replay(&plan, &run, &mut store, ExprIdx::new(0))
        .map_err(replay_err_to_core)?;

    if value != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "(10 > 5) && (3 < 8) should be true",
        });
    }
    Ok(())
}

#[test]
fn eval_expr_for_replay_with_uninitialized_slot() -> Result<(), CoreError> {
    // Expression loads from a slot that was never written
    let expr = make_expr_program(vec![ExprOp::LoadSlot(SlotIdx::new(0))])?;

    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
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
        ],
        vec![],
        vec![expr],
        vec![],
    )?;

    // Slot 0 is never written -- read should fail
    let run = make_frame(2)?;
    let mut store = ValueStore::new();

    let result = eval_expr_for_replay(&plan, &run, &mut store, ExprIdx::new(0));
    assert!(
        result.is_err(),
        "expression loading uninitialized slot must fail"
    );
    Ok(())
}

#[test]
fn eval_expr_for_replay_out_of_bounds_expr() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(2)?;
    let mut store = ValueStore::new();

    let result = eval_expr_for_replay(&plan, &run, &mut store, ExprIdx::new(99));
    assert!(
        matches!(result, Err(ReplayError::Internal { .. })),
        "out-of-bounds expression must fail"
    );
    Ok(())
}

// =========================================================================
// BLACKHAT security regression tests -- replay expression evaluation
// =========================================================================

// --- FINDING BH-OPS-01: Division of i64::MIN by -1 must overflow ---
//
// checked_div correctly returns None for i64::MIN / -1, but we verify
// the replay path handles this case.

#[test]
fn blackhat_eval_div_min_by_neg_one_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(i64::MIN))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(-1)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Div, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "BLACKHAT BH-OPS-01: i64::MIN / -1 must overflow"
    );
    Ok(())
}

// --- FINDING BH-OPS-02: Subtraction of i64::MIN by 1 must underflow ---

#[test]
fn blackhat_eval_sub_min_by_one_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(i64::MIN))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(1)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Sub, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "BLACKHAT BH-OPS-02: i64::MIN - 1 must underflow"
    );
    Ok(())
}

// --- FINDING BH-OPS-03: Addition of i64::MAX + 1 must overflow ---

#[test]
fn blackhat_eval_add_max_plus_one_returns_error() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(i64::MAX))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(1)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Add, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "BLACKHAT BH-OPS-03: i64::MAX + 1 must overflow"
    );
    Ok(())
}

// --- FINDING BH-OPS-04: Multiplication overflow at boundary values ---

#[test]
fn blackhat_eval_mul_boundary_overflow() -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    stack
        .push(SlotValue::I64(i64::MIN))
        .map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(2)).map_err(replay_err_to_core)?;
    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Mul, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "BLACKHAT BH-OPS-04: i64::MIN * 2 must overflow"
    );
    Ok(())
}

// --- FINDING BH-OPS-05: Taint joins across LoadSlot operations ---
//
// When two slots are loaded (one Clean, one Secret) and added,
// the accumulated taint must be Secret.

#[test]
fn blackhat_taint_joins_across_expression() -> Result<(), CoreError> {
    let expr = make_expr_program(vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Add,
    ])?;

    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
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
        ],
        vec![],
        vec![expr],
        vec![],
    )?;

    let mut run = make_frame(4)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(10), Taint::Clean)?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(20), Taint::Secret)?;
    let mut store = ValueStore::new();

    let (value, taint) = eval_expr_for_replay(&plan, &run, &mut store, ExprIdx::new(0))
        .map_err(replay_err_to_core)?;

    assert_eq!(
        value,
        SlotValue::I64(30),
        "BLACKHAT BH-OPS-05: value must be 30"
    );
    assert_eq!(
        taint,
        Taint::Secret,
        "BLACKHAT BH-OPS-05: taint must be Secret when one operand is Secret"
    );
    Ok(())
}

// --- FINDING BH-OPS-06: Expression stack overflow is caught ---
//
// Pushing more than capacity allows must fail, not corrupt memory.

#[test]
fn blackhat_expression_stack_overflow_is_safe() -> Result<(), CoreError> {
    let mut stack = ReplayExprStack::new(2).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(1)).map_err(replay_err_to_core)?;
    stack.push(SlotValue::I64(2)).map_err(replay_err_to_core)?;
    let result = stack.push(SlotValue::I64(3));
    assert!(
        result.is_err(),
        "BLACKHAT BH-OPS-06: stack overflow must return error, not corrupt"
    );

    // Verify the stack is still consistent after the failed push
    let first = stack.pop().map_err(replay_err_to_core)?;
    assert_eq!(
        first,
        SlotValue::I64(2),
        "BLACKHAT BH-OPS-06: stack must remain consistent after overflow"
    );
    Ok(())
}

// --- FINDING BH-OPS-07: Helper ops evaluate like the engine (vb-cofe1) ---
//
// Replay previously rejected every helper opcode. The bead extends replay to
// cover them with engine-matching semantics, so these tests now pin the
// supported behavior. Wrong operand types must still fail evaluation
// (ExpressionEvalFailed), never panic and never surface as Internal.

fn eval_stack_helper(
    op: ExprOp,
    store: &mut ValueStore,
    values: &[SlotValue],
) -> Result<SlotValue, CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut stack = ReplayExprStack::new(8).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;
    for value in values {
        stack.push(*value).map_err(replay_err_to_core)?;
    }
    eval_replay_op(&plan, &run, store, op, &mut stack, &mut taint).map_err(replay_err_to_core)?;
    stack.pop().map_err(replay_err_to_core)
}

fn check_helper_type_error(op: ExprOp, values: &[SlotValue]) -> Result<(), CoreError> {
    let plan = make_minimal_plan_with_constants(vec![])?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(8).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;
    for value in values {
        stack.push(*value).map_err(replay_err_to_core)?;
    }
    let result = eval_replay_op(&plan, &run, &mut store, op, &mut stack, &mut taint);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { .. })),
        "BLACKHAT BH-OPS-07: {op:?} on wrong types must fail evaluation, not panic"
    );
    Ok(())
}

fn insert_text(store: &mut ValueStore, text: &str) -> Result<SymbolId, CoreError> {
    store.insert_symbol(text)
}

fn insert_numbers(store: &mut ValueStore, values: &[i64]) -> Result<ListId, CoreError> {
    let items: Vec<SlotValue> = values.iter().map(|&v| SlotValue::I64(v)).collect();
    store.insert_list(items.into_boxed_slice())
}

#[test]
fn blackhat_helper_contains_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let haystack = insert_text(&mut store, "hello world")?;
    let needle = insert_text(&mut store, "world")?;
    let missing = insert_text(&mut store, "xyz")?;
    let found = eval_stack_helper(
        ExprOp::Contains,
        &mut store,
        &[SlotValue::Symbol(haystack), SlotValue::Symbol(needle)],
    )?;
    if found != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: contains should find substring",
        });
    }
    let absent = eval_stack_helper(
        ExprOp::Contains,
        &mut store,
        &[SlotValue::Symbol(haystack), SlotValue::Symbol(missing)],
    )?;
    if absent != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: contains should miss absent substring",
        });
    }
    Ok(())
}

#[test]
fn blackhat_helper_starts_with_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let text = insert_text(&mut store, "hello world")?;
    let prefix = insert_text(&mut store, "hello")?;
    let result = eval_stack_helper(
        ExprOp::StartsWith,
        &mut store,
        &[SlotValue::Symbol(text), SlotValue::Symbol(prefix)],
    )?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: starts_with should match prefix",
        });
    }
    Ok(())
}

#[test]
fn blackhat_helper_ends_with_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let text = insert_text(&mut store, "hello world")?;
    let suffix = insert_text(&mut store, "world")?;
    let result = eval_stack_helper(
        ExprOp::EndsWith,
        &mut store,
        &[SlotValue::Symbol(text), SlotValue::Symbol(suffix)],
    )?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: ends_with should match suffix",
        });
    }
    Ok(())
}

#[test]
fn blackhat_helper_has_matches_engine_list_membership() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let list = insert_numbers(&mut store, &[1, 2, 3])?;
    let present = eval_stack_helper(
        ExprOp::Has,
        &mut store,
        &[SlotValue::List(list), SlotValue::I64(2)],
    )?;
    if present != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: has should find list member",
        });
    }
    let absent = eval_stack_helper(
        ExprOp::Has,
        &mut store,
        &[SlotValue::List(list), SlotValue::I64(9)],
    )?;
    if absent != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: has should miss absent member",
        });
    }
    Ok(())
}

#[test]
fn blackhat_helper_exists_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let key = insert_text(&mut store, "k")?;
    let object = store
        .insert_object(
            vec![crate::value_store::ObjectField::clean(
                key,
                SlotValue::I64(1),
            )]
            .into_boxed_slice(),
        )
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "object insert must succeed",
        })?;
    let present = eval_stack_helper(ExprOp::Exists, &mut store, &[SlotValue::Object(object)])?;
    if present != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: exists should hold for non-empty object",
        });
    }
    let null = eval_stack_helper(ExprOp::Exists, &mut store, &[SlotValue::Null])?;
    if null != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: exists should fail for null",
        });
    }
    check_helper_type_error(ExprOp::Exists, &[SlotValue::I64(1)])
}

#[test]
fn blackhat_helper_length_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let text = insert_text(&mut store, "hello")?;
    let text_len = eval_stack_helper(ExprOp::Length, &mut store, &[SlotValue::Symbol(text)])?;
    if text_len != SlotValue::I64(5) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: length of text should be 5",
        });
    }
    let list = insert_numbers(&mut store, &[1, 2, 3])?;
    let list_len = eval_stack_helper(ExprOp::Length, &mut store, &[SlotValue::List(list)])?;
    if list_len != SlotValue::I64(3) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: length of list should be 3",
        });
    }
    check_helper_type_error(ExprOp::Length, &[SlotValue::Bool(true)])
}

#[test]
fn blackhat_helper_empty_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let null_empty = eval_stack_helper(ExprOp::Empty, &mut store, &[SlotValue::Null])?;
    if null_empty != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: empty null should be true",
        });
    }
    let empty_list = insert_numbers(&mut store, &[])?;
    let list_empty = eval_stack_helper(ExprOp::Empty, &mut store, &[SlotValue::List(empty_list)])?;
    if list_empty != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: empty list should be true",
        });
    }
    let full_list = insert_numbers(&mut store, &[1])?;
    let list_full = eval_stack_helper(ExprOp::Empty, &mut store, &[SlotValue::List(full_list)])?;
    if list_full != SlotValue::Bool(false) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: non-empty list should be false",
        });
    }
    Ok(())
}

#[test]
fn blackhat_helper_append_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let list = insert_numbers(&mut store, &[1, 2])?;
    let result = eval_stack_helper(
        ExprOp::Append,
        &mut store,
        &[SlotValue::List(list), SlotValue::I64(3)],
    )?;
    let appended_id = match result {
        SlotValue::List(id) => id,
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "BLACKHAT BH-OPS-07: append must return a list",
            });
        }
    };
    let items = store
        .list(appended_id)
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "appended list must resolve",
        })?;
    if items != [SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)] {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: append should extend the list",
        });
    }
    check_helper_type_error(ExprOp::Append, &[SlotValue::I64(1), SlotValue::I64(2)])
}

#[test]
fn blackhat_helper_append_if_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let list = insert_numbers(&mut store, &[1])?;
    let appended = eval_stack_helper(
        ExprOp::AppendIf,
        &mut store,
        &[
            SlotValue::List(list),
            SlotValue::I64(2),
            SlotValue::Bool(true),
        ],
    )?;
    let appended_id = match appended {
        SlotValue::List(id) => id,
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "BLACKHAT BH-OPS-07: append_if must return a list",
            });
        }
    };
    let items = store
        .list(appended_id)
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "append_if list must resolve",
        })?;
    if items != [SlotValue::I64(1), SlotValue::I64(2)] {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: append_if true should extend the list",
        });
    }
    let kept = eval_stack_helper(
        ExprOp::AppendIf,
        &mut store,
        &[
            SlotValue::List(list),
            SlotValue::I64(2),
            SlotValue::Bool(false),
        ],
    )?;
    let kept_id = match kept {
        SlotValue::List(id) => id,
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "BLACKHAT BH-OPS-07: append_if must return a list",
            });
        }
    };
    let kept_items = store
        .list(kept_id)
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "append_if list must resolve",
        })?;
    if kept_items != [SlotValue::I64(1)] {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: append_if false should copy the list",
        });
    }
    Ok(())
}

#[test]
fn blackhat_helper_merge_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let key_a = insert_text(&mut store, "a")?;
    let key_b = insert_text(&mut store, "b")?;
    let left = store
        .insert_object(
            vec![crate::value_store::ObjectField::clean(
                key_a,
                SlotValue::I64(1),
            )]
            .into_boxed_slice(),
        )
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "object insert must succeed",
        })?;
    let right = store
        .insert_object(
            vec![crate::value_store::ObjectField::clean(
                key_b,
                SlotValue::I64(2),
            )]
            .into_boxed_slice(),
        )
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "object insert must succeed",
        })?;
    let result = eval_stack_helper(
        ExprOp::Merge,
        &mut store,
        &[SlotValue::Object(left), SlotValue::Object(right)],
    )?;
    let merged_id = match result {
        SlotValue::Object(id) => id,
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "BLACKHAT BH-OPS-07: merge must return an object",
            });
        }
    };
    let fields = store
        .object(merged_id)
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "merged object must resolve",
        })?;
    if fields.len() != 2 {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: merge should combine fields",
        });
    }
    check_helper_type_error(ExprOp::Merge, &[SlotValue::I64(1), SlotValue::I64(2)])
}

#[test]
fn blackhat_helper_sum_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let list = insert_numbers(&mut store, &[1, 2, 3])?;
    let result = eval_stack_helper(ExprOp::Sum, &mut store, &[SlotValue::List(list)])?;
    if result != SlotValue::I64(6) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: sum should total list items",
        });
    }
    check_helper_type_error(ExprOp::Sum, &[SlotValue::I64(1)])
}

#[test]
fn blackhat_helper_count_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let list = insert_numbers(&mut store, &[1, 2, 3])?;
    let result = eval_stack_helper(ExprOp::Count, &mut store, &[SlotValue::List(list)])?;
    if result != SlotValue::I64(3) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: count should report list length",
        });
    }
    Ok(())
}

#[test]
fn blackhat_helper_unique_matches_engine() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let list = insert_numbers(&mut store, &[1, 2, 1, 3])?;
    let result = eval_stack_helper(ExprOp::Unique, &mut store, &[SlotValue::List(list)])?;
    let unique_id = match result {
        SlotValue::List(id) => id,
        _ => {
            return Err(CoreError::InternalInvariantViolation {
                reason: "BLACKHAT BH-OPS-07: unique must return a list",
            });
        }
    };
    let items = store
        .list(unique_id)
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "unique list must resolve",
        })?;
    if items != [SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)] {
        return Err(CoreError::InternalInvariantViolation {
            reason: "BLACKHAT BH-OPS-07: unique should deduplicate preserving order",
        });
    }
    Ok(())
}

// ---- Mixed I64/F64 arithmetic (vb-cofe1) ----

fn f64_value(raw: f64) -> Result<SlotValue, CoreError> {
    FiniteF64::new(raw).map(SlotValue::F64)
}

#[test]
fn replay_mixed_add_promotes_both_directions() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let result = eval_stack_helper(
        ExprOp::Add,
        &mut store,
        &[SlotValue::I64(3), f64_value(0.5)?],
    )?;
    if result != f64_value(3.5)? {
        return Err(CoreError::InternalInvariantViolation {
            reason: "I64(3) + F64(0.5) should be F64(3.5)",
        });
    }
    let flipped = eval_stack_helper(
        ExprOp::Add,
        &mut store,
        &[f64_value(1.5)?, SlotValue::I64(2)],
    )?;
    if flipped != f64_value(3.5)? {
        return Err(CoreError::InternalInvariantViolation {
            reason: "F64(1.5) + I64(2) should be F64(3.5)",
        });
    }
    Ok(())
}

#[test]
fn replay_mixed_sub_mul_div_promote() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let result = eval_stack_helper(
        ExprOp::Sub,
        &mut store,
        &[SlotValue::I64(10), f64_value(0.5)?],
    )?;
    if result != f64_value(9.5)? {
        return Err(CoreError::InternalInvariantViolation {
            reason: "I64(10) - F64(0.5) should be F64(9.5)",
        });
    }
    let result = eval_stack_helper(
        ExprOp::Mul,
        &mut store,
        &[f64_value(2.5)?, SlotValue::I64(4)],
    )?;
    if result != f64_value(10.0)? {
        return Err(CoreError::InternalInvariantViolation {
            reason: "F64(2.5) * I64(4) should be F64(10.0)",
        });
    }
    let result = eval_stack_helper(
        ExprOp::Div,
        &mut store,
        &[SlotValue::I64(1), f64_value(2.0)?],
    )?;
    if result != f64_value(0.5)? {
        return Err(CoreError::InternalInvariantViolation {
            reason: "I64(1) / F64(2.0) should be F64(0.5)",
        });
    }
    Ok(())
}

#[test]
fn replay_mixed_div_by_zero_fails() -> Result<(), CoreError> {
    check_helper_type_error(ExprOp::Div, &[SlotValue::I64(1), f64_value(0.0)?])?;
    check_helper_type_error(ExprOp::Div, &[f64_value(1.0)?, SlotValue::I64(0)])
}

#[test]
fn replay_f64_add_and_div_by_zero() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let result = eval_stack_helper(ExprOp::Add, &mut store, &[f64_value(1.5)?, f64_value(2.5)?])?;
    if result != f64_value(4.0)? {
        return Err(CoreError::InternalInvariantViolation {
            reason: "F64(1.5) + F64(2.5) should be F64(4.0)",
        });
    }
    check_helper_type_error(ExprOp::Div, &[f64_value(1.0)?, f64_value(0.0)?])
}

#[test]
fn replay_mixed_comparisons_promote_operands() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let result = eval_stack_helper(
        ExprOp::Lt,
        &mut store,
        &[SlotValue::I64(3), f64_value(3.5)?],
    )?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "I64(3) < F64(3.5) should be true",
        });
    }
    let result = eval_stack_helper(
        ExprOp::Gte,
        &mut store,
        &[f64_value(2.0)?, SlotValue::I64(2)],
    )?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "F64(2.0) >= I64(2) should be true",
        });
    }
    let result = eval_stack_helper(
        ExprOp::Gt,
        &mut store,
        &[SlotValue::I64(10), f64_value(9.5)?],
    )?;
    if result != SlotValue::Bool(true) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "I64(10) > F64(9.5) should be true",
        });
    }
    Ok(())
}

#[test]
fn replay_integer_error_contract_preserved() -> Result<(), CoreError> {
    check_helper_type_error(ExprOp::Add, &[SlotValue::I64(i64::MAX), SlotValue::I64(1)])?;
    check_helper_type_error(ExprOp::Div, &[SlotValue::I64(10), SlotValue::I64(0)])?;
    check_helper_type_error(ExprOp::Div, &[SlotValue::I64(i64::MIN), SlotValue::I64(-1)])?;
    check_helper_type_error(ExprOp::Add, &[SlotValue::Bool(true), SlotValue::I64(1)])?;
    check_helper_type_error(ExprOp::Mul, &[f64_value(2.0)?, SlotValue::Null])
}

#[test]
fn replay_mixed_add_via_expr_program() -> Result<(), CoreError> {
    let expr = make_expr_program(vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Add,
    ])?;
    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
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
        ],
        vec![
            ConstValue::I64(3),
            ConstValue::F64(FiniteF64::new(2.5).map_err(|_| {
                CoreError::InternalInvariantViolation {
                    reason: "test constant must be finite",
                }
            })?),
        ],
        vec![expr],
        vec![],
    )?;
    let run = make_frame(4)?;
    let mut store = ValueStore::new();
    let (value, _) = eval_expr_for_replay(&plan, &run, &mut store, ExprIdx::new(0))
        .map_err(replay_err_to_core)?;
    if value != f64_value(5.5)? {
        return Err(CoreError::InternalInvariantViolation {
            reason: "replay of I64(3) + F64(2.5) should be F64(5.5)",
        });
    }
    Ok(())
}

// --- CE-003: Replay accessor must propagate field/list-item taint ---
//
// Engine evaluation joins segment taint from every object_field and list_item
// traversal. The replay path must do the same, otherwise accessor-derived
// values can land on the replay stack with weaker taint than the engine
// produced, silently downgrading secret propagation.

#[test]
fn ce003_replay_object_field_taint_joins_segment() -> Result<(), CoreError> {
    let field_sym = SymbolId::new(0);
    let mut store = ValueStore::new();
    let fields = vec![crate::value_store::ObjectField::with_taint(
        field_sym,
        SlotValue::I64(42),
        Taint::Secret,
    )]
    .into_boxed_slice();
    let obj_handle = store.insert_object(fields)?;

    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
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
        ],
        vec![],
        vec![],
        vec![AccessorProgram {
            root: SlotIdx::new(0),
            path: vec![PathSegment::Field(field_sym)].into(),
        }],
    )?;

    let mut run = make_frame(4)?;
    // Root slot is Clean — segment taint is Secret. Combined result must be Secret.
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(obj_handle), Taint::Clean)?;
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadAccessor(AccessorIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(42) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: accessor should resolve object field to I64(42)",
        });
    }
    if taint != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: replay accessor must join field taint into accumulator",
        });
    }
    Ok(())
}

#[test]
fn ce003_replay_list_item_taint_joins_segment() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let values =
        vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)].into_boxed_slice();
    let taints = vec![Taint::Clean, Taint::Secret, Taint::Clean].into_boxed_slice();
    let list_handle = store.insert_list_with_taint(values, taints)?;

    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
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
        ],
        vec![],
        vec![],
        vec![AccessorProgram {
            root: SlotIdx::new(0),
            path: vec![PathSegment::Index(1)].into(),
        }],
    )?;

    let mut run = make_frame(4)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list_handle), Taint::Clean)?;
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadAccessor(AccessorIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(20) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: accessor should resolve list[1] to I64(20)",
        });
    }
    if taint != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: replay accessor must join list-item taint into accumulator",
        });
    }
    Ok(())
}

#[test]
fn ce003_replay_multi_segment_path_joins_every_segment_taint() -> Result<(), CoreError> {
    // Build: list[0] -> list_inner -> list_inner_field
    // list_inner contains one object; list_inner_field is Secret.
    let field_sym = SymbolId::new(0);
    let mut store = ValueStore::new();

    let inner_obj = store.insert_object(
        vec![crate::value_store::ObjectField::with_taint(
            field_sym,
            SlotValue::I64(7),
            Taint::Secret,
        )]
        .into_boxed_slice(),
    )?;
    let inner_list = store.insert_list_with_taint(
        vec![SlotValue::Object(inner_obj)].into_boxed_slice(),
        vec![Taint::Clean].into_boxed_slice(),
    )?;
    let outer_list = store.insert_list_with_taint(
        vec![SlotValue::List(inner_list)].into_boxed_slice(),
        vec![Taint::Clean].into_boxed_slice(),
    )?;

    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
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
        ],
        vec![],
        vec![],
        vec![AccessorProgram {
            root: SlotIdx::new(0),
            path: vec![
                PathSegment::Index(0),
                PathSegment::Index(0),
                PathSegment::Field(field_sym),
            ]
            .into(),
        }],
    )?;

    let mut run = make_frame(4)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(outer_list), Taint::Clean)?;
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadAccessor(AccessorIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(7) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: deep accessor should resolve to I64(7)",
        });
    }
    if taint != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: replay must join every segment's taint along the path",
        });
    }
    Ok(())
}

#[test]
fn ce003_replay_root_taint_joins_into_accumulator() -> Result<(), CoreError> {
    // Regression for vb-a68zo follow-up vb-yt5wq: the original CE-003 tests
    // only exercised segment taint propagation (root slot was always Clean).
    // This test sets the root slot taint to Secret while the field it points
    // at is Clean. A correct replay accessor must join the root taint into
    // the accumulator so the final taint is Secret. Mutation resistance:
    // deleting the `accumulated_taint = run.read_taint(program.root)` line
    // (or any other "drop root taint" mutation in `eval_accessor_for_replay`)
    // would leave the accumulator at Clean and this test would fail.
    let field_sym = SymbolId::new(0);
    let mut store = ValueStore::new();
    let fields = vec![crate::value_store::ObjectField::with_taint(
        field_sym,
        SlotValue::I64(99),
        Taint::Clean,
    )]
    .into_boxed_slice();
    let obj_handle = store.insert_object(fields)?;

    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
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
        ],
        vec![],
        vec![],
        vec![AccessorProgram {
            root: SlotIdx::new(0),
            path: vec![PathSegment::Field(field_sym)].into(),
        }],
    )?;

    let mut run = make_frame(4)?;
    // Root slot carries Secret taint; field itself is Clean. The accessor
    // must surface Secret as the joined accumulator value.
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::Object(obj_handle),
        Taint::Secret,
    )?;
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadAccessor(AccessorIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(99) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: accessor should resolve object field to I64(99)",
        });
    }
    if taint != Taint::Secret {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: replay accessor must join root slot taint into accumulator",
        });
    }
    Ok(())
}

#[test]
fn ce003_replay_lattice_random_respected() -> Result<(), CoreError> {
    // Regression for vb-a68zo follow-up vb-yt5wq: the original CE-003 tests
    // only ever produced Taint::Secret. The taint lattice orders
    //   Clean(0) < DerivedFromSecret(1) < Secret(2) < Random(3) < TimeDependent(4)
    // via `join_taint` (max-discriminant wins). This test puts a Random
    // taint on the deepest segment and a Secret taint on the root slot,
    // then asserts the accumulator is Random -- proving the join uses the
    // lattice's max-rule rather than a hard-coded "Secret" shortcut.
    // Mutation resistance: replacing `join_taint(accumulated_taint, segment_taint)`
    // with identity (e.g. just `accumulated_taint`) would leave the result
    // at Secret, and this test would fail.
    let field_sym = SymbolId::new(0);
    let mut store = ValueStore::new();

    let inner_obj = store.insert_object(
        vec![crate::value_store::ObjectField::with_taint(
            field_sym,
            SlotValue::I64(11),
            Taint::Random,
        )]
        .into_boxed_slice(),
    )?;
    let inner_list = store.insert_list_with_taint(
        vec![SlotValue::Object(inner_obj)].into_boxed_slice(),
        vec![Taint::Clean].into_boxed_slice(),
    )?;
    let outer_list = store.insert_list_with_taint(
        vec![SlotValue::List(inner_list)].into_boxed_slice(),
        vec![Taint::Clean].into_boxed_slice(),
    )?;

    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
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
        ],
        vec![],
        vec![],
        vec![AccessorProgram {
            root: SlotIdx::new(0),
            path: vec![
                PathSegment::Index(0),
                PathSegment::Index(0),
                PathSegment::Field(field_sym),
            ]
            .into(),
        }],
    )?;

    let mut run = make_frame(4)?;
    // Root slot is Secret -- must lose to the deeper Random segment under
    // the lattice join. Intermediate lists/objects stay Clean.
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(outer_list), Taint::Secret)?;
    let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
    let mut taint = Taint::Clean;

    eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadAccessor(AccessorIdx::new(0)),
        &mut stack,
        &mut taint,
    )
    .map_err(replay_err_to_core)?;

    let result = stack.pop().map_err(replay_err_to_core)?;
    if result != SlotValue::I64(11) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: deep accessor should resolve to I64(11)",
        });
    }
    if taint != Taint::Random {
        return Err(CoreError::InternalInvariantViolation {
            reason: "CE-003: replay must respect lattice -- Random(3) outranks Secret(2)",
        });
    }
    Ok(())
}

#[test]
fn blackhat_helper_unique_rejects_non_list() -> Result<(), CoreError> {
    check_helper_type_error(ExprOp::Unique, &[SlotValue::I64(7)])
}
