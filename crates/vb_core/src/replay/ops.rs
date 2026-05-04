//! Expression evaluation operations for replay.

use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::{AccessorIdx, ConstIdx, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint, join_taint};
use crate::value_store::ValueStore;
use crate::workflow::{CompiledWorkflow, ExprOp};

use super::{ReplayError, ReplayExprStack};

pub fn eval_replay_op(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    store: &mut ValueStore,
    op: ExprOp,
    stack: &mut ReplayExprStack,
    taint_accum: &mut Taint,
) -> Result<(), ReplayError> {
    match op {
        ExprOp::LoadSlot(slot) => eval_load_slot(run, slot, stack, taint_accum),
        ExprOp::LoadConst(constant) => eval_load_const(plan, constant, stack),
        ExprOp::LoadAccessor(accessor) => {
            eval_load_accessor(plan, run, store, accessor, stack, taint_accum)
        }
        ExprOp::Eq => eval_eq(stack),
        ExprOp::NotEq => eval_not_eq(stack),
        ExprOp::And => eval_and(stack),
        ExprOp::Or => eval_or(stack),
        ExprOp::Not => eval_not(stack),
        ExprOp::Add => eval_add(stack),
        ExprOp::Sub => eval_sub(stack),
        ExprOp::Mul => eval_mul(stack),
        ExprOp::Div => eval_div(stack),
        ExprOp::Gt => eval_gt(stack),
        ExprOp::Gte => eval_gte(stack),
        ExprOp::Lt => eval_lt(stack),
        ExprOp::Lte => eval_lte(stack),
        _ => Err(ReplayError::Internal {
            reason: "unsupported expression op for replay",
        }),
    }
}

fn eval_load_slot(
    run: &RunFrame,
    slot: SlotIdx,
    stack: &mut ReplayExprStack,
    taint_accum: &mut Taint,
) -> Result<(), ReplayError> {
    let value = *run.read_slot(slot).map_err(|e| match e {
        EngineError::SlotOutOfBounds { slot: s } => ReplayError::SlotNotAvailable { slot: s },
        _ => ReplayError::Internal {
            reason: "unexpected error reading expression load slot",
        },
    })?;
    let slot_taint = run.read_taint(slot).map_err(|_| ReplayError::Internal {
        reason: "read_taint failed",
    })?;
    *taint_accum = join_taint(*taint_accum, slot_taint);
    stack.push(value)
}

fn eval_load_const(
    plan: &CompiledWorkflow,
    constant: ConstIdx,
    stack: &mut ReplayExprStack,
) -> Result<(), ReplayError> {
    let value = plan
        .constant(constant)
        .ok_or(ReplayError::Internal {
            reason: "constant out of bounds",
        })?
        .to_slot_value()
        .map_err(|_| ReplayError::Internal {
            reason: "constant to slot value failed",
        })?;
    stack.push(value)
}

fn eval_load_accessor(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    store: &mut ValueStore,
    accessor: AccessorIdx,
    stack: &mut ReplayExprStack,
    taint_accum: &mut Taint,
) -> Result<(), ReplayError> {
    let accessor_program = plan.accessor(accessor).ok_or(ReplayError::Internal {
        reason: "accessor out of bounds",
    })?;
    let root_taint = run
        .read_taint(accessor_program.root)
        .map_err(|_| ReplayError::Internal {
            reason: "read_taint failed for accessor root",
        })?;
    let value = eval_accessor_for_replay(run, store, accessor_program)?;
    *taint_accum = join_taint(*taint_accum, root_taint);
    stack.push(value)
}

fn eval_eq(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_pair(stack)?;
    stack.push(SlotValue::Bool(left == right))
}

fn eval_not_eq(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_pair(stack)?;
    stack.push(SlotValue::Bool(left != right))
}

fn eval_and(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_pair(stack)?;
    let left_bool = expect_bool_replay(left)?;
    let right_bool = expect_bool_replay(right)?;
    stack.push(SlotValue::Bool(left_bool && right_bool))
}

fn eval_or(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_pair(stack)?;
    let left_bool = expect_bool_replay(left)?;
    let right_bool = expect_bool_replay(right)?;
    stack.push(SlotValue::Bool(left_bool || right_bool))
}

fn eval_not(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let value = stack.pop()?;
    let b = expect_bool_replay(value)?;
    stack.push(SlotValue::Bool(!b))
}

fn eval_add(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    let result = left
        .checked_add(right)
        .ok_or(ReplayError::ExpressionEvalFailed {
            step: StepIdx::ZERO,
        })?;
    stack.push(SlotValue::I64(result))
}

fn eval_sub(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    let result = left
        .checked_sub(right)
        .ok_or(ReplayError::ExpressionEvalFailed {
            step: StepIdx::ZERO,
        })?;
    stack.push(SlotValue::I64(result))
}

fn eval_mul(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    let result = left
        .checked_mul(right)
        .ok_or(ReplayError::ExpressionEvalFailed {
            step: StepIdx::ZERO,
        })?;
    stack.push(SlotValue::I64(result))
}

fn eval_div(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    let result = left
        .checked_div(right)
        .ok_or(ReplayError::ExpressionEvalFailed {
            step: StepIdx::ZERO,
        })?;
    stack.push(SlotValue::I64(result))
}

fn eval_gt(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    stack.push(SlotValue::Bool(left > right))
}

fn eval_gte(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    stack.push(SlotValue::Bool(left >= right))
}

fn eval_lt(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    stack.push(SlotValue::Bool(left < right))
}

fn eval_lte(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    stack.push(SlotValue::Bool(left <= right))
}

fn eval_accessor_for_replay(
    run: &RunFrame,
    store: &mut ValueStore,
    program: &crate::workflow::AccessorProgram,
) -> Result<SlotValue, ReplayError> {
    let mut current = *run.read_slot(program.root).map_err(|e| match e {
        EngineError::SlotOutOfBounds { slot } => ReplayError::SlotNotAvailable { slot },
        _ => ReplayError::Internal {
            reason: "unexpected error reading accessor root",
        },
    })?;
    if program.path.is_empty() {
        return Ok(current);
    }
    let mut index = 0usize;
    while index < program.path.len() {
        let segment = program
            .path
            .get(index)
            .copied()
            .ok_or(ReplayError::Internal {
                reason: "accessor path index checked by loop bound",
            })?;
        current = match (current, segment) {
            (SlotValue::Object(object), crate::workflow::PathSegment::Field(field)) => store
                .object_field(object, field)
                .map_err(|_| ReplayError::Internal {
                    reason: "object field not found during replay accessor",
                })?,
            (SlotValue::List(list), crate::workflow::PathSegment::Index(idx)) => store
                .list_item(list, idx)
                .map_err(|_| ReplayError::Internal {
                    reason: "list index out of bounds during replay accessor",
                })?,
            (_, _) => {
                return Err(ReplayError::Internal {
                    reason: "unsupported accessor traversal during replay",
                });
            }
        };
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "accessor path index overflow",
        })?;
    }
    Ok(current)
}

pub fn pop_pair(stack: &mut ReplayExprStack) -> Result<(SlotValue, SlotValue), ReplayError> {
    let right = stack.pop()?;
    let left = stack.pop()?;
    Ok((left, right))
}

pub fn pop_i64_pair(stack: &mut ReplayExprStack) -> Result<(i64, i64), ReplayError> {
    let right = stack.pop()?;
    let left = stack.pop()?;
    Ok((expect_i64_replay(left)?, expect_i64_replay(right)?))
}

fn expect_bool_replay(value: SlotValue) -> Result<bool, ReplayError> {
    match value {
        SlotValue::Bool(b) => Ok(b),
        _ => Err(ReplayError::ExpressionEvalFailed {
            step: StepIdx::ZERO,
        }),
    }
}

fn expect_i64_replay(value: SlotValue) -> Result<i64, ReplayError> {
    match value {
        SlotValue::I64(v) => Ok(v),
        _ => Err(ReplayError::ExpressionEvalFailed {
            step: StepIdx::ZERO,
        }),
    }
}

#[cfg(test)]
mod tests {
    use crate::errors::CoreError;
    use crate::frame::RunFrame;
    use crate::ids::{
        AccessorIdx, ConstIdx, ExprIdx, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest,
    };
    use crate::limits::MAX_EXPRESSION_STACK;
    use crate::replay::{ReplayError, ReplayExprStack, eval_expr_for_replay};
    use crate::value::{ConstValue, SlotValue, Taint};
    use crate::value_store::ValueStore;
    use crate::workflow::{
        AccessorProgram, CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, PathSegment,
        ResourceContract, WorkflowParts, check_expr_stack_bound,
    };

    use super::{eval_replay_op, pop_i64_pair, pop_pair};

    fn make_plan(
        nodes: Vec<CompiledNode>,
        constants: Vec<ConstValue>,
        expressions: Vec<ExprProgram>,
        accessors: Vec<AccessorProgram>,
    ) -> Result<crate::workflow::CompiledWorkflow, CoreError> {
        crate::workflow::CompiledWorkflow::try_from_parts(WorkflowParts {
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
                CompiledNode {
                    id: StepIdx::new(0),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                    output: None,
                    next: Some(StepIdx::new(1)),
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                    output: None,
                    next: None,
                },
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
            ReplayError::NonDeterministicStep { step, .. } => {
                CoreError::InvalidProgramCounter { step }
            }
            ReplayError::Internal { reason } => {
                CoreError::InternalInvariantViolation { reason }
            }
        }
    }

    // ---- ReplayExprStack lifecycle ----

    #[test]
    fn expr_stack_push_pop_roundtrip() -> Result<(), CoreError> {
        let mut stack = ReplayExprStack::new(3).map_err(replay_err_to_core)?;
        stack.push(SlotValue::I64(42)).map_err(replay_err_to_core)?;
        stack.push(SlotValue::Bool(true)).map_err(replay_err_to_core)?;

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
            result.is_err(),
            "popping from empty stack must fail"
        );
        Ok(())
    }

    #[test]
    fn expr_stack_push_overflow_returns_error() -> Result<(), CoreError> {
        let mut stack = ReplayExprStack::new(1).map_err(replay_err_to_core)?;
        stack.push(SlotValue::Null).map_err(replay_err_to_core)?;
        assert!(stack.push(SlotValue::Null).is_err());
        Ok(())
    }

    #[test]
    fn expr_stack_max_capacity_boundary() -> Result<(), CoreError> {
        let mut stack =
            ReplayExprStack::new(MAX_EXPRESSION_STACK).map_err(replay_err_to_core)?;
        for i in 0..64u64 {
            stack
                .push(SlotValue::I64(
                    i64::try_from(i).map_err(|_| CoreError::InternalInvariantViolation {
                        reason: "conversion failed",
                    })?,
                ))
                .map_err(replay_err_to_core)?;
        }
        assert!(stack.push(SlotValue::Null).is_err());
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
        stack.push(SlotValue::Bool(true)).map_err(replay_err_to_core)?;
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
        stack.push(SlotValue::Bool(false)).map_err(replay_err_to_core)?;

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
        assert!(result.is_err());
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
        let result =
            eval_replay_op(&plan, &run, &mut store, ExprOp::Add, &mut stack, &mut taint);
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

        stack.push(SlotValue::I64(100)).map_err(replay_err_to_core)?;
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
        let result =
            eval_replay_op(&plan, &run, &mut store, ExprOp::Sub, &mut stack, &mut taint);
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
        let result =
            eval_replay_op(&plan, &run, &mut store, ExprOp::Mul, &mut stack, &mut taint);
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
        let result =
            eval_replay_op(&plan, &run, &mut store, ExprOp::Div, &mut stack, &mut taint);
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
        stack
            .push(SlotValue::I64(-1))
            .map_err(replay_err_to_core)?;
        let result =
            eval_replay_op(&plan, &run, &mut store, ExprOp::Add, &mut stack, &mut taint);
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
        eval_replay_op(&plan, &run, &mut store, ExprOp::NotEq, &mut stack, &mut taint)
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
        let result =
            eval_replay_op(&plan, &run, &mut store, ExprOp::And, &mut stack, &mut taint);
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
        let result =
            eval_replay_op(&plan, &run, &mut store, ExprOp::Or, &mut stack, &mut taint);
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
        let result =
            eval_replay_op(&plan, &run, &mut store, ExprOp::Not, &mut stack, &mut taint);
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
        let fields =
            vec![crate::value_store::ObjectField::clean(field_sym, SlotValue::I64(42))];
        let obj_handle = store.insert_object(fields.into_boxed_slice())?;

        let plan = make_plan(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                    output: None,
                    next: Some(StepIdx::new(1)),
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                    output: None,
                    next: None,
                },
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
                CompiledNode {
                    id: StepIdx::new(0),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                    output: None,
                    next: Some(StepIdx::new(1)),
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                    output: None,
                    next: None,
                },
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
                CompiledNode {
                    id: StepIdx::new(0),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                    output: None,
                    next: Some(StepIdx::new(1)),
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                    output: None,
                    next: None,
                },
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

    // ---- Unsupported ops ----

    #[test]
    fn eval_unsupported_op_returns_error() -> Result<(), CoreError> {
        let plan = make_minimal_plan_with_constants(vec![])?;
        let run = make_frame(2)?;
        let mut store = ValueStore::new();
        let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
        let mut taint = Taint::Clean;

        let result = eval_replay_op(
            &plan,
            &run,
            &mut store,
            ExprOp::Contains,
            &mut stack,
            &mut taint,
        );
        assert!(
            matches!(
                result,
                Err(ReplayError::Internal {
                    reason: "unsupported expression op for replay"
                })
            ),
            "Contains op should be unsupported for replay"
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
                CompiledNode {
                    id: StepIdx::new(0),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                    output: None,
                    next: Some(StepIdx::new(1)),
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                    output: None,
                    next: None,
                },
            ],
            vec![],
            vec![expr],
            vec![],
        )?;

        let mut run = make_frame(4)?;
        run.write_slot(SlotIdx::new(0), SlotValue::I64(15))?;
        run.write_slot(SlotIdx::new(1), SlotValue::I64(27))?;
        let mut store = ValueStore::new();

        let (value, _taint) =
            eval_expr_for_replay(&plan, &run, &mut store, ExprIdx::new(0))
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
                CompiledNode {
                    id: StepIdx::new(0),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                    output: None,
                    next: Some(StepIdx::new(1)),
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                    output: None,
                    next: None,
                },
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

        let (value, _taint) =
            eval_expr_for_replay(&plan, &run, &mut store, ExprIdx::new(0))
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
                CompiledNode {
                    id: StepIdx::new(0),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                    output: None,
                    next: Some(StepIdx::new(1)),
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                    output: None,
                    next: None,
                },
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
        stack
            .push(SlotValue::I64(-1))
            .map_err(replay_err_to_core)?;
        let result =
            eval_replay_op(&plan, &run, &mut store, ExprOp::Div, &mut stack, &mut taint);
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
        stack
            .push(SlotValue::I64(1))
            .map_err(replay_err_to_core)?;
        let result =
            eval_replay_op(&plan, &run, &mut store, ExprOp::Sub, &mut stack, &mut taint);
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
        stack
            .push(SlotValue::I64(1))
            .map_err(replay_err_to_core)?;
        let result =
            eval_replay_op(&plan, &run, &mut store, ExprOp::Add, &mut stack, &mut taint);
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
        stack
            .push(SlotValue::I64(2))
            .map_err(replay_err_to_core)?;
        let result =
            eval_replay_op(&plan, &run, &mut store, ExprOp::Mul, &mut stack, &mut taint);
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
                CompiledNode {
                    id: StepIdx::new(0),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                    output: None,
                    next: Some(StepIdx::new(1)),
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                    output: None,
                    next: None,
                },
            ],
            vec![],
            vec![expr],
            vec![],
        )?;

        let mut run = make_frame(4)?;
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(10), Taint::Clean)?;
        run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(20), Taint::Secret)?;
        let mut store = ValueStore::new();

        let (value, taint) =
            eval_expr_for_replay(&plan, &run, &mut store, ExprIdx::new(0))
                .map_err(replay_err_to_core)?;

        assert_eq!(value, SlotValue::I64(30), "BLACKHAT BH-OPS-05: value must be 30");
        assert_eq!(
            taint, Taint::Secret,
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

    // --- FINDING BH-OPS-07: Unsupported ops return error, not panic ---

    #[test]
    fn blackhat_unsupported_ops_return_error_not_panic() -> Result<(), CoreError> {
        let plan = make_minimal_plan_with_constants(vec![])?;
        let run = make_frame(4)?;
        let mut store = ValueStore::new();

        let unsupported_ops = [
            ExprOp::Contains,
            ExprOp::StartsWith,
            ExprOp::EndsWith,
            ExprOp::Has,
            ExprOp::Exists,
            ExprOp::Length,
            ExprOp::Empty,
            ExprOp::Append,
            ExprOp::AppendIf,
            ExprOp::Merge,
            ExprOp::Sum,
            ExprOp::Count,
            ExprOp::Unique,
        ];

        for op in unsupported_ops {
            let mut stack = ReplayExprStack::new(4).map_err(replay_err_to_core)?;
            let mut taint = Taint::Clean;
            let result = eval_replay_op(&plan, &run, &mut store, op, &mut stack, &mut taint);
            assert!(
                matches!(result, Err(ReplayError::Internal { .. })),
                "BLACKHAT BH-OPS-07: unsupported op {op:?} must return error, not panic"
            );
        }
        Ok(())
    }
}
