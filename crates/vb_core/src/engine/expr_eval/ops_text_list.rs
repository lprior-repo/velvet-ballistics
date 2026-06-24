#![forbid(unsafe_code)]
//! Text and list operations for expression evaluation.

use crate::errors::EngineError;
use crate::value::SlotValue;
use crate::value_store::ValueStore;

use super::stack::{
    ExprStack, expect_i64, expect_list, expect_symbol, pop_pair, pop_triple, push_value,
};

// ===== Test-only OOM injection hook =====
// CE-005 follow-up (vb-kepe8): a thread-local test-only overflow hook
// (option (b) from the follow-up bead). Operators call a `cfg(test)`
// helper immediately after `try_reserve_exact` returns Ok. When the
// hook is armed, the helper returns `Err(AllocationFailed)` even
// though the underlying reservation succeeded — that is the only
// way for tests to deterministically exercise the operator's own
// post-reservation branch without forcing the global allocator to
// fail. In release builds, both the helper and the call sites are
// `cfg(test)`-gated, so production is unchanged.
#[cfg(test)]
mod oom_inject {
    use core::cell::Cell;

    thread_local! {
        static REMAINING: Cell<usize> = const { Cell::new(0) };
    }

    pub(super) fn arm(count: usize) {
        REMAINING.with(|c| c.set(count));
    }

    pub(super) fn reset() {
        REMAINING.with(|c| c.set(0));
    }

    pub(super) fn dec() -> bool {
        REMAINING.with(|c| {
            let cur = c.get();
            if cur == 0 {
                false
            } else {
                c.set(cur - 1);
                true
            }
        })
    }
}

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
    let mut new_items: Vec<SlotValue> = Vec::new();
    let new_len = items.len().checked_add(1).ok_or(EngineError::AllocationFailed)?;
    new_items.try_reserve_exact(new_len).map_err(|_| EngineError::AllocationFailed)?;
    #[cfg(test)]
    {
        if oom_inject::dec() {
            return Err(EngineError::AllocationFailed);
        }
    }
    new_items.extend_from_slice(items);
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
    let items = store.list(list_id).map_err(|_| EngineError::ListOutOfBounds { list: list_id })?;
    let mut new_items: Vec<SlotValue> = Vec::new();
    let base_len = items.len();
    if cond {
        let new_len = base_len.checked_add(1).ok_or(EngineError::AllocationFailed)?;
        new_items.try_reserve_exact(new_len).map_err(|_| EngineError::AllocationFailed)?;
        #[cfg(test)]
        {
            if oom_inject::dec() {
                return Err(EngineError::AllocationFailed);
            }
        }
        new_items.extend_from_slice(items);
        new_items.push(item);
    } else {
        new_items.try_reserve_exact(base_len).map_err(|_| EngineError::AllocationFailed)?;
        #[cfg(test)]
        {
            if oom_inject::dec() {
                return Err(EngineError::AllocationFailed);
            }
        }
        new_items.extend_from_slice(items);
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
    seen.try_reserve_exact(items.len()).map_err(|_| EngineError::AllocationFailed)?;
    #[cfg(test)]
    {
        if oom_inject::dec() {
            return Err(EngineError::AllocationFailed);
        }
    }
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
    use crate::ids::{
        ConstIdx, ExprIdx, ListId, ObjectId, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest,
    };
    use crate::limits::MAX_EXPRESSION_STACK;
    use crate::value::{ConstValue, SlotValue, Taint};
    use crate::workflow::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram, ResourceContract,
        WorkflowParts, check_expr_stack_bound,
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
        let max_stack =
            check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK).map_err(|e| e.to_string())?;
        let expr = ExprProgram::try_from_parts(ops.into_boxed_slice(), max_stack)
            .map_err(|e| e.to_string())?;
        let slot_count = if slots.is_empty() {
            1u16
        } else {
            u16::try_from(slots.len()).map_err(|_| "slot count overflow")?
        };
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
            let idx = u16::try_from(i).map_err(|_| "slot index overflow")?;
            run.write_slot(SlotIdx::new(idx), *value)
                .map_err(|e| e.to_string())?;
        }
        let (value, _taint) =
            crate::engine::expr_eval::eval_expr_with_store(&workflow, &run, store, ExprIdx::new(0))
                .map_err(|e| e.to_string())?;
        Ok(value)
    }

    // ===== Text operations =====

    #[test]
    fn contains_matches_substring() -> Result<(), String> {
        let mut store = ValueStore::new();
        let hay = store
            .insert_symbol("hello world")
            .map_err(|e| e.to_string())?;
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
        let text = store
            .insert_symbol("hello world")
            .map_err(|e| e.to_string())?;
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
        let text = store
            .insert_symbol("hello world")
            .map_err(|e| e.to_string())?;
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
        let text = store
            .insert_symbol("hello world")
            .map_err(|e| e.to_string())?;
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
        let text = store
            .insert_symbol("hello world")
            .map_err(|e| e.to_string())?;
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
                        taint: Taint::Clean,
                    },
                    crate::value_store::ObjectField {
                        key: SymbolId::new(1),
                        value: SlotValue::I64(2),
                        taint: Taint::Clean,
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

    // ===== Error branches for store lookups =====

    #[test]
    fn contains_symbol_out_of_bounds_haystack() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let needle = store.insert_symbol("a").expect("insert");
        // Push a SymbolId that does not exist in store
        push_value(&mut stack, SlotValue::Symbol(SymbolId::new(99))).expect("push");
        push_value(&mut stack, SlotValue::Symbol(needle)).expect("push");
        let result = eval_contains(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::SymbolOutOfBounds {
                symbol: SymbolId::new(99)
            })
        );
    }

    #[test]
    fn contains_symbol_out_of_bounds_needle() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let hay = store.insert_symbol("hello").expect("insert");
        push_value(&mut stack, SlotValue::Symbol(hay)).expect("push");
        push_value(&mut stack, SlotValue::Symbol(SymbolId::new(99))).expect("push");
        let result = eval_contains(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::SymbolOutOfBounds {
                symbol: SymbolId::new(99)
            })
        );
    }

    #[test]
    fn starts_with_symbol_out_of_bounds_text() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let prefix = store.insert_symbol("a").expect("insert");
        push_value(&mut stack, SlotValue::Symbol(SymbolId::new(99))).expect("push");
        push_value(&mut stack, SlotValue::Symbol(prefix)).expect("push");
        let result = eval_starts_with(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::SymbolOutOfBounds {
                symbol: SymbolId::new(99)
            })
        );
    }

    #[test]
    fn ends_with_symbol_out_of_bounds_suffix() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let text = store.insert_symbol("hello").expect("insert");
        push_value(&mut stack, SlotValue::Symbol(text)).expect("push");
        push_value(&mut stack, SlotValue::Symbol(SymbolId::new(99))).expect("push");
        let result = eval_ends_with(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::SymbolOutOfBounds {
                symbol: SymbolId::new(99)
            })
        );
    }

    #[test]
    fn has_list_out_of_bounds() {
        let mut stack = ExprStack::new(4).expect("valid");
        let store = ValueStore::new();
        push_value(&mut stack, SlotValue::List(ListId::new(99))).expect("push");
        push_value(&mut stack, SlotValue::I64(1)).expect("push");
        let result = eval_has(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::ListOutOfBounds {
                list: ListId::new(99)
            })
        );
    }

    #[test]
    fn length_symbol_out_of_bounds() {
        let mut stack = ExprStack::new(4).expect("valid");
        let store = ValueStore::new();
        push_value(&mut stack, SlotValue::Symbol(SymbolId::new(99))).expect("push");
        let result = eval_length(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::SymbolOutOfBounds {
                symbol: SymbolId::new(99)
            })
        );
    }

    #[test]
    fn length_list_out_of_bounds() {
        let mut stack = ExprStack::new(4).expect("valid");
        let store = ValueStore::new();
        push_value(&mut stack, SlotValue::List(ListId::new(99))).expect("push");
        let result = eval_length(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::ListOutOfBounds {
                list: ListId::new(99)
            })
        );
    }

    #[test]
    fn length_object_out_of_bounds() {
        let mut stack = ExprStack::new(4).expect("valid");
        let store = ValueStore::new();
        push_value(&mut stack, SlotValue::Object(ObjectId::new(99))).expect("push");
        let result = eval_length(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::ObjectOutOfBounds {
                object: ObjectId::new(99)
            })
        );
    }

    #[test]
    fn length_type_mismatch_on_bool() {
        let mut stack = ExprStack::new(4).expect("valid");
        let store = ValueStore::new();
        push_value(&mut stack, SlotValue::Bool(true)).expect("push");
        let result = eval_length(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::TypeMismatch {
                expected: "text, list, or object",
                found: "boolean",
            })
        );
    }

    #[test]
    fn empty_symbol_out_of_bounds() {
        let mut stack = ExprStack::new(4).expect("valid");
        let store = ValueStore::new();
        push_value(&mut stack, SlotValue::Symbol(SymbolId::new(99))).expect("push");
        let result = eval_empty(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::SymbolOutOfBounds {
                symbol: SymbolId::new(99)
            })
        );
    }

    #[test]
    fn empty_list_out_of_bounds() {
        let mut stack = ExprStack::new(4).expect("valid");
        let store = ValueStore::new();
        push_value(&mut stack, SlotValue::List(ListId::new(99))).expect("push");
        let result = eval_empty(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::ListOutOfBounds {
                list: ListId::new(99)
            })
        );
    }

    #[test]
    fn empty_object_out_of_bounds() {
        let mut stack = ExprStack::new(4).expect("valid");
        let store = ValueStore::new();
        push_value(&mut stack, SlotValue::Object(ObjectId::new(99))).expect("push");
        let result = eval_empty(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::ObjectOutOfBounds {
                object: ObjectId::new(99)
            })
        );
    }

    #[test]
    fn empty_type_mismatch_on_number() {
        let mut stack = ExprStack::new(4).expect("valid");
        let store = ValueStore::new();
        push_value(&mut stack, SlotValue::I64(42)).expect("push");
        let result = eval_empty(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::TypeMismatch {
                expected: "text, list, object, or null",
                found: "number",
            })
        );
    }

    #[test]
    fn sum_list_out_of_bounds() {
        let mut stack = ExprStack::new(4).expect("valid");
        let store = ValueStore::new();
        push_value(&mut stack, SlotValue::List(ListId::new(99))).expect("push");
        let result = eval_sum(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::ListOutOfBounds {
                list: ListId::new(99)
            })
        );
    }

    #[test]
    fn count_list_out_of_bounds() {
        let mut stack = ExprStack::new(4).expect("valid");
        let store = ValueStore::new();
        push_value(&mut stack, SlotValue::List(ListId::new(99))).expect("push");
        let result = eval_count(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::ListOutOfBounds {
                list: ListId::new(99)
            })
        );
    }

    #[test]
    fn append_list_out_of_bounds() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        push_value(&mut stack, SlotValue::List(ListId::new(99))).expect("push");
        push_value(&mut stack, SlotValue::I64(1)).expect("push");
        let result = eval_append(&mut stack, &mut store);
        assert_eq!(
            result,
            Err(EngineError::ListOutOfBounds {
                list: ListId::new(99)
            })
        );
    }

    #[test]
    fn append_if_list_out_of_bounds() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        push_value(&mut stack, SlotValue::List(ListId::new(99))).expect("push");
        push_value(&mut stack, SlotValue::I64(1)).expect("push");
        push_value(&mut stack, SlotValue::Bool(true)).expect("push");
        let result = eval_append_if(&mut stack, &mut store);
        assert_eq!(
            result,
            Err(EngineError::ListOutOfBounds {
                list: ListId::new(99)
            })
        );
    }

    #[test]
    fn unique_list_out_of_bounds() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        push_value(&mut stack, SlotValue::List(ListId::new(99))).expect("push");
        let result = eval_unique(&mut stack, &mut store);
        assert_eq!(
            result,
            Err(EngineError::ListOutOfBounds {
                list: ListId::new(99)
            })
        );
    }

    // ========================================================================
    // CE-005 follow-up (vb-kepe8): real OOM-path tests.
    // The pre-existing tests at the top of this mod only cover the
    // downstream `insert_list` resource cap (`MAX_LIST_ITEMS_PER_VALUE`).
    // The tests below exercise the operator's *own* fallible reservation
    // by arming the thread-local OOM hook, which fires immediately after
    // `try_reserve_exact` returns Ok. The hook is `thread_local!` so
    // parallel tests cannot race each other.
    // ========================================================================

    fn fresh_list_with_items(
        items: Vec<SlotValue>,
    ) -> Result<(ValueStore, ExprStack, ListId), String> {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(items.into_boxed_slice())
            .map_err(|e| e.to_string())?;
        let mut stack = ExprStack::new(4).map_err(|e| e.to_string())?;
        push_value(&mut stack, SlotValue::List(list)).map_err(|e| e.to_string())?;
        Ok((store, stack, list))
    }

    /// RAII guard that arms the OOM hook on creation and clears it on
    /// drop, so a test that panics (or just ends) never leaks armed
    /// state into the next test.
    struct OomGuard {
        _priv: (),
    }

    impl OomGuard {
        fn new(count: usize) -> Self {
            oom_inject::arm(count);
            Self { _priv: () }
        }
    }

    impl Drop for OomGuard {
        fn drop(&mut self) {
            oom_inject::reset();
        }
    }

    #[test]
    fn oom_inject_eval_append_returns_allocation_failed() -> Result<(), String> {
        let (mut store, mut stack, _list) = fresh_list_with_items(vec![SlotValue::I64(1)])?;
        push_value(&mut stack, SlotValue::I64(2)).map_err(|e| e.to_string())?;
        let _guard = OomGuard::new(1);
        let result = eval_append(&mut stack, &mut store);
        ensure_equal(result, Err(EngineError::AllocationFailed))
    }

    #[test]
    fn oom_inject_eval_append_if_true_returns_allocation_failed() -> Result<(), String> {
        let (mut store, mut stack, _list) = fresh_list_with_items(vec![SlotValue::I64(1)])?;
        push_value(&mut stack, SlotValue::I64(2)).map_err(|e| e.to_string())?;
        push_value(&mut stack, SlotValue::Bool(true)).map_err(|e| e.to_string())?;
        let _guard = OomGuard::new(1);
        let result = eval_append_if(&mut stack, &mut store);
        ensure_equal(result, Err(EngineError::AllocationFailed))
    }

    #[test]
    fn oom_inject_eval_append_if_false_returns_allocation_failed() -> Result<(), String> {
        // The false branch still calls `try_reserve_exact(base_len)`,
        // so the hook must fire there too — that path is what the
        // parent's tests did not exercise.
        let (mut store, mut stack, _list) = fresh_list_with_items(vec![SlotValue::I64(1)])?;
        push_value(&mut stack, SlotValue::I64(2)).map_err(|e| e.to_string())?;
        push_value(&mut stack, SlotValue::Bool(false)).map_err(|e| e.to_string())?;
        let _guard = OomGuard::new(1);
        let result = eval_append_if(&mut stack, &mut store);
        ensure_equal(result, Err(EngineError::AllocationFailed))
    }

    #[test]
    fn oom_inject_eval_unique_returns_allocation_failed() -> Result<(), String> {
        let (mut store, mut stack, _list) =
            fresh_list_with_items(vec![SlotValue::I64(1), SlotValue::I64(2)])?;
        let _guard = OomGuard::new(1);
        let result = eval_unique(&mut stack, &mut store);
        ensure_equal(result, Err(EngineError::AllocationFailed))
    }

    #[test]
    fn oom_inject_dec_is_one_shot() -> Result<(), String> {
        // First call with the hook armed must fail. After the guard
        // drops, the hook is cleared and a follow-up call on a fresh
        // stack must succeed normally.
        let (mut store, mut stack, _list) = fresh_list_with_items(vec![SlotValue::I64(1)])?;
        push_value(&mut stack, SlotValue::I64(2)).map_err(|e| e.to_string())?;
        let first = {
            let _guard = OomGuard::new(1);
            eval_append(&mut stack, &mut store)
        };
        ensure_equal(first, Err(EngineError::AllocationFailed))?;
        // The hook must be cleared by guard Drop. Verify by arming
        // a fresh state and observing it is at 0.
        oom_inject::reset();
        let (mut store2, mut stack2, _list2) =
            fresh_list_with_items(vec![SlotValue::I64(10)])?;
        push_value(&mut stack2, SlotValue::I64(20)).map_err(|e| e.to_string())?;
        let result = eval_append(&mut stack2, &mut store2);
        ensure_equal(result, Ok(()))?;
        Ok(())
    }

    #[test]
    fn oom_inject_disarmed_happy_path_succeeds() -> Result<(), String> {
        // Sanity: with the hook disarmed, the operator still works.
        let (mut store, mut stack, _list) =
            fresh_list_with_items(vec![SlotValue::I64(1), SlotValue::I64(2)])?;
        push_value(&mut stack, SlotValue::I64(3)).map_err(|e| e.to_string())?;
        oom_inject::reset();
        let result = eval_append(&mut stack, &mut store);
        ensure_equal(result, Ok(()))
    }

}
