#![forbid(unsafe_code)]
//! Bounded stack-based expression bytecode evaluator.

use arrayvec::ArrayVec;
use vb_core::limits::MAX_EXPRESSION_STACK_USIZE;
use vb_core::value_store::ValueStore;
use vb_core::{AccessorProgram, ConstValue, ExprOp, ExprProgram, SlotValue};

use super::helpers::eval_helper_op_with_store;
use super::ops::{eval_binary_op, eval_unary_op};
use super::stack::{pop_pair, pop_value, push_value};
use crate::lexer::{BinaryOp, UnaryOp};
use crate::{AccessorContext, AccessorContextAbsence, ExprError, ExprResult};
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
    eval_expr_program_with_context(
        program,
        slots,
        constants,
        AccessorContext::Absent(AccessorContextAbsence::LegacyApiNoAccessorTable),
        store,
    )
}

pub fn eval_expr_program_with_accessors_and_store(
    program: &ExprProgram,
    slots: &[Option<SlotValue>],
    constants: &[ConstValue],
    accessors: &[AccessorProgram],
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    eval_expr_program_with_context(
        program,
        slots,
        constants,
        AccessorContext::Present(accessors),
        store,
    )
}

pub fn eval_expr_program_with_context(
    program: &ExprProgram,
    slots: &[Option<SlotValue>],
    constants: &[ConstValue],
    accessors: AccessorContext<'_>,
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
        eval_expr_op_with_store(
            op,
            &mut stack,
            slots,
            constants,
            accessors,
            store,
            &program.constants,
        )?;
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
    accessors: AccessorContext<'_>,
    store: &mut ValueStore,
    program_constants: &[ConstValue],
) -> ExprResult<()> {
    match op {
        ExprOp::LoadSlot(idx) => eval_load_slot(stack, slots, idx),
        ExprOp::LoadConst(idx) => eval_load_const(stack, constants, program_constants, idx),
        ExprOp::LoadAccessor(idx) => {
            super::accessors::eval_load_accessor(stack, slots, accessors, store, idx)
        }
        ExprOp::Eq => eval_eq(stack),
        ExprOp::NotEq => eval_ne(stack),
        ExprOp::Not => eval_unary_stack(stack, UnaryOp::Not),
        _ => eval_non_load_op_with_store(op, stack, store),
    }
}

fn eval_non_load_op_with_store(
    op: ExprOp,
    stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    store: &mut ValueStore,
) -> ExprResult<()> {
    match op {
        ExprOp::And => eval_binary_stack(stack, BinaryOp::And),
        ExprOp::Or => eval_binary_stack(stack, BinaryOp::Or),
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
    program_constants: &[ConstValue],
    idx: vb_core::ConstIdx,
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

fn eval_eq(stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>) -> ExprResult<()> {
    let (left, right) = pop_pair(stack)?;
    push_value(stack, SlotValue::Bool(left == right))
}

fn eval_ne(stack: &mut ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>) -> ExprResult<()> {
    let (left, right) = pop_pair(stack)?;
    push_value(stack, SlotValue::Bool(left != right))
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
