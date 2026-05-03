//! Text and list operations for expression evaluation.

use crate::errors::EngineError;
use crate::value::SlotValue;
use crate::value_store::ValueStore;

use super::stack::{
    ExprStack, expect_i64, expect_list, expect_symbol, pop_pair, pop_triple, push_value,
};

// ===== Text operations =====

pub(super) fn eval_contains(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
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

pub(super) fn eval_starts_with(
    stack: &mut ExprStack,
    store: &ValueStore,
) -> Result<(), EngineError> {
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

pub(super) fn eval_ends_with(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
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

pub(super) fn eval_has(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
    let (list, item) = pop_pair(stack)?;
    let list_id = expect_list(list)?;
    let items = store
        .list(list_id)
        .map_err(|_| EngineError::ListOutOfBounds { list: list_id })?;
    let found = items.contains(&item);
    push_value(stack, SlotValue::Bool(found))
}

pub(super) fn eval_length(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
    let value = super::stack::pop_value(stack)?;
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

pub(super) fn eval_empty(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
    let value = super::stack::pop_value(stack)?;
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

pub(super) fn eval_sum(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
    let value = super::stack::pop_value(stack)?;
    let list_id = expect_list(value)?;
    let items = store
        .list(list_id)
        .map_err(|_| EngineError::ListOutOfBounds { list: list_id })?;
    let mut sum: i64 = 0;
    for &item in items {
        let n = expect_i64(item)?;
        sum = sum
            .checked_add(n)
            .ok_or(EngineError::InvalidCompiledWorkflow {
                reason: "sum overflow",
            })?;
    }
    push_value(stack, SlotValue::I64(sum))
}

pub(super) fn eval_count(stack: &mut ExprStack, store: &ValueStore) -> Result<(), EngineError> {
    let value = super::stack::pop_value(stack)?;
    let list_id = expect_list(value)?;
    let items = store
        .list(list_id)
        .map_err(|_| EngineError::ListOutOfBounds { list: list_id })?;
    let count = i64::try_from(items.len()).map_err(|_| EngineError::InvalidCompiledWorkflow {
        reason: "count exceeds i64 range",
    })?;
    push_value(stack, SlotValue::I64(count))
}

pub(super) fn eval_append(
    stack: &mut ExprStack,
    store: &mut ValueStore,
) -> Result<(), EngineError> {
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

pub(super) fn eval_append_if(
    stack: &mut ExprStack,
    store: &mut ValueStore,
) -> Result<(), EngineError> {
    let (list, item, condition) = pop_triple(stack)?;
    let list_id = expect_list(list)?;
    let cond = super::stack::expect_bool(condition)?;
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

pub(super) fn eval_unique(
    stack: &mut ExprStack,
    store: &mut ValueStore,
) -> Result<(), EngineError> {
    let value = super::stack::pop_value(stack)?;
    let list_id = expect_list(value)?;
    let items = store
        .list(list_id)
        .map_err(|_| EngineError::ListOutOfBounds { list: list_id })?;
    let mut seen: Vec<SlotValue> = Vec::new();
    for &item in items {
        if !seen.contains(&item) {
            seen.push(item);
        }
    }
    let new_list = store
        .insert_list(seen.into_boxed_slice())
        .map_err(|_| EngineError::AllocationFailed)?;
    push_value(stack, SlotValue::List(new_list))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::EngineError;
    use crate::ids::{ConstIdx, ExprIdx, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
    use crate::limits::MAX_EXPRESSION_STACK;
    use crate::value::{ConstValue, SlotValue};
    use crate::workflow::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram,
        ResourceContract, WorkflowParts, check_expr_stack_bound,
    };

    fn ensure_equal<T>(actual: T, expected: T) -> Result<(), String>
    where
        T: core::fmt::Debug + PartialEq,
    {
        if actual == expected {
            Ok(())
        } else {
            Err(format!("expected {expected:?}, found {actual:?}"))
        }
    }

    fn eval_ops(
        ops: Vec<ExprOp>,
        constants: Vec<ConstValue>,
        store: &mut ValueStore,
    ) -> Result<SlotValue, String> {
        eval_ops_with_slots(ops, vec![], constants, store)
    }

    fn eval_ops_with_slots(
        ops: Vec<ExprOp>,
        slots: Vec<SlotValue>,
        constants: Vec<ConstValue>,
        store: &mut ValueStore,
    ) -> Result<SlotValue, String> {
        let max_stack = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK)
            .map_err(|e| e.to_string())?;
        let expr = ExprProgram::try_from_parts(ops.into_boxed_slice(), max_stack)
            .map_err(|e| e.to_string())?;
        let slot_count = if slots.is_empty() { 1u16 } else { slots.len() as u16 };
        let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::<str>::from("text_list_test"),
            digest: WorkflowDigest::from_bytes([0xFB; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            }]
            .into_boxed_slice(),
            expressions: vec![expr].into_boxed_slice(),
            accessors: Box::new([]),
            constants: constants.into_boxed_slice(),
            slot_count,
            symbols_count: 10,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .map_err(|e| e.to_string())?;
        let mut run = crate::frame::RunFrame::new(RunId::new(1), StepIdx::new(0), 1, slot_count)
            .map_err(|e| e.to_string())?;
        for (i, value) in slots.iter().enumerate() {
            run.write_slot(SlotIdx::new(i as u16), *value)
                .map_err(|e| e.to_string())?;
        }
        let (value, _taint) = crate::engine::expr_eval::eval_expr_with_store(
            &workflow,
            &run,
            store,
            ExprIdx::new(0),
        )
        .map_err(|e| e.to_string())?;
        Ok(value)
    }

    // ===== Text operations =====

    #[test]
    fn contains_matches_substring() -> Result<(), String> {
        let mut store = ValueStore::new();
        let hay = store.insert_symbol("hello world").map_err(|e| e.to_string())?;
        let needle = store.insert_symbol("world").map_err(|e| e.to_string())?;
        let result = eval_ops(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Contains,
            ],
            vec![ConstValue::Symbol(hay), ConstValue::Symbol(needle)],
            &mut store,
        )?;
        ensure_equal(result, SlotValue::Bool(true))
    }

    #[test]
    fn contains_rejects_non_matching() -> Result<(), String> {
        let mut store = ValueStore::new();
        let hay = store.insert_symbol("hello").map_err(|e| e.to_string())?;
        let needle = store.insert_symbol("xyz").map_err(|e| e.to_string())?;
        let result = eval_ops(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Contains,
            ],
            vec![ConstValue::Symbol(hay), ConstValue::Symbol(needle)],
            &mut store,
        )?;
        ensure_equal(result, SlotValue::Bool(false))
    }

    #[test]
    fn starts_with_matches_prefix() -> Result<(), String> {
        let mut store = ValueStore::new();
        let text = store.insert_symbol("hello world").map_err(|e| e.to_string())?;
        let prefix = store.insert_symbol("hello").map_err(|e| e.to_string())?;
        let result = eval_ops(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::StartsWith,
            ],
            vec![ConstValue::Symbol(text), ConstValue::Symbol(prefix)],
            &mut store,
        )?;
        ensure_equal(result, SlotValue::Bool(true))
    }

    #[test]
    fn starts_with_rejects_non_prefix() -> Result<(), String> {
        let mut store = ValueStore::new();
        let text = store.insert_symbol("hello world").map_err(|e| e.to_string())?;
        let prefix = store.insert_symbol("world").map_err(|e| e.to_string())?;
        let result = eval_ops(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::StartsWith,
            ],
            vec![ConstValue::Symbol(text), ConstValue::Symbol(prefix)],
            &mut store,
        )?;
        ensure_equal(result, SlotValue::Bool(false))
    }

    #[test]
    fn ends_with_matches_suffix() -> Result<(), String> {
        let mut store = ValueStore::new();
        let text = store.insert_symbol("hello world").map_err(|e| e.to_string())?;
        let suffix = store.insert_symbol("world").map_err(|e| e.to_string())?;
        let result = eval_ops(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::EndsWith,
            ],
            vec![ConstValue::Symbol(text), ConstValue::Symbol(suffix)],
            &mut store,
        )?;
        ensure_equal(result, SlotValue::Bool(true))
    }

    #[test]
    fn ends_with_rejects_non_suffix() -> Result<(), String> {
        let mut store = ValueStore::new();
        let text = store.insert_symbol("hello world").map_err(|e| e.to_string())?;
        let suffix = store.insert_symbol("hello").map_err(|e| e.to_string())?;
        let result = eval_ops(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::EndsWith,
            ],
            vec![ConstValue::Symbol(text), ConstValue::Symbol(suffix)],
            &mut store,
        )?;
        ensure_equal(result, SlotValue::Bool(false))
    }

    // ===== List operations =====

    #[test]
    fn has_finds_existing_element() -> Result<(), String> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![SlotValue::I64(10), SlotValue::I64(20)].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Has,
            ],
            vec![SlotValue::List(list)],
            vec![ConstValue::I64(20)],
            &mut store,
        )?;
        ensure_equal(result, SlotValue::Bool(true))
    }

    #[test]
    fn has_returns_false_for_missing_element() -> Result<(), String> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![SlotValue::I64(10)].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Has,
            ],
            vec![SlotValue::List(list)],
            vec![ConstValue::I64(99)],
            &mut store,
        )?;
        ensure_equal(result, SlotValue::Bool(false))
    }

    #[test]
    fn length_counts_text_characters() -> Result<(), String> {
        let mut store = ValueStore::new();
        let sym = store.insert_symbol("abc").map_err(|e| e.to_string())?;
        let result = eval_ops(
            vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Length],
            vec![ConstValue::Symbol(sym)],
            &mut store,
        )?;
        ensure_equal(result, SlotValue::I64(3))
    }

    #[test]
    fn length_counts_list_items() -> Result<(), String> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(
                vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)].into_boxed_slice(),
            )
            .map_err(|e| e.to_string())?;
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Length],
            vec![SlotValue::List(list)],
            vec![],
            &mut store,
        )?;
        ensure_equal(result, SlotValue::I64(3))
    }

    #[test]
    fn length_counts_object_fields() -> Result<(), String> {
        let mut store = ValueStore::new();
        let obj = store
            .insert_object(
                vec![
                    crate::value_store::ObjectField {
                        key: SymbolId::new(0),
                        value: SlotValue::I64(1),
                    },
                    crate::value_store::ObjectField {
                        key: SymbolId::new(1),
                        value: SlotValue::I64(2),
                    },
                ]
                .into_boxed_slice(),
            )
            .map_err(|e| e.to_string())?;
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Length],
            vec![SlotValue::Object(obj)],
            vec![],
            &mut store,
        )?;
        ensure_equal(result, SlotValue::I64(2))
    }

    #[test]
    fn empty_on_null_returns_true() -> Result<(), String> {
        let mut store = ValueStore::new();
        let result = eval_ops(
            vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Empty],
            vec![ConstValue::Null],
            &mut store,
        )?;
        ensure_equal(result, SlotValue::Bool(true))
    }

    #[test]
    fn empty_on_empty_list_returns_true() -> Result<(), String> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Empty],
            vec![SlotValue::List(list)],
            vec![],
            &mut store,
        )?;
        ensure_equal(result, SlotValue::Bool(true))
    }

    #[test]
    fn empty_on_non_empty_list_returns_false() -> Result<(), String> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Empty],
            vec![SlotValue::List(list)],
            vec![],
            &mut store,
        )?;
        ensure_equal(result, SlotValue::Bool(false))
    }

    #[test]
    fn sum_computes_total() -> Result<(), String> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(
                vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)].into_boxed_slice(),
            )
            .map_err(|e| e.to_string())?;
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Sum],
            vec![SlotValue::List(list)],
            vec![],
            &mut store,
        )?;
        ensure_equal(result, SlotValue::I64(6))
    }

    #[test]
    fn sum_empty_list_produces_zero() -> Result<(), String> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Sum],
            vec![SlotValue::List(list)],
            vec![],
            &mut store,
        )?;
        ensure_equal(result, SlotValue::I64(0))
    }

    #[test]
    fn count_returns_list_length() -> Result<(), String> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Count],
            vec![SlotValue::List(list)],
            vec![],
            &mut store,
        )?;
        ensure_equal(result, SlotValue::I64(2))
    }

    #[test]
    fn append_adds_item_to_list() -> Result<(), String> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Append,
            ],
            vec![SlotValue::List(list)],
            vec![ConstValue::I64(2)],
            &mut store,
        )?;
        match result {
            SlotValue::List(id) => {
                let items = store.list(id).map_err(|e| e.to_string())?;
                ensure_equal(items.len(), 2)?;
                ensure_equal(items[0], SlotValue::I64(1))?;
                ensure_equal(items[1], SlotValue::I64(2))
            }
            other => Err(format!("expected List, got {other:?}")),
        }
    }

    #[test]
    fn append_if_true_adds_item() -> Result<(), String> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::AppendIf,
            ],
            vec![SlotValue::List(list)],
            vec![ConstValue::I64(2), ConstValue::Bool(true)],
            &mut store,
        )?;
        match result {
            SlotValue::List(id) => {
                let items = store.list(id).map_err(|e| e.to_string())?;
                ensure_equal(items.len(), 2)
            }
            other => Err(format!("expected List, got {other:?}")),
        }
    }

    #[test]
    fn append_if_false_does_not_add_item() -> Result<(), String> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::AppendIf,
            ],
            vec![SlotValue::List(list)],
            vec![ConstValue::I64(2), ConstValue::Bool(false)],
            &mut store,
        )?;
        match result {
            SlotValue::List(id) => {
                let items = store.list(id).map_err(|e| e.to_string())?;
                ensure_equal(items.len(), 1)
            }
            other => Err(format!("expected List, got {other:?}")),
        }
    }

    #[test]
    fn unique_removes_duplicates_preserving_order() -> Result<(), String> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(
                vec![
                    SlotValue::I64(1),
                    SlotValue::I64(2),
                    SlotValue::I64(1),
                    SlotValue::I64(3),
                    SlotValue::I64(2),
                ]
                .into_boxed_slice(),
            )
            .map_err(|e| e.to_string())?;
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Unique],
            vec![SlotValue::List(list)],
            vec![],
            &mut store,
        )?;
        match result {
            SlotValue::List(id) => {
                let items = store.list(id).map_err(|e| e.to_string())?;
                ensure_equal(items.len(), 3)?;
                ensure_equal(items[0], SlotValue::I64(1))?;
                ensure_equal(items[1], SlotValue::I64(2))?;
                ensure_equal(items[2], SlotValue::I64(3))
            }
            other => Err(format!("expected List, got {other:?}")),
        }
    }

    #[test]
    fn unique_empty_list_produces_empty_list() -> Result<(), String> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Unique],
            vec![SlotValue::List(list)],
            vec![],
            &mut store,
        )?;
        match result {
            SlotValue::List(id) => {
                let items = store.list(id).map_err(|e| e.to_string())?;
                ensure_equal(items.is_empty(), true)
            }
            other => Err(format!("expected List, got {other:?}")),
        }
    }

    #[test]
    fn contains_rejects_non_symbol_haystack() -> Result<(), String> {
        let mut store = ValueStore::new();
        let needle = store.insert_symbol("a").map_err(|e| e.to_string())?;
        let result = eval_ops(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Contains,
            ],
            vec![ConstValue::I64(42), ConstValue::Symbol(needle)],
            &mut store,
        );
        match result {
            Err(msg) if msg.contains("TypeMismatch") || msg.contains("text") => Ok(()),
            other => Err(format!("expected type error, got {other:?}")),
        }
    }

    #[test]
    fn sum_overflow_returns_error() -> Result<(), String> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![SlotValue::I64(i64::MAX), SlotValue::I64(1)].into_boxed_slice())
            .map_err(|e| e.to_string())?;
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Sum],
            vec![SlotValue::List(list)],
            vec![],
            &mut store,
        );
        match result {
            Err(msg) if msg.contains("overflow") => Ok(()),
            other => Err(format!("expected overflow error, got {other:?}")),
        }
    }
}
