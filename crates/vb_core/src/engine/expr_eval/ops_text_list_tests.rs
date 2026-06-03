#![forbid(unsafe_code)]
//! Tests for text and list operations in expression evaluation.

use crate::errors::EngineError;
use crate::ids::{
    ConstIdx, ExprIdx, ListId, ObjectId, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest,
};
use crate::limits::MAX_EXPRESSION_STACK;
use crate::value::{ConstValue, SlotValue, Taint};
use crate::value_store::ValueStore;
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram, ResourceContract,
    WorkflowParts, check_expr_stack_bound,
};

use crate::engine::expr_eval::ops_text_list::{eval_append, eval_append_if, eval_contains,
    eval_empty, eval_ends_with, eval_has, eval_length, eval_starts_with, eval_sum,
    eval_count, eval_unique};
use crate::engine::expr_eval::stack::{push_value, ExprStack};

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
