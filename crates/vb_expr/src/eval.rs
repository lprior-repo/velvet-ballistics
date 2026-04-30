//! Bounded stack-based expression bytecode evaluator.

use arrayvec::ArrayVec;
use vb_core::limits::{MAX_EXPRESSION_STACK, MAX_EXPRESSION_STACK_USIZE};
use vb_core::{ConstValue, ExprOp, ExprProgram, SlotValue};

use crate::{ExprError, ExprResult};

/// Evaluates a compiled expression program against slot and constant pools.
///
/// The evaluator uses a fixed-size stack (`ArrayVec`) bounded to 64 entries.
/// It walks the postfix bytecode program operation by operation.
pub fn eval_expr_program(
    program: &ExprProgram,
    slots: &[Option<SlotValue>],
    constants: &[ConstValue],
) -> ExprResult<SlotValue> {
    let mut stack: ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE> = ArrayVec::new();
    let mut index = 0usize;
    while index < program.ops.len() {
        let op = *program
            .ops
            .as_ref()
            .get(index)
            .ok_or(ExprError::UnexpectedEof)?;
        eval_expr_op(op, &mut stack, slots, constants)?;
        index = next_index(index)?;
    }
    finish_stack(&mut stack)
}

fn next_index(index: usize) -> ExprResult<usize> {
    index.checked_add(1).ok_or(ExprError::UnexpectedEof)
}

fn finish_stack(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
) -> ExprResult<SlotValue> {
    if stack.len() == 1 {
        stack.pop().ok_or(ExprError::StackUnderflow)
    } else if stack.is_empty() {
        Err(ExprError::StackUnderflow)
    } else {
        Err(ExprError::StackOverflow {
            max: MAX_EXPRESSION_STACK,
        })
    }
}

fn eval_expr_op(
    op: ExprOp,
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    slots: &[Option<SlotValue>],
    constants: &[ConstValue],
) -> ExprResult<()> {
    match op {
        ExprOp::LoadSlot(idx) => eval_load_slot(stack, slots, idx),
        ExprOp::LoadConst(idx) => eval_load_const(stack, constants, idx),
        ExprOp::Eq => eval_eq(stack, true),
        ExprOp::NotEq => eval_eq(stack, false),
        ExprOp::And => eval_bool_pair(stack, bool_ops::and),
        ExprOp::Or => eval_bool_pair(stack, bool_ops::or),
        ExprOp::Not => eval_not(stack),
        ExprOp::Add => eval_i64_pair(stack, i64::checked_add),
        ExprOp::Sub => eval_i64_pair(stack, i64::checked_sub),
        ExprOp::Mul => eval_i64_pair(stack, i64::checked_mul),
        ExprOp::Div => eval_div(stack),
        ExprOp::Gt => eval_i64_cmp(stack, i64::gt),
        ExprOp::Gte => eval_i64_cmp(stack, i64::ge),
        ExprOp::Lt => eval_i64_cmp(stack, i64::lt),
        ExprOp::Lte => eval_i64_cmp(stack, i64::le),
        _ => eval_helper_op(op, stack),
    }
}

fn eval_load_slot(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    slots: &[Option<SlotValue>],
    idx: vb_core::SlotIdx,
) -> ExprResult<()> {
    let value = slots
        .get(idx.as_usize())
        .and_then(|opt| *opt)
        .ok_or(ExprError::StackUnderflow)?;
    push_value(stack, value)
}

fn eval_load_const(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    constants: &[ConstValue],
    idx: vb_core::ConstIdx,
) -> ExprResult<()> {
    let constant = constants
        .get(idx.as_usize())
        .ok_or(ExprError::UnexpectedEof)?;
    let value = constant.to_slot_value().map_err(|_| ExprError::UnexpectedEof)?;
    push_value(stack, value)
}

fn eval_eq(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    positive: bool,
) -> ExprResult<()> {
    let (left, right) = pop_pair(stack)?;
    push_value(stack, SlotValue::Bool((left == right) == positive))
}

fn eval_not(stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>) -> ExprResult<()> {
    let value = expect_bool(pop_value(stack)?)?;
    push_value(stack, SlotValue::Bool(!value))
}

fn eval_bool_pair(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    op: fn(bool, bool) -> bool,
) -> ExprResult<()> {
    let (left, right) = pop_pair(stack)?;
    push_value(stack, SlotValue::Bool(op(expect_bool(left)?, expect_bool(right)?)))
}

fn eval_i64_pair(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    op: fn(i64, i64) -> Option<i64>,
) -> ExprResult<()> {
    let (left, right) = pop_i64_pair(stack)?;
    let value = op(left, right).ok_or(ExprError::UnexpectedEof)?;
    push_value(stack, SlotValue::I64(value))
}

fn eval_div(stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>) -> ExprResult<()> {
    let (left, right) = pop_i64_pair(stack)?;
    let value = left.checked_div(right).ok_or(ExprError::DivisionByZero)?;
    push_value(stack, SlotValue::I64(value))
}

fn eval_i64_cmp(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    op: fn(&i64, &i64) -> bool,
) -> ExprResult<()> {
    let (left, right) = pop_i64_pair(stack)?;
    push_value(stack, SlotValue::Bool(op(&left, &right)))
}

fn eval_helper_op(
    op: ExprOp,
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
) -> ExprResult<()> {
    match op {
        ExprOp::Exists => eval_exists(stack),
        ExprOp::Length => eval_length(stack),
        ExprOp::Empty => eval_empty(stack),
        _ => Err(ExprError::UnknownOperator {
            op: format!("{op:?}"),
        }),
    }
}

fn eval_exists(stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>) -> ExprResult<()> {
    let value = pop_value(stack)?;
    let result = !matches!(value, SlotValue::Null);
    push_value(stack, SlotValue::Bool(result))
}

fn eval_length(stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>) -> ExprResult<()> {
    let value = pop_value(stack)?;
    let len = match value {
        SlotValue::List(id) => id.get(),
        _ => 0u32,
    };
    push_value(stack, SlotValue::I64(i64::from(len)))
}

fn eval_empty(stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>) -> ExprResult<()> {
    let value = pop_value(stack)?;
    let result = matches!(value, SlotValue::Null | SlotValue::List(_));
    push_value(stack, SlotValue::Bool(result))
}

fn push_value(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    value: SlotValue,
) -> ExprResult<()> {
    stack
        .try_push(value)
        .map_err(|_| ExprError::StackOverflow {
            max: MAX_EXPRESSION_STACK,
        })
}

fn pop_value(stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>) -> ExprResult<SlotValue> {
    stack.pop().ok_or(ExprError::StackUnderflow)
}

fn pop_pair(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
) -> ExprResult<(SlotValue, SlotValue)> {
    let right = pop_value(stack)?;
    let left = pop_value(stack)?;
    Ok((left, right))
}

fn pop_i64_pair(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
) -> ExprResult<(i64, i64)> {
    let (left, right) = pop_pair(stack)?;
    Ok((expect_i64(left)?, expect_i64(right)?))
}

fn expect_bool(value: SlotValue) -> ExprResult<bool> {
    match value {
        SlotValue::Bool(b) => Ok(b),
        other => Err(ExprError::TypeMismatch {
            expected: "boolean".into(),
            found: other.type_name().into(),
        }),
    }
}

fn expect_i64(value: SlotValue) -> ExprResult<i64> {
    match value {
        SlotValue::I64(n) => Ok(n),
        other => Err(ExprError::TypeMismatch {
            expected: "number".into(),
            found: other.type_name().into(),
        }),
    }
}

mod bool_ops {
    pub(super) const fn and(a: bool, b: bool) -> bool {
        a && b
    }
    pub(super) const fn or(a: bool, b: bool) -> bool {
        a || b
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::{ConstIdx, ExprOp, SlotIdx};

    fn make_program(ops: Vec<ExprOp>) -> ExprResult<ExprProgram> {
        ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|_| ExprError::StackOverflow {
            max: MAX_EXPRESSION_STACK,
        })
    }

    fn eval_with_const(program: &ExprProgram, constants: Vec<ConstValue>) -> ExprResult<SlotValue> {
        let slots: Vec<Option<SlotValue>> = Vec::new();
        eval_expr_program(program, &slots, &constants)
    }

    #[test]
    fn evaluates_addition() -> ExprResult<()> {
        let program = make_program(vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Add,
        ])?;
        let result = eval_with_const(&program, vec![ConstValue::I64(19), ConstValue::I64(23)])?;
        assert_eq!(result, SlotValue::I64(42));
        Ok(())
    }

    #[test]
    fn evaluates_subtraction() -> ExprResult<()> {
        let program = make_program(vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Sub,
        ])?;
        let result = eval_with_const(&program, vec![ConstValue::I64(10), ConstValue::I64(3)])?;
        assert_eq!(result, SlotValue::I64(7));
        Ok(())
    }

    #[test]
    fn evaluates_multiplication() -> ExprResult<()> {
        let program = make_program(vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Mul,
        ])?;
        let result = eval_with_const(&program, vec![ConstValue::I64(6), ConstValue::I64(7)])?;
        assert_eq!(result, SlotValue::I64(42));
        Ok(())
    }

    #[test]
    fn evaluates_division() -> ExprResult<()> {
        let program = make_program(vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Div,
        ])?;
        let result = eval_with_const(&program, vec![ConstValue::I64(42), ConstValue::I64(6)])?;
        assert_eq!(result, SlotValue::I64(7));
        Ok(())
    }

    #[test]
    fn rejects_division_by_zero() -> ExprResult<()> {
        let program = make_program(vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Div,
        ])?;
        let result = eval_with_const(&program, vec![ConstValue::I64(1), ConstValue::I64(0)]);
        assert!(matches!(result, Err(ExprError::DivisionByZero)));
        Ok(())
    }

    #[test]
    fn evaluates_equality() -> ExprResult<()> {
        let program = make_program(vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Eq,
        ])?;
        let result = eval_with_const(&program, vec![ConstValue::I64(5), ConstValue::I64(5)])?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn evaluates_inequality() -> ExprResult<()> {
        let program = make_program(vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::NotEq,
        ])?;
        let result = eval_with_const(&program, vec![ConstValue::I64(5), ConstValue::I64(3)])?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn evaluates_comparison_ops() -> ExprResult<()> {
        let constants = vec![ConstValue::I64(3), ConstValue::I64(5)];
        for (op, expected) in [
            (ExprOp::Lt, true),
            (ExprOp::Lte, true),
            (ExprOp::Gt, false),
            (ExprOp::Gte, false),
        ] {
            let program = make_program(vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                op,
            ])?;
            let result = eval_with_const(&program, constants.clone())?;
            assert_eq!(result, SlotValue::Bool(expected), "failed for {op:?}");
        }
        Ok(())
    }

    #[test]
    fn evaluates_boolean_not() -> ExprResult<()> {
        let program = make_program(vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::Not,
        ])?;
        let result = eval_with_const(&program, vec![ConstValue::Bool(true)])?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn evaluates_boolean_and_or() -> ExprResult<()> {
        let program = make_program(vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::And,
        ])?;
        let result = eval_with_const(&program, vec![ConstValue::Bool(true), ConstValue::Bool(false)])?;
        assert_eq!(result, SlotValue::Bool(false));

        let program = make_program(vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Or,
        ])?;
        let result = eval_with_const(&program, vec![ConstValue::Bool(true), ConstValue::Bool(false)])?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn evaluates_load_slot() -> ExprResult<()> {
        let program = make_program(vec![ExprOp::LoadSlot(SlotIdx::new(0))])?;
        let slots = vec![Some(SlotValue::I64(99))];
        let result = eval_expr_program(&program, &slots, &[])?;
        assert_eq!(result, SlotValue::I64(99));
        Ok(())
    }

    #[test]
    fn rejects_type_mismatch_for_arithmetic() -> ExprResult<()> {
        let program = make_program(vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Add,
        ])?;
        let result = eval_with_const(&program, vec![ConstValue::Bool(true), ConstValue::I64(1)]);
        assert!(matches!(result, Err(ExprError::TypeMismatch { .. })));
        Ok(())
    }

    #[test]
    fn end_to_end_lex_parse_compile_eval() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("3 + 4 * 2")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let _program = crate::bytecode::compile_expr_to_bytecode(&ast)?;
        let mut constants = Vec::new();
        let p2 = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&p2, &[], &constants)?;
        assert_eq!(result, SlotValue::I64(11));
        Ok(())
    }
}
