//! Helper function evaluation (Exists, Length, Empty, Count, Unique).

use arrayvec::ArrayVec;
use vb_core::limits::MAX_EXPRESSION_STACK_USIZE;
use vb_core::SlotValue;

use crate::parser::{helper_name, ExprHelper};
use crate::stack_ops::{pop_value, push_value};
use crate::{ExprError, ExprResult};

/// Dispatches ExprOp variants that correspond to helpers.
pub fn eval_helper_op(
    op: crate::ExprOp,
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
) -> ExprResult<()> {
    match op {
        crate::ExprOp::Exists => eval_helper_stack(stack, ExprHelper::Exists),
        crate::ExprOp::Length => eval_helper_stack(stack, ExprHelper::Length),
        crate::ExprOp::Empty => eval_helper_stack(stack, ExprHelper::Empty),
        crate::ExprOp::Count => eval_helper_stack(stack, ExprHelper::Count),
        crate::ExprOp::Unique => eval_helper_stack(stack, ExprHelper::Unique),
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
            helper: helper_name(helper).into(),
        }),
    }
}

fn one_arg(args: &[SlotValue], helper: ExprHelper) -> ExprResult<&SlotValue> {
    if args.len() != 1 {
        return Err(ExprError::HelperArityMismatch {
            helper: helper_name(helper).into(),
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
