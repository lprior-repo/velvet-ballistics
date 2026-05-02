//! Expression evaluation engine.

use crate::errors::EngineError;
use crate::ids::{AccessorIdx, ConstIdx, ExprIdx, SlotIdx};
use crate::limits::MAX_EXPRESSION_STACK_USIZE;
use crate::value::SlotValue;
use crate::value_store::ValueStore;
use crate::workflow::{AccessorProgram, CompiledWorkflow, ExprOp};

struct ExprStack {
    values: [SlotValue; MAX_EXPRESSION_STACK_USIZE],
    len: u8,
    capacity: u8,
}

impl ExprStack {
    fn new(capacity: u8) -> Result<Self, EngineError> {
        if usize::from(capacity) <= MAX_EXPRESSION_STACK_USIZE {
            Ok(Self {
                values: [SlotValue::Null; MAX_EXPRESSION_STACK_USIZE],
                len: 0,
                capacity,
            })
        } else {
            Err(EngineError::ExpressionStackOverflow { max: capacity })
        }
    }

    const fn len(&self) -> u8 {
        self.len
    }

    fn push(&mut self, value: SlotValue) -> Result<(), EngineError> {
        if self.len >= self.capacity {
            return Err(EngineError::ExpressionStackOverflow { max: self.capacity });
        }
        let index = usize::from(self.len);
        *self
            .values
            .get_mut(index)
            .ok_or(EngineError::ExpressionStackOverflow { max: self.capacity })? = value;
        self.len = self
            .len
            .checked_add(1)
            .ok_or(EngineError::ExpressionStackOverflow { max: self.capacity })?;
        Ok(())
    }

    fn pop(&mut self) -> Result<SlotValue, EngineError> {
        if self.len == 0 {
            return Err(EngineError::ExpressionStackUnderflow);
        }
        self.len = self
            .len
            .checked_sub(1)
            .ok_or(EngineError::ExpressionStackUnderflow)?;
        self.values.get(usize::from(self.len)).copied().ok_or(
            EngineError::InternalInvariantViolation {
                reason: "expression stack pop index checked by length",
            },
        )
    }
}

fn push_value(stack: &mut ExprStack, value: SlotValue) -> Result<(), EngineError> {
    stack.push(value)
}

fn pop_value(stack: &mut ExprStack) -> Result<SlotValue, EngineError> {
    stack.pop()
}

fn pop_pair(stack: &mut ExprStack) -> Result<(SlotValue, SlotValue), EngineError> {
    let right = pop_value(stack)?;
    let left = pop_value(stack)?;
    Ok((left, right))
}

fn pop_i64_pair(stack: &mut ExprStack) -> Result<(i64, i64), EngineError> {
    let (left, right) = pop_pair(stack)?;
    Ok((expect_i64(left)?, expect_i64(right)?))
}

fn expect_bool(value: SlotValue) -> Result<bool, EngineError> {
    match value {
        SlotValue::Bool(value) => Ok(value),
        other => Err(EngineError::TypeMismatch {
            expected: "boolean",
            found: other.type_name(),
        }),
    }
}

fn expect_i64(value: SlotValue) -> Result<i64, EngineError> {
    match value {
        SlotValue::I64(value) => Ok(value),
        other => Err(EngineError::TypeMismatch {
            expected: "number",
            found: other.type_name(),
        }),
    }
}

fn eval_eq(stack: &mut ExprStack, positive: bool) -> Result<(), EngineError> {
    let (left, right) = pop_pair(stack)?;
    push_value(stack, SlotValue::Bool((left == right) == positive))
}

fn eval_not(stack: &mut ExprStack) -> Result<(), EngineError> {
    let value = expect_bool(pop_value(stack)?)?;
    push_value(stack, SlotValue::Bool(!value))
}

fn eval_bool_pair(stack: &mut ExprStack, op: fn(bool, bool) -> bool) -> Result<(), EngineError> {
    let (left, right) = pop_pair(stack)?;
    push_value(
        stack,
        SlotValue::Bool(op(expect_bool(left)?, expect_bool(right)?)),
    )
}

fn eval_i64_pair(
    stack: &mut ExprStack,
    op: fn(i64, i64) -> Option<i64>,
) -> Result<(), EngineError> {
    let (left, right) = pop_i64_pair(stack)?;
    let value = op(left, right).ok_or(EngineError::InvalidCompiledWorkflow {
        reason: "integer arithmetic overflow",
    })?;
    push_value(stack, SlotValue::I64(value))
}

fn eval_div(stack: &mut ExprStack) -> Result<(), EngineError> {
    let (left, right) = pop_i64_pair(stack)?;
    let value = left.checked_div(right).ok_or(EngineError::DivisionByZero)?;
    push_value(stack, SlotValue::I64(value))
}

fn eval_i64_cmp(stack: &mut ExprStack, op: fn(&i64, &i64) -> bool) -> Result<(), EngineError> {
    let (left, right) = pop_i64_pair(stack)?;
    push_value(stack, SlotValue::Bool(op(&left, &right)))
}

fn eval_expr_operator(op: ExprOp, stack: &mut ExprStack) -> Result<(), EngineError> {
    match op {
        ExprOp::Eq => eval_eq(stack, true),
        ExprOp::NotEq => eval_eq(stack, false),
        ExprOp::And => eval_bool_pair(stack, |left, right| left && right),
        ExprOp::Or => eval_bool_pair(stack, |left, right| left || right),
        ExprOp::Not => eval_not(stack),
        ExprOp::Add => eval_i64_pair(stack, i64::checked_add),
        ExprOp::Sub => eval_i64_pair(stack, i64::checked_sub),
        ExprOp::Mul => eval_i64_pair(stack, i64::checked_mul),
        ExprOp::Div => eval_div(stack),
        ExprOp::Gt => eval_i64_cmp(stack, i64::gt),
        ExprOp::Gte => eval_i64_cmp(stack, i64::ge),
        ExprOp::Lt => eval_i64_cmp(stack, i64::lt),
        ExprOp::Lte => eval_i64_cmp(stack, i64::le),
        _ => Err(EngineError::InvalidCompiledWorkflow {
            reason: "unsupported expression op",
        }),
    }
}

fn expression_op(ops: &[ExprOp], index: usize) -> Result<ExprOp, EngineError> {
    ops.get(index)
        .copied()
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "expression op index checked by loop bound",
        })
}

fn next_expr_index(index: usize) -> Result<usize, EngineError> {
    index
        .checked_add(1)
        .ok_or(EngineError::InternalInvariantViolation {
            reason: "expression op index overflow",
        })
}

fn finish_expr_stack(stack: &mut ExprStack) -> Result<SlotValue, EngineError> {
    if stack.len() == 1 {
        stack.pop()
    } else {
        Err(EngineError::InvalidCompiledWorkflow {
            reason: "expression leaves non-single result",
        })
    }
}

fn eval_expr_inner(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: Option<&ValueStore>,
    expr: ExprIdx,
) -> Result<SlotValue, EngineError> {
    let program = plan
        .expression(expr)
        .ok_or(EngineError::ExprOutOfBounds { expr })?;
    let mut stack = ExprStack::new(program.max_stack)?;
    let mut index = 0usize;
    while index < program.ops.len() {
        let op = expression_op(program.ops.as_ref(), index)?;
        eval_expr_op(plan, run, store, op, &mut stack)?;
        index = next_expr_index(index)?;
    }
    finish_expr_stack(&mut stack)
}

fn eval_expr_op(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: Option<&ValueStore>,
    op: ExprOp,
    stack: &mut ExprStack,
) -> Result<(), EngineError> {
    match op {
        ExprOp::LoadSlot(slot) => eval_load_slot(run, stack, slot),
        ExprOp::LoadConst(constant) => eval_load_const(plan, stack, constant),
        ExprOp::LoadAccessor(accessor) => eval_load_accessor(plan, run, store, stack, accessor),
        other => eval_expr_operator(other, stack),
    }
}

fn eval_load_slot(
    run: &crate::RunFrame,
    stack: &mut ExprStack,
    slot: SlotIdx,
) -> Result<(), EngineError> {
    push_value(stack, *run.read_slot(slot)?)
}

fn eval_load_const(
    plan: &CompiledWorkflow,
    stack: &mut ExprStack,
    constant: ConstIdx,
) -> Result<(), EngineError> {
    push_value(
        stack,
        plan.constant(constant)
            .ok_or(EngineError::ConstOutOfBounds { index: constant })?
            .to_slot_value()?,
    )
}

fn eval_load_accessor(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: Option<&ValueStore>,
    stack: &mut ExprStack,
    accessor: AccessorIdx,
) -> Result<(), EngineError> {
    push_value(stack, eval_accessor_inner(plan, run, store, accessor)?)
}

pub fn eval_expr_with_store(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: &ValueStore,
    expr: ExprIdx,
) -> Result<SlotValue, EngineError> {
    eval_expr_inner(plan, run, Some(store), expr)
}

pub fn eval_expr(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    expr: ExprIdx,
) -> Result<SlotValue, EngineError> {
    eval_expr_inner(plan, run, None, expr)
}

fn eval_accessor_inner(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: Option<&ValueStore>,
    accessor: AccessorIdx,
) -> Result<SlotValue, EngineError> {
    let program = plan
        .accessor(accessor)
        .ok_or(EngineError::InvalidCompiledWorkflow {
            reason: "accessor index out of bounds",
        })?;
    eval_accessor_program(run, store, program)
}

pub fn eval_accessor(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    accessor: AccessorIdx,
) -> Result<SlotValue, EngineError> {
    eval_accessor_inner(plan, run, None, accessor)
}

pub fn eval_accessor_with_store(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: &ValueStore,
    accessor: AccessorIdx,
) -> Result<SlotValue, EngineError> {
    eval_accessor_inner(plan, run, Some(store), accessor)
}

fn eval_accessor_program(
    run: &crate::RunFrame,
    store: Option<&ValueStore>,
    program: &AccessorProgram,
) -> Result<SlotValue, EngineError> {
    let mut current = *run.read_slot(program.root)?;
    if program.path.is_empty() {
        return Ok(current);
    }

    let store = match store {
        Some(store) => store,
        None => {
            let segment = program.path.first().copied().ok_or({
                EngineError::InternalInvariantViolation {
                    reason: "accessor path checked non-empty",
                }
            })?;
            return Err(EngineError::UnsupportedAccessorTraversal {
                segment: path_segment_name(segment),
                found: current.type_name(),
            });
        }
    };
    let mut index = 0usize;
    while index < program.path.len() {
        let segment = program.path.get(index).copied().ok_or({
            EngineError::InternalInvariantViolation {
                reason: "accessor path index checked by loop bound",
            }
        })?;
        current = traverse_accessor_segment(store, current, segment)?;
        index = index
            .checked_add(1)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "accessor path index overflow",
            })?;
    }
    Ok(current)
}

fn traverse_accessor_segment(
    store: &ValueStore,
    current: SlotValue,
    segment: crate::workflow::PathSegment,
) -> Result<SlotValue, EngineError> {
    match (current, segment) {
        (SlotValue::Object(object), crate::workflow::PathSegment::Field(field)) => {
            store.object_field(object, field)
        }
        (SlotValue::List(list), crate::workflow::PathSegment::Index(index)) => {
            store.list_item(list, index)
        }
        (value, segment) => Err(EngineError::UnsupportedAccessorTraversal {
            segment: path_segment_name(segment),
            found: value.type_name(),
        }),
    }
}

const fn path_segment_name(segment: crate::workflow::PathSegment) -> &'static str {
    match segment {
        crate::workflow::PathSegment::Field(_) => "field",
        crate::workflow::PathSegment::Index(_) => "index",
    }
}
