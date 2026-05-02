//! Expression evaluation engine.

use crate::errors::EngineError;
use crate::ids::{AccessorIdx, ConstIdx, ExprIdx, ListId, ObjectId, SlotIdx, SymbolId};
use crate::limits::MAX_EXPRESSION_STACK_USIZE;
use crate::value::SlotValue;
use crate::value_store::{ObjectField, ValueStore};
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

fn pop_triple(stack: &mut ExprStack) -> Result<(SlotValue, SlotValue, SlotValue), EngineError> {
    let right = pop_value(stack)?;
    let mid = pop_value(stack)?;
    let left = pop_value(stack)?;
    Ok((left, mid, right))
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

fn expect_symbol(value: SlotValue) -> Result<SymbolId, EngineError> {
    match value {
        SlotValue::Symbol(id) => Ok(id),
        other => Err(EngineError::TypeMismatch {
            expected: "text",
            found: other.type_name(),
        }),
    }
}

fn expect_list(value: SlotValue) -> Result<ListId, EngineError> {
    match value {
        SlotValue::List(id) => Ok(id),
        other => Err(EngineError::TypeMismatch {
            expected: "list",
            found: other.type_name(),
        }),
    }
}

fn expect_object(value: SlotValue) -> Result<ObjectId, EngineError> {
    match value {
        SlotValue::Object(id) => Ok(id),
        other => Err(EngineError::TypeMismatch {
            expected: "object",
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

// ===== Text operations =====

fn eval_contains(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
    let (haystack, needle) = pop_pair(stack)?;
    let haystack_id = expect_symbol(haystack)?;
    let needle_id = expect_symbol(needle)?;
    let haystack_str = store
        .symbol(haystack_id)
        .map_err(|_| EngineError::SymbolOutOfBounds {
            symbol: haystack_id,
        })?;
    let needle_str = store
        .symbol(needle_id)
        .map_err(|_| EngineError::SymbolOutOfBounds { symbol: needle_id })?;
    push_value(stack, SlotValue::Bool(haystack_str.contains(needle_str)))
}

fn eval_starts_with(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
    let (text, prefix) = pop_pair(stack)?;
    let text_id = expect_symbol(text)?;
    let prefix_id = expect_symbol(prefix)?;
    let text_str = store
        .symbol(text_id)
        .map_err(|_| EngineError::SymbolOutOfBounds { symbol: text_id })?;
    let prefix_str = store
        .symbol(prefix_id)
        .map_err(|_| EngineError::SymbolOutOfBounds { symbol: prefix_id })?;
    push_value(stack, SlotValue::Bool(text_str.starts_with(prefix_str)))
}

fn eval_ends_with(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
    let (text, suffix) = pop_pair(stack)?;
    let text_id = expect_symbol(text)?;
    let suffix_id = expect_symbol(suffix)?;
    let text_str = store
        .symbol(text_id)
        .map_err(|_| EngineError::SymbolOutOfBounds { symbol: text_id })?;
    let suffix_str = store
        .symbol(suffix_id)
        .map_err(|_| EngineError::SymbolOutOfBounds { symbol: suffix_id })?;
    push_value(stack, SlotValue::Bool(text_str.ends_with(suffix_str)))
}

// ===== List operations =====

fn eval_has(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
    let (list, item) = pop_pair(stack)?;
    let list_id = expect_list(list)?;
    let items = store
        .list(list_id)
        .map_err(|_| EngineError::ListOutOfBounds { list: list_id })?;
    let found = items.contains(&item);
    push_value(stack, SlotValue::Bool(found))
}

fn eval_length(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
    let value = pop_value(stack)?;
    let len = match value {
        SlotValue::Symbol(id) => {
            let s = store
                .symbol(id)
                .map_err(|_| EngineError::SymbolOutOfBounds { symbol: id })?;
            s.len()
        }
        SlotValue::List(id) => {
            let items = store
                .list(id)
                .map_err(|_| EngineError::ListOutOfBounds { list: id })?;
            items.len()
        }
        SlotValue::Object(id) => {
            let fields = store
                .object(id)
                .map_err(|_| EngineError::ObjectOutOfBounds { object: id })?;
            fields.len()
        }
        other => {
            return Err(EngineError::TypeMismatch {
                expected: "text, list, or object",
                found: other.type_name(),
            });
        }
    };
    let len_i64 = i64::try_from(len).map_err(|_| EngineError::InvalidCompiledWorkflow {
        reason: "length exceeds i64 range",
    })?;
    push_value(stack, SlotValue::I64(len_i64))
}

fn eval_empty(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
    let value = pop_value(stack)?;
    let is_empty = match value {
        SlotValue::Null => true,
        SlotValue::Symbol(id) => {
            let s = store
                .symbol(id)
                .map_err(|_| EngineError::SymbolOutOfBounds { symbol: id })?;
            s.is_empty()
        }
        SlotValue::List(id) => {
            let items = store
                .list(id)
                .map_err(|_| EngineError::ListOutOfBounds { list: id })?;
            items.is_empty()
        }
        SlotValue::Object(id) => {
            let fields = store
                .object(id)
                .map_err(|_| EngineError::ObjectOutOfBounds { object: id })?;
            fields.is_empty()
        }
        other => {
            return Err(EngineError::TypeMismatch {
                expected: "text, list, object, or null",
                found: other.type_name(),
            });
        }
    };
    push_value(stack, SlotValue::Bool(is_empty))
}

fn eval_sum(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
    let value = pop_value(stack)?;
    let list_id = expect_list(value)?;
    let items = store
        .list(list_id)
        .map_err(|_| EngineError::ListOutOfBounds { list: list_id })?;
    let mut sum: i64 = 0;
    for &item in items.iter() {
        let n = expect_i64(item)?;
        sum = sum
            .checked_add(n)
            .ok_or(EngineError::InvalidCompiledWorkflow {
                reason: "sum overflow",
            })?;
    }
    push_value(stack, SlotValue::I64(sum))
}

fn eval_count(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
    let value = pop_value(stack)?;
    let list_id = expect_list(value)?;
    let items = store
        .list(list_id)
        .map_err(|_| EngineError::ListOutOfBounds { list: list_id })?;
    let count = i64::try_from(items.len()).map_err(|_| EngineError::InvalidCompiledWorkflow {
        reason: "count exceeds i64 range",
    })?;
    push_value(stack, SlotValue::I64(count))
}

fn eval_append(stack: &mut ExprStack, store: &mut ValueStore) -> Result<(), EngineError> {
    let (list, item) = pop_pair(stack)?;
    let list_id = expect_list(list)?;
    let items = store
        .list(list_id)
        .map_err(|_| EngineError::ListOutOfBounds { list: list_id })?;
    let mut new_items: Vec<SlotValue> = items.to_vec();
    new_items.push(item);
    let new_list = store
        .insert_list(new_items.into_boxed_slice())
        .map_err(|_| EngineError::AllocationFailed)?;
    push_value(stack, SlotValue::List(new_list))
}

fn eval_append_if(stack: &mut ExprStack, store: &mut ValueStore) -> Result<(), EngineError> {
    let (list, item, condition) = pop_triple(stack)?;
    let list_id = expect_list(list)?;
    let cond = expect_bool(condition)?;
    let items = store
        .list(list_id)
        .map_err(|_| EngineError::ListOutOfBounds { list: list_id })?;
    let mut new_items: Vec<SlotValue> = items.to_vec();
    if cond {
        new_items.push(item);
    }
    let new_list = store
        .insert_list(new_items.into_boxed_slice())
        .map_err(|_| EngineError::AllocationFailed)?;
    push_value(stack, SlotValue::List(new_list))
}

fn eval_unique(stack: &mut ExprStack, store: &mut ValueStore) -> Result<(), EngineError> {
    let value = pop_value(stack)?;
    let list_id = expect_list(value)?;
    let items = store
        .list(list_id)
        .map_err(|_| EngineError::ListOutOfBounds { list: list_id })?;
    let mut seen: Vec<SlotValue> = Vec::new();
    for &item in items.iter() {
        if !seen.contains(&item) {
            seen.push(item);
        }
    }
    let new_list = store
        .insert_list(seen.into_boxed_slice())
        .map_err(|_| EngineError::AllocationFailed)?;
    push_value(stack, SlotValue::List(new_list))
}

// ===== Object operations =====

fn eval_exists(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
    let value = pop_value(stack)?;
    match value {
        SlotValue::Null => push_value(stack, SlotValue::Bool(false)),
        SlotValue::Object(object_id) => {
            let fields = store
                .object(object_id)
                .map_err(|_| EngineError::ObjectOutOfBounds { object: object_id })?;
            push_value(stack, SlotValue::Bool(!fields.is_empty()))
        }
        other => Err(EngineError::TypeMismatch {
            expected: "object or null",
            found: other.type_name(),
        }),
    }
}

fn eval_merge(stack: &mut ExprStack, store: &mut ValueStore) -> Result<(), EngineError> {
    let (left, right) = pop_pair(stack)?;
    let left_id = expect_object(left)?;
    let right_id = expect_object(right)?;
    let left_fields = store
        .object(left_id)
        .map_err(|_| EngineError::ObjectOutOfBounds { object: left_id })?;
    let right_fields = store
        .object(right_id)
        .map_err(|_| EngineError::ObjectOutOfBounds { object: right_id })?;
    let mut merged: Vec<ObjectField> = left_fields.to_vec();
    for &field in right_fields.iter() {
        if let Some(pos) = merged.iter().position(|&f| f.key == field.key) {
            if let Some(entry) = merged.get_mut(pos) {
                *entry = field;
            }
        } else {
            merged.push(field);
        }
    }
    let new_object = store
        .insert_object(merged.into_boxed_slice())
        .map_err(|_| EngineError::AllocationFailed)?;
    push_value(stack, SlotValue::Object(new_object))
}

fn eval_expr_operator(
    op: ExprOp,
    stack: &mut ExprStack,
    store: &mut ValueStore,
) -> Result<(), EngineError> {
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
        ExprOp::Contains => eval_contains(stack, store),
        ExprOp::StartsWith => eval_starts_with(stack, store),
        ExprOp::EndsWith => eval_ends_with(stack, store),
        ExprOp::Has => eval_has(stack, store),
        ExprOp::Exists => eval_exists(stack, store),
        ExprOp::Length => eval_length(stack, store),
        ExprOp::Empty => eval_empty(stack, store),
        ExprOp::Append => eval_append(stack, store),
        ExprOp::AppendIf => eval_append_if(stack, store),
        ExprOp::Merge => eval_merge(stack, store),
        ExprOp::Sum => eval_sum(stack, store),
        ExprOp::Count => eval_count(stack, store),
        ExprOp::Unique => eval_unique(stack, store),
        ExprOp::LoadSlot(_) | ExprOp::LoadConst(_) | ExprOp::LoadAccessor(_) => {
            Err(EngineError::InternalInvariantViolation {
                reason: "load ops should be handled in eval_expr_op",
            })
        }
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
    store: &mut ValueStore,
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

fn eval_expr_op(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: &mut ValueStore,
    op: ExprOp,
    stack: &mut ExprStack,
) -> Result<(), EngineError> {
    match op {
        ExprOp::LoadSlot(slot) => eval_load_slot(run, stack, slot),
        ExprOp::LoadConst(constant) => eval_load_const(plan, stack, constant),
        ExprOp::LoadAccessor(accessor) => eval_load_accessor(plan, run, store, stack, accessor),
        other => eval_expr_operator(other, stack, store),
    }
}

fn eval_load_accessor(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: &mut ValueStore,
    stack: &mut ExprStack,
    accessor: AccessorIdx,
) -> Result<(), EngineError> {
    push_value(stack, eval_accessor_inner(plan, run, store, accessor)?)
}

pub fn eval_expr_with_store(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: &mut ValueStore,
    expr: ExprIdx,
) -> Result<SlotValue, EngineError> {
    eval_expr_inner(plan, run, store, expr)
}

pub fn eval_expr(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    expr: ExprIdx,
) -> Result<SlotValue, EngineError> {
    let mut store = ValueStore::new();
    eval_expr_inner(plan, run, &mut store, expr)
}

fn eval_accessor_inner(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: &mut ValueStore,
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
    let program = plan
        .accessor(accessor)
        .ok_or(EngineError::InvalidCompiledWorkflow {
            reason: "accessor index out of bounds",
        })?;
    eval_accessor_program_without_store(run, program)
}

pub fn eval_accessor_with_store(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: &mut ValueStore,
    accessor: AccessorIdx,
) -> Result<SlotValue, EngineError> {
    eval_accessor_inner(plan, run, store, accessor)
}

fn eval_accessor_program_without_store(
    run: &crate::RunFrame,
    program: &AccessorProgram,
) -> Result<SlotValue, EngineError> {
    let current = *run.read_slot(program.root)?;
    if program.path.is_empty() {
        return Ok(current);
    }
    let segment = program.path.first().copied().ok_or({
        EngineError::InternalInvariantViolation {
            reason: "accessor path checked non-empty",
        }
    })?;
    Err(EngineError::UnsupportedAccessorTraversal {
        segment: path_segment_name(segment),
        found: current.type_name(),
    })
}

fn eval_accessor_program(
    run: &crate::RunFrame,
    store: &mut ValueStore,
    program: &AccessorProgram,
) -> Result<SlotValue, EngineError> {
    let mut current = *run.read_slot(program.root)?;
    if program.path.is_empty() {
        return Ok(current);
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ExprIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
    use crate::limits::MAX_EXPRESSION_STACK;
    use crate::workflow::{
        CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, ResourceContract, WorkflowParts,
        check_expr_stack_bound,
    };

    fn empty_plan_with_expr(
        ops: Box<[ExprOp]>,
        constants: Box<[crate::value::ConstValue]>,
    ) -> Result<CompiledWorkflow, EngineError> {
        let max_stack = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK).map_err(|_| {
            EngineError::InvalidCompiledWorkflow {
                reason: "stack check failed",
            }
        })?;
        let expr = ExprProgram::try_from_parts(ops, max_stack).map_err(|_| {
            EngineError::InvalidCompiledWorkflow {
                reason: "expr parts",
            }
        })?;
        CompiledWorkflow::try_from_parts(WorkflowParts {
            name: "test".into(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                kind: CompiledNodeKind::Nop,
                next: None,
                output: None,
            }]
            .into(),
            expressions: vec![expr].into(),
            accessors: vec![].into(),
            constants,
            slot_count: 8,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        })
        .map_err(|_| EngineError::InvalidCompiledWorkflow {
            reason: "workflow parts",
        })
    }

    fn run_frame_with_slots(slots: Vec<SlotValue>) -> Result<crate::RunFrame, EngineError> {
        let mut run = crate::RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 8)?;
        for (i, value) in slots.iter().enumerate() {
            let idx = SlotIdx::new(i as u16);
            run.write_slot(idx, *value)?;
        }
        Ok(run)
    }

    fn eval_expr_ops_with_constants(
        ops: &[ExprOp],
        constants: Vec<crate::value::ConstValue>,
        store: &mut ValueStore,
    ) -> Result<SlotValue, EngineError> {
        let plan = empty_plan_with_expr(ops.into(), constants.into())?;
        let run = run_frame_with_slots(vec![])?;
        eval_expr_with_store(&plan, &run, store, ExprIdx::new(0))
    }

    #[test]
    fn contains_finds_substring() -> Result<(), EngineError> {
        let mut store = ValueStore::new();
        let haystack = store.insert_symbol("hello world")?;
        let needle = store.insert_symbol("world")?;
        let ops = vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Contains,
        ];
        let result = eval_expr_ops_with_constants(
            &ops,
            vec![
                crate::value::ConstValue::Symbol(haystack),
                crate::value::ConstValue::Symbol(needle),
            ],
            &mut store,
        )?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn contains_rejects_missing_substring() -> Result<(), EngineError> {
        let mut store = ValueStore::new();
        let haystack = store.insert_symbol("hello")?;
        let needle = store.insert_symbol("xyz")?;
        let ops = vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Contains,
        ];
        let result = eval_expr_ops_with_constants(
            &ops,
            vec![
                crate::value::ConstValue::Symbol(haystack),
                crate::value::ConstValue::Symbol(needle),
            ],
            &mut store,
        )?;
        assert_eq!(result, SlotValue::Bool(false));
        Ok(())
    }

    #[test]
    fn starts_with_matches_prefix() -> Result<(), EngineError> {
        let mut store = ValueStore::new();
        let text = store.insert_symbol("hello world")?;
        let prefix = store.insert_symbol("hello")?;
        let ops = vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::StartsWith,
        ];
        let result = eval_expr_ops_with_constants(
            &ops,
            vec![
                crate::value::ConstValue::Symbol(text),
                crate::value::ConstValue::Symbol(prefix),
            ],
            &mut store,
        )?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn ends_with_matches_suffix() -> Result<(), EngineError> {
        let mut store = ValueStore::new();
        let text = store.insert_symbol("hello world")?;
        let suffix = store.insert_symbol("world")?;
        let ops = vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::EndsWith,
        ];
        let result = eval_expr_ops_with_constants(
            &ops,
            vec![
                crate::value::ConstValue::Symbol(text),
                crate::value::ConstValue::Symbol(suffix),
            ],
            &mut store,
        )?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    fn eval_expr_ops_with_store(
        ops: &[ExprOp],
        slots: Vec<SlotValue>,
        constants: Vec<crate::value::ConstValue>,
        store: &mut ValueStore,
    ) -> Result<SlotValue, EngineError> {
        let plan = empty_plan_with_expr(ops.into(), constants.into())?;
        let run = run_frame_with_slots(slots)?;
        eval_expr_with_store(&plan, &run, store, ExprIdx::new(0))
    }

    #[test]
    fn has_finds_element_in_list() -> Result<(), EngineError> {
        let mut store = ValueStore::new();
        let list = store.insert_list(
            vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)].into_boxed_slice(),
        )?;

        let ops = vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::Has,
        ];
        let result = eval_expr_ops_with_store(
            &ops,
            vec![SlotValue::List(list)],
            vec![crate::value::ConstValue::I64(20)],
            &mut store,
        )?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn exists_checks_object_field() -> Result<(), EngineError> {
        let mut store = ValueStore::new();
        let sym = store.insert_symbol("key")?;
        let obj = store.insert_object(
            vec![ObjectField {
                key: sym,
                value: SlotValue::Bool(true),
            }]
            .into_boxed_slice(),
        )?;

        let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Exists];
        let result =
            eval_expr_ops_with_store(&ops, vec![SlotValue::Object(obj)], vec![], &mut store)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn length_counts_list_items() -> Result<(), EngineError> {
        let mut store = ValueStore::new();
        let list = store.insert_list(
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)].into_boxed_slice(),
        )?;

        let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Length];
        let result =
            eval_expr_ops_with_store(&ops, vec![SlotValue::List(list)], vec![], &mut store)?;
        assert_eq!(result, SlotValue::I64(3));
        Ok(())
    }

    #[test]
    fn empty_detects_empty_list() -> Result<(), EngineError> {
        let mut store = ValueStore::new();
        let list = store.insert_list(vec![].into_boxed_slice())?;

        let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Empty];
        let result =
            eval_expr_ops_with_store(&ops, vec![SlotValue::List(list)], vec![], &mut store)?;
        assert_eq!(result, SlotValue::Bool(true));
        Ok(())
    }

    #[test]
    fn append_adds_to_list() -> Result<(), EngineError> {
        let mut store = ValueStore::new();
        let list = store.insert_list(vec![SlotValue::I64(1)].into_boxed_slice())?;

        let ops = vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::Append,
        ];
        let result = eval_expr_ops_with_store(
            &ops,
            vec![SlotValue::List(list)],
            vec![crate::value::ConstValue::I64(2)],
            &mut store,
        )?;
        let result_list_id = expect_list(result)?;
        let items = store.list(result_list_id)?;
        assert_eq!(items.len(), 2);
        Ok(())
    }

    #[test]
    fn append_if_conditionally_adds() -> Result<(), EngineError> {
        let mut store = ValueStore::new();
        let list = store.insert_list(vec![SlotValue::I64(1)].into_boxed_slice())?;

        let ops = vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::AppendIf,
        ];
        let result = eval_expr_ops_with_store(
            &ops,
            vec![SlotValue::List(list)],
            vec![
                crate::value::ConstValue::I64(2),
                crate::value::ConstValue::Bool(true),
            ],
            &mut store,
        )?;
        let result_list_id = expect_list(result)?;
        let items = store.list(result_list_id)?;
        assert_eq!(items.len(), 2);
        Ok(())
    }

    #[test]
    fn merge_combines_objects() -> Result<(), EngineError> {
        let mut store = ValueStore::new();
        let sym1 = store.insert_symbol("a")?;
        let sym2 = store.insert_symbol("b")?;
        let obj1 = store.insert_object(
            vec![ObjectField {
                key: sym1,
                value: SlotValue::I64(1),
            }]
            .into_boxed_slice(),
        )?;
        let obj2 = store.insert_object(
            vec![ObjectField {
                key: sym2,
                value: SlotValue::I64(2),
            }]
            .into_boxed_slice(),
        )?;

        let ops = vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Merge,
        ];
        let result = eval_expr_ops_with_store(
            &ops,
            vec![SlotValue::Object(obj1), SlotValue::Object(obj2)],
            vec![],
            &mut store,
        )?;
        let result_obj_id = expect_object(result)?;
        let merged = store.object(result_obj_id)?;
        assert_eq!(merged.len(), 2);
        Ok(())
    }

    #[test]
    fn sum_computes_list_total() -> Result<(), EngineError> {
        let mut store = ValueStore::new();
        let list = store.insert_list(
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)].into_boxed_slice(),
        )?;

        let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Sum];
        let result =
            eval_expr_ops_with_store(&ops, vec![SlotValue::List(list)], vec![], &mut store)?;
        assert_eq!(result, SlotValue::I64(6));
        Ok(())
    }

    #[test]
    fn count_computes_list_length() -> Result<(), EngineError> {
        let mut store = ValueStore::new();
        let list =
            store.insert_list(vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice())?;

        let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Count];
        let result =
            eval_expr_ops_with_store(&ops, vec![SlotValue::List(list)], vec![], &mut store)?;
        assert_eq!(result, SlotValue::I64(2));
        Ok(())
    }

    #[test]
    fn unique_removes_duplicates() -> Result<(), EngineError> {
        let mut store = ValueStore::new();
        let list = store.insert_list(
            vec![SlotValue::I64(1), SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice(),
        )?;

        let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Unique];
        let result =
            eval_expr_ops_with_store(&ops, vec![SlotValue::List(list)], vec![], &mut store)?;
        let result_list_id = expect_list(result)?;
        let items = store.list(result_list_id)?;
        assert_eq!(items.len(), 2);
        Ok(())
    }
}
