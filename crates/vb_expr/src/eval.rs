//! Bounded stack-based expression bytecode evaluator.

use arrayvec::ArrayVec;
use vb_core::limits::{MAX_EXPRESSION_STACK, MAX_EXPRESSION_STACK_USIZE};
use vb_core::{ConstValue, ExprOp, ExprProgram, SlotValue};

use crate::lexer::{BinaryOp, UnaryOp};
use crate::parser::ExprHelper;
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
        ExprOp::And => eval_binary_stack(stack, BinaryOp::And),
        ExprOp::Or => eval_binary_stack(stack, BinaryOp::Or),
        ExprOp::Not => eval_unary_stack(stack, UnaryOp::Not),
        ExprOp::Add => eval_binary_stack(stack, BinaryOp::Add),
        ExprOp::Sub => eval_binary_stack(stack, BinaryOp::Sub),
        ExprOp::Mul => eval_binary_stack(stack, BinaryOp::Mul),
        ExprOp::Div => eval_binary_stack(stack, BinaryOp::Div),
        ExprOp::Gt => eval_binary_stack(stack, BinaryOp::Gt),
        ExprOp::Gte => eval_binary_stack(stack, BinaryOp::Gte),
        ExprOp::Lt => eval_binary_stack(stack, BinaryOp::Lt),
        ExprOp::Lte => eval_binary_stack(stack, BinaryOp::Lte),
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
    let value = constant
        .to_slot_value()
        .map_err(|_| ExprError::UnexpectedEof)?;
    push_value(stack, value)
}

fn eval_eq(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    positive: bool,
) -> ExprResult<()> {
    let (left, right) = pop_pair(stack)?;
    push_value(stack, SlotValue::Bool((left == right) == positive))
}

fn eval_binary_stack(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    op: BinaryOp,
) -> ExprResult<()> {
    let (left, right) = pop_pair(stack)?;
    let value = eval_binary_op(op, left, right)?;
    push_value(stack, value)
}

fn eval_unary_stack(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    op: UnaryOp,
) -> ExprResult<()> {
    let value = pop_value(stack)?;
    let result = eval_unary_op(op, value)?;
    push_value(stack, result)
}

/// Evaluates one binary operation over two already-popped values.
pub fn eval_binary_op(op: BinaryOp, left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match op {
        BinaryOp::And => Ok(SlotValue::Bool(expect_bool(left)? && expect_bool(right)?)),
        BinaryOp::Or => Ok(SlotValue::Bool(expect_bool(left)? || expect_bool(right)?)),
        BinaryOp::Eq => Ok(SlotValue::Bool(left == right)),
        BinaryOp::NotEq => Ok(SlotValue::Bool(left != right)),
        BinaryOp::Add => eval_i64_values(left, right, i64::checked_add),
        BinaryOp::Sub => eval_i64_values(left, right, i64::checked_sub),
        BinaryOp::Mul => eval_i64_values(left, right, i64::checked_mul),
        BinaryOp::Div => eval_div_values(left, right),
        BinaryOp::Gt => eval_i64_cmp_values(left, right, i64::gt),
        BinaryOp::Gte => eval_i64_cmp_values(left, right, i64::ge),
        BinaryOp::Lt => eval_i64_cmp_values(left, right, i64::lt),
        BinaryOp::Lte => eval_i64_cmp_values(left, right, i64::le),
    }
}

/// Evaluates one unary operation over an already-popped value.
pub fn eval_unary_op(op: UnaryOp, value: SlotValue) -> ExprResult<SlotValue> {
    match op {
        UnaryOp::Not => Ok(SlotValue::Bool(!expect_bool(value)?)),
        UnaryOp::Neg => {
            let number = expect_i64(value)?;
            let negated = number.checked_neg().ok_or(ExprError::UnexpectedEof)?;
            Ok(SlotValue::I64(negated))
        }
    }
}

fn eval_i64_values(
    left: SlotValue,
    right: SlotValue,
    op: fn(i64, i64) -> Option<i64>,
) -> ExprResult<SlotValue> {
    let value = op(expect_i64(left)?, expect_i64(right)?).ok_or(ExprError::UnexpectedEof)?;
    Ok(SlotValue::I64(value))
}

fn eval_div_values(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    let left_i64 = expect_i64(left)?;
    let right_i64 = expect_i64(right)?;
    let value = left_i64
        .checked_div(right_i64)
        .ok_or(ExprError::DivisionByZero)?;
    Ok(SlotValue::I64(value))
}

fn eval_i64_cmp_values(
    left: SlotValue,
    right: SlotValue,
    op: fn(&i64, &i64) -> bool,
) -> ExprResult<SlotValue> {
    let left_i64 = expect_i64(left)?;
    let right_i64 = expect_i64(right)?;
    Ok(SlotValue::Bool(op(&left_i64, &right_i64)))
}

fn eval_helper_op(
    op: ExprOp,
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
) -> ExprResult<()> {
    match op {
        ExprOp::Exists => eval_helper_stack(stack, ExprHelper::Exists),
        ExprOp::Length => eval_helper_stack(stack, ExprHelper::Length),
        ExprOp::Empty => eval_helper_stack(stack, ExprHelper::Empty),
        ExprOp::Count => eval_helper_stack(stack, ExprHelper::Count),
        ExprOp::Unique => eval_helper_stack(stack, ExprHelper::Unique),
        _ => Err(ExprError::UnknownOperator {
            op: format!("{op:?}"),
        }),
    }
}

fn eval_helper_stack(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    helper: ExprHelper,
) -> ExprResult<()> {
    let value = pop_value(stack)?;
    let args = [value];
    let result = eval_helper(helper, &args)?;
    push_value(stack, result)
}

/// Evaluates helper behavior that is local to scalar/handle values.
pub fn eval_helper(helper: ExprHelper, args: &[SlotValue]) -> ExprResult<SlotValue> {
    match helper {
        ExprHelper::Exists => eval_helper_exists(args),
        ExprHelper::Length | ExprHelper::Count => eval_helper_length(args),
        ExprHelper::Empty => eval_helper_empty(args),
        ExprHelper::Unique => eval_helper_unique(args),
        _ => Err(ExprError::UnknownHelper {
            helper: crate::parser::helper_name(helper).into(),
        }),
    }
}

fn one_arg(args: &[SlotValue], helper: ExprHelper) -> ExprResult<&SlotValue> {
    if args.len() != 1 {
        return Err(ExprError::HelperArityMismatch {
            helper: crate::parser::helper_name(helper).into(),
            expected: 1,
            actual: args.len(),
        });
    }
    args.first().ok_or(ExprError::StackUnderflow)
}

fn eval_helper_exists(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let value = one_arg(args, ExprHelper::Exists)?;
    Ok(SlotValue::Bool(!matches!(*value, SlotValue::Null)))
}

fn eval_helper_length(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let value = one_arg(args, ExprHelper::Length)?;
    let len = match value {
        SlotValue::List(id) => id.get(),
        _ => 0u32,
    };
    Ok(SlotValue::I64(i64::from(len)))
}

fn eval_helper_empty(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let value = one_arg(args, ExprHelper::Empty)?;
    let result = matches!(*value, SlotValue::Null | SlotValue::List(_));
    Ok(SlotValue::Bool(result))
}

fn eval_helper_unique(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let value = one_arg(args, ExprHelper::Unique)?;
    match *value {
        SlotValue::List(_) => Ok(*value),
        other => Err(ExprError::TypeMismatch {
            expected: "list".into(),
            found: other.type_name().into(),
        }),
    }
}

fn push_value(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    value: SlotValue,
) -> ExprResult<()> {
    stack.try_push(value).map_err(|_| ExprError::StackOverflow {
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
        let program = make_program(vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Not])?;
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
        let result = eval_with_const(
            &program,
            vec![ConstValue::Bool(true), ConstValue::Bool(false)],
        )?;
        assert_eq!(result, SlotValue::Bool(false));

        let program = make_program(vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Or,
        ])?;
        let result = eval_with_const(
            &program,
            vec![ConstValue::Bool(true), ConstValue::Bool(false)],
        )?;
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
    fn public_binary_eval_matches_stack_arithmetic() -> ExprResult<()> {
        let result = eval_binary_op(BinaryOp::Add, SlotValue::I64(20), SlotValue::I64(22))?;
        assert_eq!(result, SlotValue::I64(42));
        Ok(())
    }

    #[test]
    fn public_unary_eval_rejects_wrong_type() {
        let result = eval_unary_op(UnaryOp::Not, SlotValue::I64(1));
        assert!(matches!(result, Err(ExprError::TypeMismatch { .. })));
    }

    #[test]
    fn public_helper_eval_supports_scalar_exists() -> ExprResult<()> {
        let args = [SlotValue::Null];
        let result = eval_helper(ExprHelper::Exists, &args)?;
        assert_eq!(result, SlotValue::Bool(false));
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
