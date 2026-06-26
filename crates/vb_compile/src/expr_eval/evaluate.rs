#![forbid(unsafe_code)]
//! Bounded stack-based expression bytecode evaluator.

use arrayvec::ArrayVec;
use vb_core::limits::MAX_EXPRESSION_STACK_USIZE;
use vb_core::value_store::ValueStore;
use vb_core::{ConstValue, ExprOp, ExprProgram, SlotValue};

use super::environment::{
    expect_bool, expect_i64, expect_list, expect_object, expect_symbol, pop_pair, pop_triple,
    pop_value, push_value,
};
use crate::lexer::{BinaryOp, UnaryOp};
use crate::parser::ExprHelper;
use crate::{ExprError, ExprResult};
use vb_core::limits::MAX_EXPRESSION_STACK;

pub fn eval_expr_program(
    program: &ExprProgram,
    slots: &[Option<SlotValue>],
    constants: &[ConstValue],
) -> ExprResult<SlotValue> {
    let mut store = ValueStore::new();
    eval_expr_program_with_store(program, slots, constants, &mut store)
}

pub fn eval_expr_program_with_store(
    program: &ExprProgram,
    slots: &[Option<SlotValue>],
    constants: &[ConstValue],
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let mut stack: ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE> = ArrayVec::new();
    let mut index = 0usize;
    while index < program.ops.len() {
        let op = *program
            .ops
            .as_ref()
            .get(index)
            .ok_or(ExprError::UnexpectedEof)?;
        eval_expr_op_with_store(op, &mut stack, slots, constants, store)?;
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

fn eval_expr_op_with_store(
    op: ExprOp,
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    slots: &[Option<SlotValue>],
    constants: &[ConstValue],
    store: &mut ValueStore,
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
        _ => eval_helper_op_with_store(op, stack, store),
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

pub fn eval_binary_op(op: BinaryOp, left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match op {
        BinaryOp::And => Ok(SlotValue::Bool(expect_bool(left)? && expect_bool(right)?)),
        BinaryOp::Or => Ok(SlotValue::Bool(expect_bool(left)? || expect_bool(right)?)),
        BinaryOp::Eq => Ok(SlotValue::Bool(left == right)),
        BinaryOp::NotEq => Ok(SlotValue::Bool(left != right)),
        BinaryOp::Add => eval_add_op(left, right),
        BinaryOp::Sub => eval_sub_op(left, right),
        BinaryOp::Mul => eval_mul_op(left, right),
        BinaryOp::Div => eval_div_op(left, right),
        BinaryOp::Gt => eval_gt_op(left, right),
        BinaryOp::Gte => eval_gte_op(left, right),
        BinaryOp::Lt => eval_lt_op(left, right),
        BinaryOp::Lte => eval_lte_op(left, right),
    }
}

fn eval_add_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match (left, right) {
        (SlotValue::F64(l), SlotValue::F64(r)) => {
            let result = l.get() + r.get();
            let finite = vb_core::value::FiniteF64::new(result)?;
            Ok(SlotValue::F64(finite))
        }
        (SlotValue::I64(l), SlotValue::I64(r)) => eval_i64_values_(l, r, i64::checked_add),
        (other_left, other_right) => eval_i64_values_(
            expect_i64(other_left)?,
            expect_i64(other_right)?,
            i64::checked_add,
        ),
    }
}

fn eval_sub_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match (left, right) {
        (SlotValue::F64(l), SlotValue::F64(r)) => {
            let result = l.get() - r.get();
            let finite = vb_core::value::FiniteF64::new(result)?;
            Ok(SlotValue::F64(finite))
        }
        (SlotValue::I64(l), SlotValue::I64(r)) => eval_i64_values_(l, r, i64::checked_sub),
        (other_left, other_right) => eval_i64_values_(
            expect_i64(other_left)?,
            expect_i64(other_right)?,
            i64::checked_sub,
        ),
    }
}

fn eval_mul_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match (left, right) {
        (SlotValue::F64(l), SlotValue::F64(r)) => {
            let result = l.get() * r.get();
            let finite = vb_core::value::FiniteF64::new(result)?;
            Ok(SlotValue::F64(finite))
        }
        (SlotValue::I64(l), SlotValue::I64(r)) => eval_i64_values_(l, r, i64::checked_mul),
        (other_left, other_right) => eval_i64_values_(
            expect_i64(other_left)?,
            expect_i64(other_right)?,
            i64::checked_mul,
        ),
    }
}

fn eval_div_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match (left, right) {
        (SlotValue::F64(l), SlotValue::F64(r)) => {
            let result = l.get() / r.get();
            let finite =
                vb_core::value::FiniteF64::new(result).map_err(|_| ExprError::NonFiniteFloat)?;
            Ok(SlotValue::F64(finite))
        }
        (SlotValue::I64(l), SlotValue::I64(r)) => eval_div_values_(l, r),
        (other_left, other_right) => {
            eval_div_values_(expect_i64(other_left)?, expect_i64(other_right)?)
        }
    }
}

fn eval_gt_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match (left, right) {
        (SlotValue::F64(l), SlotValue::F64(r)) => Ok(SlotValue::Bool(l.get() > r.get())),
        (SlotValue::I64(l), SlotValue::I64(r)) => eval_i64_cmp_values_(l, r, i64::gt),
        (other_left, other_right) => {
            eval_i64_cmp_values_(expect_i64(other_left)?, expect_i64(other_right)?, i64::gt)
        }
    }
}

fn eval_gte_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match (left, right) {
        (SlotValue::F64(l), SlotValue::F64(r)) => Ok(SlotValue::Bool(l.get() >= r.get())),
        (SlotValue::I64(l), SlotValue::I64(r)) => eval_i64_cmp_values_(l, r, i64::ge),
        (other_left, other_right) => {
            eval_i64_cmp_values_(expect_i64(other_left)?, expect_i64(other_right)?, i64::ge)
        }
    }
}

fn eval_lt_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match (left, right) {
        (SlotValue::F64(l), SlotValue::F64(r)) => Ok(SlotValue::Bool(l.get() < r.get())),
        (SlotValue::I64(l), SlotValue::I64(r)) => eval_i64_cmp_values_(l, r, i64::lt),
        (other_left, other_right) => {
            eval_i64_cmp_values_(expect_i64(other_left)?, expect_i64(other_right)?, i64::lt)
        }
    }
}

fn eval_lte_op(left: SlotValue, right: SlotValue) -> ExprResult<SlotValue> {
    match (left, right) {
        (SlotValue::F64(l), SlotValue::F64(r)) => Ok(SlotValue::Bool(l.get() <= r.get())),
        (SlotValue::I64(l), SlotValue::I64(r)) => eval_i64_cmp_values_(l, r, i64::le),
        (other_left, other_right) => {
            eval_i64_cmp_values_(expect_i64(other_left)?, expect_i64(other_right)?, i64::le)
        }
    }
}

fn eval_i64_values_(
    left: i64,
    right: i64,
    op: fn(i64, i64) -> Option<i64>,
) -> ExprResult<SlotValue> {
    let value = op(left, right).ok_or(ExprError::IntegerOverflow)?;
    Ok(SlotValue::I64(value))
}

fn eval_div_values_(left: i64, right: i64) -> ExprResult<SlotValue> {
    if right == 0 {
        return Err(ExprError::DivisionByZero);
    }
    let value = left.checked_div(right).ok_or(ExprError::IntegerOverflow)?;
    Ok(SlotValue::I64(value))
}

fn eval_i64_cmp_values_(
    left: i64,
    right: i64,
    op: fn(&i64, &i64) -> bool,
) -> ExprResult<SlotValue> {
    Ok(SlotValue::Bool(op(&left, &right)))
}

pub fn eval_unary_op(op: UnaryOp, value: SlotValue) -> ExprResult<SlotValue> {
    match op {
        UnaryOp::Not => Ok(SlotValue::Bool(!expect_bool(value)?)),
        UnaryOp::Neg => eval_neg_op(value),
    }
}

fn eval_neg_op(value: SlotValue) -> ExprResult<SlotValue> {
    match value {
        SlotValue::F64(f) => {
            let result = -f.get();
            let finite = vb_core::value::FiniteF64::new(result)?;
            Ok(SlotValue::F64(finite))
        }
        SlotValue::I64(n) => {
            let negated = n.checked_neg().ok_or(ExprError::IntegerOverflow)?;
            Ok(SlotValue::I64(negated))
        }
        other => Err(ExprError::TypeMismatch {
            expected: "number".into(),
            found: other.type_name().into(),
        }),
    }
}

fn eval_helper_op_with_store(
    op: ExprOp,
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    store: &mut ValueStore,
) -> ExprResult<()> {
    match op {
        ExprOp::Exists => {
            let value = pop_value(stack)?;
            let result = eval_helper_exists_with_store(&value, store)?;
            push_value(stack, result)
        }
        ExprOp::Length => {
            let value = pop_value(stack)?;
            let result = eval_helper_length_with_store(&value, store)?;
            push_value(stack, result)
        }
        ExprOp::Empty => {
            let value = pop_value(stack)?;
            let result = eval_helper_empty_with_store(&value, store)?;
            push_value(stack, result)
        }
        ExprOp::Count => {
            let value = pop_value(stack)?;
            let result = eval_helper_count_with_store(&value, store)?;
            push_value(stack, result)
        }
        ExprOp::Unique => {
            let value = pop_value(stack)?;
            let result = eval_helper_unique_with_store(&value, store)?;
            push_value(stack, result)
        }
        ExprOp::Contains => {
            let (right, left) = pop_pair(stack)?;
            let result = eval_helper_contains_with_store(&left, &right, store)?;
            push_value(stack, result)
        }
        ExprOp::StartsWith => {
            let (right, left) = pop_pair(stack)?;
            let result = eval_helper_starts_with_with_store(&left, &right, store)?;
            push_value(stack, result)
        }
        ExprOp::EndsWith => {
            let (right, left) = pop_pair(stack)?;
            let result = eval_helper_ends_with_with_store(&left, &right, store)?;
            push_value(stack, result)
        }
        ExprOp::Has => {
            let (right, left) = pop_pair(stack)?;
            let result = eval_helper_has_with_store(&left, &right, store)?;
            push_value(stack, result)
        }
        ExprOp::Append => {
            let (right, left) = pop_pair(stack)?;
            let result = eval_helper_append_with_store(&left, &right, store)?;
            push_value(stack, result)
        }
        ExprOp::AppendIf => {
            let (third, second, first) = pop_triple(stack)?;
            let result = eval_helper_append_if_with_store(&first, &second, &third, store)?;
            push_value(stack, result)
        }
        ExprOp::Merge => {
            let (right, left) = pop_pair(stack)?;
            let result = eval_helper_merge_with_store(&left, &right, store)?;
            push_value(stack, result)
        }
        ExprOp::Sum => {
            let value = pop_value(stack)?;
            let result = eval_helper_sum_with_store(&value, store)?;
            push_value(stack, result)
        }
        _ => Err(ExprError::UnknownOperator {
            op: format!("{op:?}"),
        }),
    }
}

pub fn eval_helper_with_store(
    helper: ExprHelper,
    args: &[SlotValue],
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    match helper {
        ExprHelper::Exists => {
            let value = one_arg(args, helper)?;
            eval_helper_exists_with_store(value, store)
        }
        ExprHelper::Length => {
            let value = one_arg(args, helper)?;
            eval_helper_length_with_store(value, store)
        }
        ExprHelper::Empty => {
            let value = one_arg(args, helper)?;
            eval_helper_empty_with_store(value, store)
        }
        ExprHelper::Count => {
            let value = one_arg(args, helper)?;
            eval_helper_count_with_store(value, store)
        }
        ExprHelper::Unique => {
            let value = one_arg(args, helper)?;
            eval_helper_unique_with_store(value, store)
        }
        ExprHelper::Contains => {
            let (left, right) = two_args(args, helper)?;
            eval_helper_contains_with_store(left, right, store)
        }
        ExprHelper::StartsWith => {
            let (left, right) = two_args(args, helper)?;
            eval_helper_starts_with_with_store(left, right, store)
        }
        ExprHelper::EndsWith => {
            let (left, right) = two_args(args, helper)?;
            eval_helper_ends_with_with_store(left, right, store)
        }
        ExprHelper::Has => {
            let (left, right) = two_args(args, helper)?;
            eval_helper_has_with_store(left, right, store)
        }
        ExprHelper::Append => {
            let (left, right) = two_args(args, helper)?;
            eval_helper_append_with_store(left, right, store)
        }
        ExprHelper::AppendIf => {
            let (first, second, third) = three_args(args, helper)?;
            eval_helper_append_if_with_store(first, second, third, store)
        }
        ExprHelper::Merge => {
            let (left, right) = two_args(args, helper)?;
            eval_helper_merge_with_store(left, right, store)
        }
        ExprHelper::Sum => {
            let value = one_arg(args, helper)?;
            eval_helper_sum_with_store(value, store)
        }
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

fn eval_helper_exists_with_store(
    value: &SlotValue,
    _store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    Ok(SlotValue::Bool(!matches!(value, SlotValue::Null)))
}

fn eval_helper_length_with_store(
    value: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let len = match *value {
        SlotValue::Symbol(id) => {
            let s = store.symbol(id).map_err(|_| ExprError::InvalidReference {
                reference: format!("symbol:{id:?}"),
            })?;
            s.len()
        }
        SlotValue::List(id) => {
            let items = store.list(id).map_err(|_| ExprError::InvalidReference {
                reference: format!("list:{id:?}"),
            })?;
            items.len()
        }
        SlotValue::Object(id) => {
            let fields = store.object(id).map_err(|_| ExprError::InvalidReference {
                reference: format!("object:{id:?}"),
            })?;
            fields.len()
        }
        ref other => {
            return Err(ExprError::TypeMismatch {
                expected: "text, list, or object".into(),
                found: other.type_name().into(),
            });
        }
    };
    let len_i64 = i64::try_from(len).map_err(|_| ExprError::IntegerOverflow)?;
    Ok(SlotValue::I64(len_i64))
}

fn eval_helper_empty_with_store(
    value: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let is_empty = match *value {
        SlotValue::Null => true,
        SlotValue::Symbol(id) => {
            let s = store.symbol(id).map_err(|_| ExprError::InvalidReference {
                reference: format!("symbol:{id:?}"),
            })?;
            s.is_empty()
        }
        SlotValue::List(id) => {
            let items = store.list(id).map_err(|_| ExprError::InvalidReference {
                reference: format!("list:{id:?}"),
            })?;
            items.is_empty()
        }
        SlotValue::Object(id) => {
            let fields = store.object(id).map_err(|_| ExprError::InvalidReference {
                reference: format!("object:{id:?}"),
            })?;
            fields.is_empty()
        }
        ref other => {
            return Err(ExprError::TypeMismatch {
                expected: "text, list, object, or null".into(),
                found: other.type_name().into(),
            });
        }
    };
    Ok(SlotValue::Bool(is_empty))
}

fn eval_helper_count_with_store(
    value: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let list_id = expect_list(*value)?;
    let items = store
        .list(list_id)
        .map_err(|_| ExprError::InvalidReference {
            reference: format!("list:{list_id:?}"),
        })?;
    let count = i64::try_from(items.len()).map_err(|_| ExprError::IntegerOverflow)?;
    Ok(SlotValue::I64(count))
}

fn eval_helper_unique_with_store(
    value: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let list_id = expect_list(*value)?;
    let items = store
        .list(list_id)
        .map_err(|_| ExprError::InvalidReference {
            reference: format!("list:{list_id:?}"),
        })?;
    let mut seen: Vec<SlotValue> = Vec::new();
    for &item in items {
        if !seen.contains(&item) {
            seen.push(item);
        }
    }
    let new_list = store
        .insert_list(seen.into_boxed_slice())
        .map_err(|_| ExprError::IntegerOverflow)?;
    Ok(SlotValue::List(new_list))
}

fn eval_helper_contains_with_store(
    haystack: &SlotValue,
    needle: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    if matches!(*haystack, SlotValue::F64(_)) || matches!(*needle, SlotValue::F64(_)) {
        return Err(ExprError::TypeMismatch {
            expected: "list, text, or object".into(),
            found: "number".into(),
        });
    }
    let haystack_id = expect_symbol(*haystack)?;
    let needle_id = expect_symbol(*needle)?;
    let haystack_str = store
        .symbol(haystack_id)
        .map_err(|_| ExprError::InvalidReference {
            reference: format!("symbol:{haystack_id:?}"),
        })?;
    let needle_str = store
        .symbol(needle_id)
        .map_err(|_| ExprError::InvalidReference {
            reference: format!("symbol:{needle_id:?}"),
        })?;
    Ok(SlotValue::Bool(haystack_str.contains(needle_str)))
}

fn eval_helper_starts_with_with_store(
    text: &SlotValue,
    prefix: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let text_id = expect_symbol(*text)?;
    let prefix_id = expect_symbol(*prefix)?;
    let text_str = store
        .symbol(text_id)
        .map_err(|_| ExprError::InvalidReference {
            reference: format!("symbol:{text_id:?}"),
        })?;
    let prefix_str = store
        .symbol(prefix_id)
        .map_err(|_| ExprError::InvalidReference {
            reference: format!("symbol:{prefix_id:?}"),
        })?;
    Ok(SlotValue::Bool(text_str.starts_with(prefix_str)))
}

fn eval_helper_ends_with_with_store(
    text: &SlotValue,
    suffix: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let text_id = expect_symbol(*text)?;
    let suffix_id = expect_symbol(*suffix)?;
    let text_str = store
        .symbol(text_id)
        .map_err(|_| ExprError::InvalidReference {
            reference: format!("symbol:{text_id:?}"),
        })?;
    let suffix_str = store
        .symbol(suffix_id)
        .map_err(|_| ExprError::InvalidReference {
            reference: format!("symbol:{suffix_id:?}"),
        })?;
    Ok(SlotValue::Bool(text_str.ends_with(suffix_str)))
}

fn eval_helper_has_with_store(
    obj: &SlotValue,
    key: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let obj_id = expect_object(*obj)?;
    let key_id = expect_symbol(*key)?;
    let fields = store
        .object(obj_id)
        .map_err(|_| ExprError::InvalidReference {
            reference: format!("object:{obj_id:?}"),
        })?;
    let found = fields.iter().any(|f| f.key == key_id);
    Ok(SlotValue::Bool(found))
}

fn eval_helper_append_with_store(
    list: &SlotValue,
    item: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let list_id = expect_list(*list)?;
    let items = store
        .list(list_id)
        .map_err(|_| ExprError::InvalidReference {
            reference: format!("list:{list_id:?}"),
        })?;
    let mut new_items: Vec<SlotValue> = items.to_vec();
    new_items.push(*item);
    let new_list = store
        .insert_list(new_items.into_boxed_slice())
        .map_err(|_| ExprError::IntegerOverflow)?;
    Ok(SlotValue::List(new_list))
}

fn eval_helper_append_if_with_store(
    list: &SlotValue,
    item: &SlotValue,
    condition: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let list_id = expect_list(*list)?;
    let cond = expect_bool(*condition)?;
    let items = store
        .list(list_id)
        .map_err(|_| ExprError::InvalidReference {
            reference: format!("list:{list_id:?}"),
        })?;
    let mut new_items: Vec<SlotValue> = items.to_vec();
    if cond {
        new_items.push(*item);
    }
    let new_list = store
        .insert_list(new_items.into_boxed_slice())
        .map_err(|_| ExprError::IntegerOverflow)?;
    Ok(SlotValue::List(new_list))
}

fn eval_helper_merge_with_store(
    left: &SlotValue,
    right: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let left_id = expect_object(*left)?;
    let right_id = expect_object(*right)?;
    let left_fields = store
        .object(left_id)
        .map_err(|_| ExprError::InvalidReference {
            reference: format!("object:{left_id:?}"),
        })?;
    let right_fields = store
        .object(right_id)
        .map_err(|_| ExprError::InvalidReference {
            reference: format!("object:{right_id:?}"),
        })?;
    let mut merged: Vec<vb_core::value_store::ObjectField> = left_fields.to_vec();
    for &field in right_fields {
        if let Some(pos) = merged.iter().position(|f| f.key == field.key) {
            if let Some(entry) = merged.get_mut(pos) {
                *entry = field;
            }
        } else {
            merged.push(field);
        }
    }
    let new_object = store
        .insert_object(merged.into_boxed_slice())
        .map_err(|_| ExprError::IntegerOverflow)?;
    Ok(SlotValue::Object(new_object))
}

fn eval_helper_sum_with_store(value: &SlotValue, store: &mut ValueStore) -> ExprResult<SlotValue> {
    let list_id = expect_list(*value)?;
    let items = store
        .list(list_id)
        .map_err(|_| ExprError::InvalidReference {
            reference: format!("list:{list_id:?}"),
        })?;
    let mut sum: i64 = 0;
    for &item in items {
        let n = expect_i64(item)?;
        sum = sum.checked_add(n).ok_or(ExprError::IntegerOverflow)?;
    }
    Ok(SlotValue::I64(sum))
}
