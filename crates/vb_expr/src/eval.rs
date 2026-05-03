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
            let negated = number.checked_neg().ok_or(ExprError::IntegerOverflow)?;
            Ok(SlotValue::I64(negated))
        }
    }
}

fn eval_i64_values(
    left: SlotValue,
    right: SlotValue,
    op: fn(i64, i64) -> Option<i64>,
) -> ExprResult<SlotValue> {
    let value = op(expect_i64(left)?, expect_i64(right)?).ok_or(ExprError::IntegerOverflow)?;
    Ok(SlotValue::I64(value))
}

fn eval_div_values(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    let left_i64 = expect_i64(left)?;
    let right_i64 = expect_i64(right)?;
    if right_i64 == 0 {
        return Err(ExprError::DivisionByZero);
    }
    let value = left_i64
        .checked_div(right_i64)
        .ok_or(ExprError::IntegerOverflow)?;
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
        ExprOp::Exists => eval_helper_stack_1(stack, ExprHelper::Exists),
        ExprOp::Length => eval_helper_stack_1(stack, ExprHelper::Length),
        ExprOp::Empty => eval_helper_stack_1(stack, ExprHelper::Empty),
        ExprOp::Count => eval_helper_stack_1(stack, ExprHelper::Count),
        ExprOp::Unique => eval_helper_stack_1(stack, ExprHelper::Unique),
        ExprOp::Contains => eval_helper_stack_2(stack, ExprHelper::Contains),
        ExprOp::StartsWith => eval_helper_stack_2(stack, ExprHelper::StartsWith),
        ExprOp::EndsWith => eval_helper_stack_2(stack, ExprHelper::EndsWith),
        ExprOp::Has => eval_helper_stack_2(stack, ExprHelper::Has),
        ExprOp::Append => eval_helper_stack_2(stack, ExprHelper::Append),
        ExprOp::AppendIf => eval_helper_stack_3(stack, ExprHelper::AppendIf),
        ExprOp::Merge => eval_helper_stack_2(stack, ExprHelper::Merge),
        ExprOp::Sum => eval_helper_stack_1(stack, ExprHelper::Sum),
        _ => Err(ExprError::UnknownOperator {
            op: format!("{op:?}"),
        }),
    }
}

fn eval_helper_stack_1(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    helper: ExprHelper,
) -> ExprResult<()> {
    let value = pop_value(stack)?;
    let result = eval_helper(helper, &[value])?;
    push_value(stack, result)
}

fn eval_helper_stack_2(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    helper: ExprHelper,
) -> ExprResult<()> {
    let (right, left) = pop_pair(stack)?;
    let result = eval_helper(helper, &[left, right])?;
    push_value(stack, result)
}

fn eval_helper_stack_3(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    helper: ExprHelper,
) -> ExprResult<()> {
    let (third, second, first) = pop_triple(stack)?;
    let result = eval_helper(helper, &[first, second, third])?;
    push_value(stack, result)
}

/// Evaluates helper behavior that is local to scalar/handle values.
pub fn eval_helper(helper: ExprHelper, args: &[SlotValue]) -> ExprResult<SlotValue> {
    match helper {
        ExprHelper::Exists => eval_helper_exists(args),
        ExprHelper::Length | ExprHelper::Count => eval_helper_length(args),
        ExprHelper::Empty => eval_helper_empty(args),
        ExprHelper::Unique => eval_helper_unique(args),
        ExprHelper::Contains => eval_helper_contains(args),
        ExprHelper::StartsWith => eval_helper_starts_with(args),
        ExprHelper::EndsWith => eval_helper_ends_with(args),
        ExprHelper::Has => eval_helper_has(args),
        ExprHelper::Append => eval_helper_append(args),
        ExprHelper::AppendIf => eval_helper_append_if(args),
        ExprHelper::Merge => eval_helper_merge(args),
        ExprHelper::Sum => eval_helper_sum(args),
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

fn two_args(args: &[SlotValue], helper: ExprHelper) -> ExprResult<(&SlotValue, &SlotValue)> {
    if args.len() != 2 {
        return Err(ExprError::HelperArityMismatch {
            helper: crate::parser::helper_name(helper).into(),
            expected: 2,
            actual: args.len(),
        });
    }
    let left = args.first().ok_or(ExprError::StackUnderflow)?;
    let right = args.get(1).ok_or(ExprError::StackUnderflow)?;
    Ok((left, right))
}

fn three_args(
    args: &[SlotValue],
    helper: ExprHelper,
) -> ExprResult<(&SlotValue, &SlotValue, &SlotValue)> {
    if args.len() != 3 {
        return Err(ExprError::HelperArityMismatch {
            helper: crate::parser::helper_name(helper).into(),
            expected: 3,
            actual: args.len(),
        });
    }
    let first = args.first().ok_or(ExprError::StackUnderflow)?;
    let second = args.get(1).ok_or(ExprError::StackUnderflow)?;
    let third = args.get(2).ok_or(ExprError::StackUnderflow)?;
    Ok((first, second, third))
}

fn eval_helper_exists(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let value = one_arg(args, ExprHelper::Exists)?;
    Ok(SlotValue::Bool(!matches!(*value, SlotValue::Null)))
}

fn eval_helper_length(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let value = one_arg(args, ExprHelper::Length)?;
    match value {
        SlotValue::List(_) | SlotValue::Null => Err(ExprError::TypeMismatch {
            expected: "value-store context required for list length".into(),
            found: "list handle without store".into(),
        }),
        other => Err(ExprError::TypeMismatch {
            expected: "list".into(),
            found: other.type_name().into(),
        }),
    }
}

fn eval_helper_empty(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let value = one_arg(args, ExprHelper::Empty)?;
    match *value {
        SlotValue::Null => Ok(SlotValue::Bool(true)),
        SlotValue::List(_) => Err(ExprError::TypeMismatch {
            expected: "value-store context required for list emptiness check".into(),
            found: "list handle without store".into(),
        }),
        other => Err(ExprError::TypeMismatch {
            expected: "list or null".into(),
            found: other.type_name().into(),
        }),
    }
}

fn eval_helper_unique(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let value = one_arg(args, ExprHelper::Unique)?;
    match *value {
        SlotValue::List(_) => Err(ExprError::TypeMismatch {
            expected: "value-store context required for list deduplication".into(),
            found: "list handle without store".into(),
        }),
        other => Err(ExprError::TypeMismatch {
            expected: "list".into(),
            found: other.type_name().into(),
        }),
    }
}

fn eval_helper_contains(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let (list_val, _item_val) = two_args(args, ExprHelper::Contains)?;
    let _list_id = expect_list(*list_val)?;
    Err(ExprError::TypeMismatch {
        expected: "value-store context required for list contains check".into(),
        found: "list handle without store".into(),
    })
}

fn eval_helper_starts_with(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let (text_val, prefix_val) = two_args(args, ExprHelper::StartsWith)?;
    let _text_id = expect_symbol(*text_val)?;
    let _prefix_id = expect_symbol(*prefix_val)?;
    Err(ExprError::TypeMismatch {
        expected: "value-store context required for text operations".into(),
        found: "symbol handle without store".into(),
    })
}

fn eval_helper_ends_with(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let (text_val, suffix_val) = two_args(args, ExprHelper::EndsWith)?;
    let _text_id = expect_symbol(*text_val)?;
    let _suffix_id = expect_symbol(*suffix_val)?;
    Err(ExprError::TypeMismatch {
        expected: "value-store context required for text operations".into(),
        found: "symbol handle without store".into(),
    })
}

fn eval_helper_has(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let (obj_val, key_val) = two_args(args, ExprHelper::Has)?;
    let _obj_id = expect_object(*obj_val)?;
    let _key_id = expect_symbol(*key_val)?;
    Err(ExprError::TypeMismatch {
        expected: "value-store context required for object field lookup".into(),
        found: "object handle without store".into(),
    })
}

fn eval_helper_append(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let (list_val, _item_val) = two_args(args, ExprHelper::Append)?;
    let _list_id = expect_list(*list_val)?;
    Err(ExprError::TypeMismatch {
        expected: "value-store context required for list append".into(),
        found: "list handle without store".into(),
    })
}

fn eval_helper_append_if(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let (list_val, _item_val, cond_val) = three_args(args, ExprHelper::AppendIf)?;
    let _list_id = expect_list(*list_val)?;
    let _ = expect_bool(*cond_val)?;
    Err(ExprError::TypeMismatch {
        expected: "value-store context required for list append".into(),
        found: "list handle without store".into(),
    })
}

fn eval_helper_merge(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let (left_val, right_val) = two_args(args, ExprHelper::Merge)?;
    let _left_id = expect_object(*left_val)?;
    let _right_id = expect_object(*right_val)?;
    Err(ExprError::TypeMismatch {
        expected: "value-store context required for object merge".into(),
        found: "object handle without store".into(),
    })
}

fn eval_helper_sum(args: &[SlotValue]) -> ExprResult<SlotValue> {
    let value = one_arg(args, ExprHelper::Sum)?;
    let _list_id = expect_list(*value)?;
    Err(ExprError::TypeMismatch {
        expected: "value-store context required for list sum".into(),
        found: "list handle without store".into(),
    })
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

fn pop_triple(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
) -> ExprResult<(SlotValue, SlotValue, SlotValue)> {
    let third = pop_value(stack)?;
    let second = pop_value(stack)?;
    let first = pop_value(stack)?;
    Ok((first, second, third))
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

fn expect_symbol(value: SlotValue) -> ExprResult<vb_core::ids::SymbolId> {
    match value {
        SlotValue::Symbol(id) => Ok(id),
        other => Err(ExprError::TypeMismatch {
            expected: "text".into(),
            found: other.type_name().into(),
        }),
    }
}

fn expect_list(value: SlotValue) -> ExprResult<vb_core::ids::ListId> {
    match value {
        SlotValue::List(id) => Ok(id),
        other => Err(ExprError::TypeMismatch {
            expected: "list".into(),
            found: other.type_name().into(),
        }),
    }
}

fn expect_object(value: SlotValue) -> ExprResult<vb_core::ids::ObjectId> {
    match value {
        SlotValue::Object(id) => Ok(id),
        other => Err(ExprError::TypeMismatch {
            expected: "object".into(),
            found: other.type_name().into(),
        }),
    }
}

#[cfg(test)]
#[allow(clippy::panic_in_result_fn)]
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

    // --- BDD evaluator tests ---

    #[test]
    fn eval_binary_op_adds_two_numbers() -> ExprResult<()> {
        // Given: two SlotValue::I64 values (10, 32)
        // When: eval_binary_op is called with Add
        // Then: the result is SlotValue::I64(42)
        let result = eval_binary_op(BinaryOp::Add, SlotValue::I64(10), SlotValue::I64(32))?;
        assert_eq!(result, SlotValue::I64(42));
        Ok(())
    }

    #[test]
    fn eval_binary_op_subtracts_two_numbers() -> ExprResult<()> {
        // Given: two SlotValue::I64 values (100, 58)
        // When: eval_binary_op is called with Sub
        // Then: the result is SlotValue::I64(42)
        let result = eval_binary_op(BinaryOp::Sub, SlotValue::I64(100), SlotValue::I64(58))?;
        assert_eq!(result, SlotValue::I64(42));
        Ok(())
    }

    #[test]
    fn eval_binary_op_multiplies_two_numbers() -> ExprResult<()> {
        // Given: two SlotValue::I64 values (6, 7)
        // When: eval_binary_op is called with Mul
        // Then: the result is SlotValue::I64(42)
        let result = eval_binary_op(BinaryOp::Mul, SlotValue::I64(6), SlotValue::I64(7))?;
        assert_eq!(result, SlotValue::I64(42));
        Ok(())
    }

    #[test]
    fn eval_binary_op_divides_two_numbers() -> ExprResult<()> {
        // Given: two SlotValue::I64 values (84, 2)
        // When: eval_binary_op is called with Div
        // Then: the result is SlotValue::I64(42)
        let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(84), SlotValue::I64(2))?;
        assert_eq!(result, SlotValue::I64(42));
        Ok(())
    }

    #[test]
    fn eval_binary_op_compares_equality() -> ExprResult<()> {
        // Given: two SlotValue::I64 values (7, 7)
        // When: eval_binary_op is called with Eq
        // Then: the result is SlotValue::Bool(true)
        let result = eval_binary_op(BinaryOp::Eq, SlotValue::I64(7), SlotValue::I64(7))?;
        assert_eq!(result, SlotValue::Bool(true));

        let result_ne = eval_binary_op(BinaryOp::Eq, SlotValue::I64(7), SlotValue::I64(8))?;
        assert_eq!(result_ne, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn eval_binary_op_compares_less_than() -> ExprResult<()> {
        // Given: two SlotValue::I64 values (3, 5)
        // When: eval_binary_op is called with Lt
        // Then: the result is SlotValue::Bool(true)
        let result = eval_binary_op(BinaryOp::Lt, SlotValue::I64(3), SlotValue::I64(5))?;
        assert_eq!(result, SlotValue::Bool(true));

        let result_false = eval_binary_op(BinaryOp::Lt, SlotValue::I64(5), SlotValue::I64(3))?;
        assert_eq!(result_false, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn eval_unary_op_negates_number() -> ExprResult<()> {
        // Given: a SlotValue::I64(42)
        // When: eval_unary_op is called with Neg
        // Then: the result is SlotValue::I64(-42)
        let result = eval_unary_op(UnaryOp::Neg, SlotValue::I64(42))?;
        assert_eq!(result, SlotValue::I64(-42));
        Ok(())
    }

    #[test]
    fn eval_unary_op_not_negates_boolean() -> ExprResult<()> {
        // Given: a SlotValue::Bool(true)
        // When: eval_unary_op is called with Not
        // Then: the result is SlotValue::Bool(false)
        let result = eval_unary_op(UnaryOp::Not, SlotValue::Bool(true))?;
        assert_eq!(result, SlotValue::Bool(false));

        let result_false = eval_unary_op(UnaryOp::Not, SlotValue::Bool(false))?;
        assert_eq!(result_false, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_helper_applies_known_helper_exists() -> ExprResult<()> {
        // Given: a SlotValue::Null argument
        // When: eval_helper is called with Exists
        // Then: the result is SlotValue::Bool(false)
        let args = [SlotValue::Null];
        let result = eval_helper(ExprHelper::Exists, &args)?;
        assert_eq!(result, SlotValue::Bool(false));

        let args_non_null = [SlotValue::I64(1)];
        let result_non_null = eval_helper(ExprHelper::Exists, &args_non_null)?;
        assert_eq!(result_non_null, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_expr_program_evaluates_simple_expression() -> ExprResult<()> {
        // Given: the source "2 * 3 + 4"
        // When: lex -> parse -> compile with pool -> eval
        // Then: the result is SlotValue::I64(10)
        let tokens = crate::lexer::lex_expr("2 * 3 + 4")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::I64(10));
        Ok(())
    }

    #[test]
    fn eval_binary_op_returns_type_mismatch_for_string_in_arithmetic() -> ExprResult<()> {
        // Given: a Bool and an I64 value
        // When: eval_binary_op is called with Add
        // Then: the result is Err(TypeMismatch { expected: "number", found: "boolean" })
        let result = eval_binary_op(BinaryOp::Add, SlotValue::Bool(true), SlotValue::I64(1));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch".into(),
            });
        };
        assert_eq!(expected, "number");
        assert_eq!(found, "boolean");
        Ok(())
    }

    #[test]
    fn eval_expr_program_returns_stack_overflow_for_deep_nesting() -> ExprResult<()> {
        // Given: a program with more than MAX_EXPRESSION_STACK values on stack
        // When: program construction is attempted
        // Then: the result is Err(StackOverflow { max: 64 })
        let mut ops = Vec::new();
        for i in 0..65u16 {
            ops.push(ExprOp::LoadConst(ConstIdx::new(i)));
        }
        let result = make_program(ops);
        let Err(ExprError::StackOverflow { max }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected StackOverflow".into(),
            });
        };
        assert_eq!(max, vb_core::limits::MAX_EXPRESSION_STACK);
        Ok(())
    }

    #[test]
    fn eval_expr_program_returns_stack_underflow_for_empty_stack_op() -> ExprResult<()> {
        // Given: a program with a single binary op and no operands
        // When: eval_expr_program is called
        // Then: the result is Err(StackUnderflow)
        let program = ExprProgram {
            ops: vec![ExprOp::Add].into_boxed_slice(),
            max_stack: 0,
        };
        let result = eval_expr_program(&program, &[], &[]);
        let Err(ExprError::StackUnderflow) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected StackUnderflow".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_returns_division_by_zero() -> ExprResult<()> {
        // Given: two SlotValue::I64 values (10, 0)
        // When: eval_binary_op is called with Div
        // Then: the result is Err(DivisionByZero)
        let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(10), SlotValue::I64(0));
        let Err(ExprError::DivisionByZero) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected DivisionByZero".into(),
            });
        };
        Ok(())
    }

    // --- Adversarial BDD evaluator tests ---

    #[test]
    fn eval_binary_op_i64_max_plus_one_is_error() -> ExprResult<()> {
        // Given: i64::MAX and 1 as SlotValue::I64
        // When: eval_binary_op is called with Add
        // Then: the result is Err(IntegerOverflow) (overflow from checked_add)
        let result = eval_binary_op(BinaryOp::Add, SlotValue::I64(i64::MAX), SlotValue::I64(1));
        let Err(ExprError::IntegerOverflow) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected IntegerOverflow for i64::MAX + 1".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_i64_min_minus_one_is_error() -> ExprResult<()> {
        // Given: i64::MIN and 1 as SlotValue::I64
        // When: eval_binary_op is called with Sub
        // Then: the result is Err(IntegerOverflow) (underflow from checked_sub)
        let result = eval_binary_op(BinaryOp::Sub, SlotValue::I64(i64::MIN), SlotValue::I64(1));
        let Err(ExprError::IntegerOverflow) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected IntegerOverflow for i64::MIN - 1".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_i64_max_times_two_is_error() -> ExprResult<()> {
        // Given: i64::MAX and 2 as SlotValue::I64
        // When: eval_binary_op is called with Mul
        // Then: the result is Err(IntegerOverflow) (overflow from checked_mul)
        let result = eval_binary_op(BinaryOp::Mul, SlotValue::I64(i64::MAX), SlotValue::I64(2));
        let Err(ExprError::IntegerOverflow) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected IntegerOverflow for i64::MAX * 2".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_negation_of_i64_min_is_error() -> ExprResult<()> {
        // Given: SlotValue::I64(i64::MIN)
        // When: eval_unary_op is called with Neg
        // Then: the result is Err(IntegerOverflow) (checked_neg fails for MIN)
        let result = eval_unary_op(UnaryOp::Neg, SlotValue::I64(i64::MIN));
        let Err(ExprError::IntegerOverflow) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected IntegerOverflow for -i64::MIN".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_rejects_null_in_addition() -> ExprResult<()> {
        // Given: SlotValue::Null and SlotValue::I64(1)
        // When: eval_binary_op is called with Add
        // Then: the result is Err(TypeMismatch { expected: "number", found: "null" })
        let result = eval_binary_op(BinaryOp::Add, SlotValue::Null, SlotValue::I64(1));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for null + 1".into(),
            });
        };
        assert_eq!(expected, "number");
        assert_eq!(found, "null");
        Ok(())
    }

    #[test]
    fn eval_binary_op_rejects_bool_in_multiplication() -> ExprResult<()> {
        // Given: SlotValue::Bool(true) and SlotValue::I64(3)
        // When: eval_binary_op is called with Mul
        // Then: the result is Err(TypeMismatch { expected: "number", found: "boolean" })
        let result = eval_binary_op(BinaryOp::Mul, SlotValue::Bool(true), SlotValue::I64(3));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for bool * int".into(),
            });
        };
        assert_eq!(expected, "number");
        assert_eq!(found, "boolean");
        Ok(())
    }

    #[test]
    fn eval_binary_op_rejects_number_in_and() -> ExprResult<()> {
        // Given: SlotValue::I64(1) and SlotValue::I64(2)
        // When: eval_binary_op is called with And
        // Then: the result is Err(TypeMismatch { expected: "boolean", found: "number" })
        let result = eval_binary_op(BinaryOp::And, SlotValue::I64(1), SlotValue::I64(2));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for i64 and i64".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn eval_binary_op_rejects_null_in_or() -> ExprResult<()> {
        // Given: SlotValue::Null and SlotValue::Bool(true)
        // When: eval_binary_op is called with Or
        // Then: the result is Err(TypeMismatch { expected: "boolean", found: "null" })
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Null, SlotValue::Bool(true));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for null or true".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "null");
        Ok(())
    }

    #[test]
    fn eval_unary_op_not_rejects_i64() -> ExprResult<()> {
        // Given: SlotValue::I64(1)
        // When: eval_unary_op is called with Not
        // Then: the result is Err(TypeMismatch { expected: "boolean", found: "number" })
        let result = eval_unary_op(UnaryOp::Not, SlotValue::I64(1));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for not 1".into(),
            });
        };
        assert_eq!(expected, "boolean");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn eval_unary_op_neg_rejects_bool() -> ExprResult<()> {
        // Given: SlotValue::Bool(true)
        // When: eval_unary_op is called with Neg
        // Then: the result is Err(TypeMismatch { expected: "number", found: "boolean" })
        let result = eval_unary_op(UnaryOp::Neg, SlotValue::Bool(true));
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for -true".into(),
            });
        };
        assert_eq!(expected, "number");
        assert_eq!(found, "boolean");
        Ok(())
    }

    #[test]
    fn eval_expr_program_end_to_end_division_by_zero() -> ExprResult<()> {
        // Given: the source "10 / 0"
        // When: lex -> parse -> compile with pool -> eval
        // Then: the result is Err(DivisionByZero)
        let tokens = crate::lexer::lex_expr("10 / 0")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::DivisionByZero) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected DivisionByZero for 10 / 0".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_expr_program_end_to_end_overflow() -> ExprResult<()> {
        // Given: the source "9223372036854775807 + 1"
        // When: lex -> parse -> compile with pool -> eval
        // Then: the result is Err(IntegerOverflow) (overflow)
        let tokens = crate::lexer::lex_expr("9223372036854775807 + 1")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::IntegerOverflow) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected IntegerOverflow for i64::MAX + 1".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_expr_program_equality_null_vs_null() -> ExprResult<()> {
        // Given: the source "null == null"
        // When: lex -> parse -> compile with pool -> eval
        // Then: the result is SlotValue::Bool(true)
        let tokens = crate::lexer::lex_expr("null == null")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_expr_program_inequality_null_vs_i64() -> ExprResult<()> {
        // Given: the source "null != 1"
        // When: lex -> parse -> compile with pool -> eval
        // Then: the result is SlotValue::Bool(true)
        let tokens = crate::lexer::lex_expr("null != 1")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_expr_program_boolean_and_type_mismatch() -> ExprResult<()> {
        // Given: the source "true and 1"
        // When: lex -> parse -> compile with pool -> eval
        // Then: the result is Err(TypeMismatch)
        // Note: typecheck would catch this, but if someone bypasses typecheck
        // the eval layer still enforces it
        let tokens = crate::lexer::lex_expr("true and 1")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants);
        assert!(
            matches!(result, Err(ExprError::TypeMismatch { .. })),
            "true and 1 should fail with TypeMismatch at eval"
        );
        Ok(())
    }

    #[test]
    fn eval_expr_program_chained_not_true() -> ExprResult<()> {
        // Given: the source "not not true"
        // When: lex -> parse -> compile with pool -> eval
        // Then: the result is SlotValue::Bool(true)
        let tokens = crate::lexer::lex_expr("not not true")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_expr_program_double_negation() -> ExprResult<()> {
        // Given: the source "--5"
        // When: lex -> parse -> compile with pool -> eval
        // Then: the result is SlotValue::I64(5) (double negation returns original)
        let tokens = crate::lexer::lex_expr("--5")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, SlotValue::I64(5));
        Ok(())
    }

    #[test]
    fn eval_load_const_out_of_bounds_returns_error() -> ExprResult<()> {
        // Given: a program with LoadConst(ConstIdx::new(99)) and empty constants
        // When: eval_expr_program is called
        // Then: the result is Err(UnexpectedEof) (constant index out of bounds)
        let program = ExprProgram {
            ops: vec![ExprOp::LoadConst(ConstIdx::new(99))].into_boxed_slice(),
            max_stack: 1,
        };
        let result = eval_expr_program(&program, &[], &[]);
        assert!(
            result.is_err(),
            "LoadConst with out-of-bounds index should fail"
        );
        Ok(())
    }

    #[test]
    fn eval_load_slot_out_of_bounds_returns_error() -> ExprResult<()> {
        // Given: a program with LoadSlot(SlotIdx::new(99)) and empty slots
        // When: eval_expr_program is called
        // Then: the result is Err(StackUnderflow) (slot index out of bounds)
        let program = ExprProgram {
            ops: vec![ExprOp::LoadSlot(SlotIdx::new(99))].into_boxed_slice(),
            max_stack: 1,
        };
        let slots: Vec<Option<SlotValue>> = vec![];
        let result = eval_expr_program(&program, &slots, &[]);
        assert!(
            result.is_err(),
            "LoadSlot with out-of-bounds index should fail"
        );
        Ok(())
    }

    #[test]
    fn eval_helper_unique_rejects_non_list() -> ExprResult<()> {
        // Given: a SlotValue::I64(42) argument
        // When: eval_helper is called with Unique
        // Then: the result is Err(TypeMismatch { expected: "list", found: "number" })
        let args = [SlotValue::I64(42)];
        let result = eval_helper(ExprHelper::Unique, &args);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for unique(42)".into(),
            });
        };
        assert_eq!(expected, "list");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn eval_helper_length_returns_type_mismatch_for_non_list() -> ExprResult<()> {
        // Given: a SlotValue::I64(42) argument
        // When: eval_helper is called with Length
        // Then: the result is Err(TypeMismatch { expected: "list", found: "number" })
        let args = [SlotValue::I64(42)];
        let result = eval_helper(ExprHelper::Length, &args);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for length(42)".into(),
            });
        };
        assert_eq!(expected, "list");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn eval_helper_empty_returns_true_for_null() -> ExprResult<()> {
        // Given: a SlotValue::Null argument
        // When: eval_helper is called with Empty
        // Then: the result is Ok(SlotValue::Bool(true))
        let args = [SlotValue::Null];
        let result = eval_helper(ExprHelper::Empty, &args)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn eval_helper_empty_returns_type_mismatch_for_i64() -> ExprResult<()> {
        // Given: a SlotValue::I64(42) argument
        // When: eval_helper is called with Empty
        // Then: the result is Err(TypeMismatch) (non-null, non-list => type error)
        let args = [SlotValue::I64(42)];
        let result = eval_helper(ExprHelper::Empty, &args);
        let Err(ExprError::TypeMismatch { expected, found }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected TypeMismatch for empty(42)".into(),
            });
        };
        assert_eq!(expected, "list or null");
        assert_eq!(found, "number");
        Ok(())
    }

    #[test]
    fn eval_helper_contains_returns_unknown_operator() -> ExprResult<()> {
        // Given: Contains is a multi-arg helper not handled in eval_helper_op
        // When: eval_expr_op encounters ExprOp::Contains
        // Then: the result is Err(UnknownOperator) via the helper dispatch
        let program = ExprProgram {
            ops: vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Contains,
            ]
            .into_boxed_slice(),
            max_stack: 2,
        };
        let constants = vec![ConstValue::I64(1), ConstValue::I64(2)];
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::UnknownOperator { op }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected UnknownOperator for Contains".into(),
            });
        };
        assert!(
            op.contains("Contains"),
            "op should mention Contains, got: {op}"
        );
        Ok(())
    }

    #[test]
    fn eval_helper_append_returns_unknown_operator() -> ExprResult<()> {
        // Given: Append is a multi-arg helper not handled in eval_helper_op
        // When: eval_expr_op encounters ExprOp::Append
        // Then: the result is Err(UnknownOperator)
        let program = ExprProgram {
            ops: vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Append,
            ]
            .into_boxed_slice(),
            max_stack: 2,
        };
        let constants = vec![ConstValue::I64(1), ConstValue::I64(2)];
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::UnknownOperator { op }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected UnknownOperator for Append".into(),
            });
        };
        assert!(op.contains("Append"), "op should mention Append, got: {op}");
        Ok(())
    }

    #[test]
    fn eval_helper_merge_returns_unknown_operator() -> ExprResult<()> {
        // Given: Merge is a multi-arg helper not handled in eval_helper_op
        // When: eval_expr_op encounters ExprOp::Merge
        // Then: the result is Err(UnknownOperator)
        let program = ExprProgram {
            ops: vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Merge,
            ]
            .into_boxed_slice(),
            max_stack: 2,
        };
        let constants = vec![ConstValue::I64(1), ConstValue::I64(2)];
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::UnknownOperator { op }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected UnknownOperator for Merge".into(),
            });
        };
        assert!(op.contains("Merge"), "op should mention Merge, got: {op}");
        Ok(())
    }

    #[test]
    fn eval_program_with_only_load_const_no_ops_returns_stack_overflow() -> ExprResult<()> {
        // Given: a program with two LoadConst ops but no binary op to consume them
        // When: eval_expr_program is called
        // Then: the result is Err(StackOverflow) because 2 values remain on the stack
        let program = ExprProgram {
            ops: vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
            ]
            .into_boxed_slice(),
            max_stack: 2,
        };
        let constants = vec![ConstValue::I64(1), ConstValue::I64(2)];
        let result = eval_expr_program(&program, &[], &constants);
        // finish_stack checks stack.len() == 1, else StackOverflow
        let Err(ExprError::StackOverflow { max }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected StackOverflow for extra values".into(),
            });
        };
        assert_eq!(max, vb_core::limits::MAX_EXPRESSION_STACK);
        Ok(())
    }

    // ===== Security regression tests =====

    #[test]
    fn eval_binary_op_i64_min_div_neg_one_is_integer_overflow_not_division_by_zero()
    -> ExprResult<()> {
        // SECURITY: i64::MIN / -1 overflows (mathematical result exceeds i64::MAX).
        // Previously, checked_div mapped None -> DivisionByZero, which is incorrect.
        // The fix checks for zero explicitly and maps overflow to IntegerOverflow.
        let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(i64::MIN), SlotValue::I64(-1));
        let Err(ExprError::IntegerOverflow) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected IntegerOverflow for i64::MIN / -1".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_binary_op_div_by_zero_still_returns_division_by_zero() -> ExprResult<()> {
        // Ensure the fix does not regress the legitimate division-by-zero path.
        let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(10), SlotValue::I64(0));
        let Err(ExprError::DivisionByZero) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected DivisionByZero for 10 / 0".into(),
            });
        };
        Ok(())
    }

    #[test]
    fn eval_expr_program_i64_min_div_neg_one_is_integer_overflow() -> ExprResult<()> {
        // SECURITY: end-to-end test that i64::MIN / -1 returns IntegerOverflow,
        // not DivisionByZero. We cannot parse i64::MIN as a literal directly since
        // the positive value overflows i64, so we construct a program manually.
        let program = ExprProgram {
            ops: vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Div,
            ]
            .into_boxed_slice(),
            max_stack: 2,
        };
        let constants = vec![ConstValue::I64(i64::MIN), ConstValue::I64(-1)];
        let result = eval_expr_program(&program, &[], &constants);
        let Err(ExprError::IntegerOverflow) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected IntegerOverflow for i64::MIN / -1 end-to-end".into(),
            });
        };
        Ok(())
    }
}
