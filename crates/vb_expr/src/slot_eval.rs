#![forbid(unsafe_code)]
//! Slot and constant loading operations.

use arrayvec::ArrayVec;
use vb_core::limits::MAX_EXPRESSION_STACK_USIZE;
use vb_core::{ConstIdx, ConstValue, SlotIdx, SlotValue};

use crate::stack_ops::push_value;
use crate::{ExprError, ExprResult};

/// Loads a slot value onto the evaluation stack.
pub fn eval_load_slot(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    slots: &[Option<SlotValue>],
    idx: SlotIdx,
) -> ExprResult<()> {
    let value = slots
        .get(idx.as_usize())
        .and_then(|opt| *opt)
        .ok_or(ExprError::StackUnderflow)?;
    push_value(stack, value)
}

/// Loads a constant value onto the evaluation stack.
///
/// Resolves `idx` against the caller-supplied `constants` slice first,
/// then falls back to the program-attached `program_constants` pool so
/// `ExprProgram`s produced by `compile_expr_to_bytecode` evaluate
/// without requiring an external constants vector.
pub fn eval_load_const(
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    constants: &[ConstValue],
    program_constants: &[ConstValue],
    idx: ConstIdx,
) -> ExprResult<()> {
    let constant = constants
        .get(idx.as_usize())
        .or_else(|| program_constants.get(idx.as_usize()))
        .ok_or(ExprError::UnexpectedEof)?;
    let value = constant
        .to_slot_value()
        .map_err(|_| ExprError::UnexpectedEof)?;
    push_value(stack, value)
}
