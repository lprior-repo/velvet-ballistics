#![forbid(unsafe_code)]
//! Expression evaluation operations for replay.

use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::{AccessorIdx, ConstIdx, ListId, ObjectId, SlotIdx, StepIdx, SymbolId};
use crate::value::{SlotValue, Taint, join_taint};
use crate::value_store::ValueStore;
use crate::workflow::{CompiledWorkflow, ExprOp};
use indexmap::IndexSet;

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
        ExprOp::Neg => eval_neg(stack),
        ExprOp::Add => eval_add(stack),
        ExprOp::Sub => eval_sub(stack),
        ExprOp::Mul => eval_mul(stack),
        ExprOp::Div => eval_div(stack),
        ExprOp::Gt => eval_gt(stack),
        ExprOp::Gte => eval_gte(stack),
        ExprOp::Lt => eval_lt(stack),
        ExprOp::Lte => eval_lte(stack),
        ExprOp::Coalesce => eval_coalesce(stack),
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
        EngineError::SlotUninitialized { slot: s } => ReplayError::SlotNotAvailable { slot: s },
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

fn eval_coalesce(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_pair(stack)?;
    if left == SlotValue::Null {
        stack.push(right)
    } else {
        stack.push(left)
    }
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

pub(crate) fn eval_add(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    let result = left
        .checked_add(right)
        .ok_or(ReplayError::ExpressionEvalFailed {
            step: StepIdx::ZERO,
        })?;
    stack.push(SlotValue::I64(result))
}

pub(crate) fn eval_sub(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let (left, right) = pop_i64_pair(stack)?;
    let result = left
        .checked_sub(right)
        .ok_or(ReplayError::ExpressionEvalFailed {
            step: StepIdx::ZERO,
        })?;
    stack.push(SlotValue::I64(result))
}

pub(crate) fn eval_mul(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
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
        EngineError::SlotUninitialized { slot } => ReplayError::SlotNotAvailable { slot },
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

fn pop_triple_replay(
    stack: &mut ReplayExprStack,
) -> Result<(SlotValue, SlotValue, SlotValue), ReplayError> {
    let right = stack.pop()?;
    let mid = stack.pop()?;
    let left = stack.pop()?;
    Ok((left, mid, right))
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

fn expect_symbol_replay(value: SlotValue) -> Result<SymbolId, ReplayError> {
    match value {
        SlotValue::Symbol(id) => Ok(id),
        other => Err(replay_type_mismatch("text", other.type_name())),
    }
}

fn expect_list_replay(value: SlotValue) -> Result<ListId, ReplayError> {
    match value {
        SlotValue::List(id) => Ok(id),
        other => Err(replay_type_mismatch("list", other.type_name())),
    }
}

fn expect_object_replay(value: SlotValue) -> Result<ObjectId, ReplayError> {
    match value {
        SlotValue::Object(id) => Ok(id),
        other => Err(replay_type_mismatch("object", other.type_name())),
    }
}

fn replay_type_mismatch(_expected: &'static str, _found: &'static str) -> ReplayError {
    ReplayError::Internal {
        reason: "replay expression type mismatch",
    }
}

fn eval_neg(stack: &mut ReplayExprStack) -> Result<(), ReplayError> {
    let value = stack.pop()?;
    match value {
        SlotValue::I64(n) => {
            let result = n.checked_neg().ok_or(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            })?;
            stack.push(SlotValue::I64(result))
        }
        SlotValue::F64(f) => {
            let raw = -f.get();
            if raw.is_finite() {
                stack.push(SlotValue::F64(crate::value::FiniteF64::new(raw).map_err(
                    |_| ReplayError::ExpressionEvalFailed {
                        step: StepIdx::ZERO,
                    },
                )?))
            } else {
                Err(ReplayError::ExpressionEvalFailed {
                    step: StepIdx::ZERO,
                })
            }
        }
        other => Err(replay_type_mismatch("number", other.type_name())),
    }
}

fn eval_contains(stack: &mut ReplayExprStack, store: &ValueStore) -> Result<(), ReplayError> {
    let (haystack, needle) = pop_pair(stack)?;
    let haystack_id = expect_symbol_replay(haystack)?;
    let needle_id = expect_symbol_replay(needle)?;
    let haystack_str = store.symbol(haystack_id).map_err(|_| ReplayError::Internal {
        reason: "symbol out of bounds during replay contains",
    })?;
    let needle_str = store.symbol(needle_id).map_err(|_| ReplayError::Internal {
        reason: "symbol out of bounds during replay contains",
    })?;
    stack.push(SlotValue::Bool(haystack_str.contains(needle_str)))
}

fn eval_starts_with(
    stack: &mut ReplayExprStack,
    store: &ValueStore,
) -> Result<(), ReplayError> {
    let (text, prefix) = pop_pair(stack)?;
    let text_id = expect_symbol_replay(text)?;
    let prefix_id = expect_symbol_replay(prefix)?;
    let text_str = store.symbol(text_id).map_err(|_| ReplayError::Internal {
        reason: "symbol out of bounds during replay starts_with",
    })?;
    let prefix_str = store.symbol(prefix_id).map_err(|_| ReplayError::Internal {
        reason: "symbol out of bounds during replay starts_with",
    })?;
    stack.push(SlotValue::Bool(text_str.starts_with(prefix_str)))
}

fn eval_ends_with(stack: &mut ReplayExprStack, store: &ValueStore) -> Result<(), ReplayError> {
    let (text, suffix) = pop_pair(stack)?;
    let text_id = expect_symbol_replay(text)?;
    let suffix_id = expect_symbol_replay(suffix)?;
    let text_str = store.symbol(text_id).map_err(|_| ReplayError::Internal {
        reason: "symbol out of bounds during replay ends_with",
    })?;
    let suffix_str = store.symbol(suffix_id).map_err(|_| ReplayError::Internal {
        reason: "symbol out of bounds during replay ends_with",
    })?;
    stack.push(SlotValue::Bool(text_str.ends_with(suffix_str)))
}

fn eval_has(stack: &mut ReplayExprStack, store: &ValueStore) -> Result<(), ReplayError> {
    let (list, item) = pop_pair(stack)?;
    let list_id = expect_list_replay(list)?;
    let items = store.list(list_id).map_err(|_| ReplayError::Internal {
        reason: "list out of bounds during replay has",
    })?;
    stack.push(SlotValue::Bool(items.contains(&item)))
}

fn eval_exists(stack: &mut ReplayExprStack, store: &ValueStore) -> Result<(), ReplayError> {
    let value = stack.pop()?;
    match value {
        SlotValue::Null => stack.push(SlotValue::Bool(false)),
        SlotValue::Object(object_id) => {
            let fields = store.object(object_id).map_err(|_| ReplayError::Internal {
                reason: "object out of bounds during replay exists",
            })?;
            stack.push(SlotValue::Bool(!fields.is_empty()))
        }
        other => Err(replay_type_mismatch("object or null", other.type_name())),
    }
}

fn eval_length(stack: &mut ReplayExprStack, store: &ValueStore) -> Result<(), ReplayError> {
    let value = stack.pop()?;
    let len = match value {
        SlotValue::Symbol(id) => store.symbol(id).map_err(|_| ReplayError::Internal {
            reason: "symbol out of bounds during replay length",
        })?.len(),
        SlotValue::List(id) => store.list(id).map_err(|_| ReplayError::Internal {
            reason: "list out of bounds during replay length",
        })?.len(),
        SlotValue::Object(id) => store.object(id).map_err(|_| ReplayError::Internal {
            reason: "object out of bounds during replay length",
        })?.len(),
        other => return Err(replay_type_mismatch("text, list, or object", other.type_name())),
    };
    let len_i64 = i64::try_from(len).map_err(|_| ReplayError::ExpressionEvalFailed {
        step: StepIdx::ZERO,
    })?;
    stack.push(SlotValue::I64(len_i64))
}

fn eval_empty(stack: &mut ReplayExprStack, store: &ValueStore) -> Result<(), ReplayError> {
    let value = stack.pop()?;
    let is_empty = match value {
        SlotValue::Null => true,
        SlotValue::Symbol(id) => store.symbol(id).map_err(|_| ReplayError::Internal {
            reason: "symbol out of bounds during replay empty",
        })?.is_empty(),
        SlotValue::List(id) => store.list(id).map_err(|_| ReplayError::Internal {
            reason: "list out of bounds during replay empty",
        })?.is_empty(),
        SlotValue::Object(id) => store.object(id).map_err(|_| ReplayError::Internal {
            reason: "object out of bounds during replay empty",
        })?.is_empty(),
        other => return Err(replay_type_mismatch("text, list, object, or null", other.type_name())),
    };
    stack.push(SlotValue::Bool(is_empty))
}

fn eval_append(stack: &mut ReplayExprStack, store: &mut ValueStore) -> Result<(), ReplayError> {
    let (list, item) = pop_pair(stack)?;
    let list_id = expect_list_replay(list)?;
    let items = store.list(list_id).map_err(|_| ReplayError::Internal {
        reason: "list out of bounds during replay append",
    })?;
    let mut new_items: Vec<SlotValue> = items.to_vec();
    new_items.push(item);
    let new_list = store
        .insert_list(new_items.into_boxed_slice())
        .map_err(|_| ReplayError::Internal {
            reason: "list allocation failed during replay append",
        })?;
    stack.push(SlotValue::List(new_list))
}

fn eval_append_if(
    stack: &mut ReplayExprStack,
    store: &mut ValueStore,
) -> Result<(), ReplayError> {
    let (list, item, condition) = pop_triple_replay(stack)?;
    let list_id = expect_list_replay(list)?;
    let cond = expect_bool_replay(condition)?;
    let items = store.list(list_id).map_err(|_| ReplayError::Internal {
        reason: "list out of bounds during replay append_if",
    })?;
    let mut new_items: Vec<SlotValue> = items.to_vec();
    if cond {
        new_items.push(item);
    }
    let new_list = store
        .insert_list(new_items.into_boxed_slice())
        .map_err(|_| ReplayError::Internal {
            reason: "list allocation failed during replay append_if",
        })?;
    stack.push(SlotValue::List(new_list))
}

fn eval_merge(stack: &mut ReplayExprStack, store: &mut ValueStore) -> Result<(), ReplayError> {
    let (left, right) = pop_pair(stack)?;
    let left_id = expect_object_replay(left)?;
    let right_id = expect_object_replay(right)?;
    let left_fields = store.object(left_id).map_err(|_| ReplayError::Internal {
        reason: "object out of bounds during replay merge",
    })?;
    let right_fields = store.object(right_id).map_err(|_| ReplayError::Internal {
        reason: "object out of bounds during replay merge",
    })?;
    let mut merged: Vec<crate::value_store::ObjectField> =
        Vec::with_capacity(left_fields.len().saturating_add(right_fields.len()));
    let mut index: std::collections::HashMap<crate::ids::SymbolId, usize> =
        std::collections::HashMap::with_capacity(
            left_fields.len().saturating_add(right_fields.len()),
        );
    for &field in left_fields {
        index.insert(field.key, merged.len());
        merged.push(field);
    }
    for &field in right_fields {
        match index.get(&field.key).copied() {
            Some(pos) => {
                if let Some(entry) = merged.get_mut(pos) {
                    *entry = field;
                }
            }
            None => {
                index.insert(field.key, merged.len());
                merged.push(field);
            }
        }
    }
    let new_object = store
        .insert_object(merged.into_boxed_slice())
        .map_err(|_| ReplayError::Internal {
            reason: "object allocation failed during replay merge",
        })?;
    stack.push(SlotValue::Object(new_object))
}

fn eval_sum(stack: &mut ReplayExprStack, store: &ValueStore) -> Result<(), ReplayError> {
    let value = stack.pop()?;
    let list_id = expect_list_replay(value)?;
    let items = store.list(list_id).map_err(|_| ReplayError::Internal {
        reason: "list out of bounds during replay sum",
    })?;
    let mut sum: i64 = 0;
    for &item in items {
        let n = expect_i64_replay(item)?;
        sum = sum.checked_add(n).ok_or(ReplayError::ExpressionEvalFailed {
            step: StepIdx::ZERO,
        })?;
    }
    stack.push(SlotValue::I64(sum))
}

fn eval_count(stack: &mut ReplayExprStack, store: &ValueStore) -> Result<(), ReplayError> {
    let value = stack.pop()?;
    let list_id = expect_list_replay(value)?;
    let items = store.list(list_id).map_err(|_| ReplayError::Internal {
        reason: "list out of bounds during replay count",
    })?;
    let count = i64::try_from(items.len()).map_err(|_| ReplayError::ExpressionEvalFailed {
        step: StepIdx::ZERO,
    })?;
    stack.push(SlotValue::I64(count))
}

fn eval_unique(stack: &mut ReplayExprStack, store: &mut ValueStore) -> Result<(), ReplayError> {
    let value = stack.pop()?;
    let list_id = expect_list_replay(value)?;
    let items = store.list(list_id).map_err(|_| ReplayError::Internal {
        reason: "list out of bounds during replay unique",
    })?;
    let seen: IndexSet<SlotValue> = items.iter().copied().collect();
    let new_list = store
        .insert_list(seen.into_iter().collect::<Vec<_>>().into_boxed_slice())
        .map_err(|_| ReplayError::Internal {
            reason: "list allocation failed during replay unique",
        })?;
    stack.push(SlotValue::List(new_list))
}

#[cfg(test)]
#[path = "ops/tests.rs"]
mod tests;
