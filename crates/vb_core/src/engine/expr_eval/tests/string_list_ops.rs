//! String and list expression-operation tests.

#![forbid(unsafe_code)]

use super::{eval_expr_ops_with_constants, eval_expr_ops_with_store};
use crate::errors::EngineError;
use crate::ids::{ConstIdx, SlotIdx};
use crate::value::{ConstValue, SlotValue, Taint};
use crate::value_store::{ObjectField, ValueStore};
use crate::workflow::ExprOp;

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
        vec![ConstValue::Symbol(haystack), ConstValue::Symbol(needle)],
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
        vec![ConstValue::Symbol(haystack), ConstValue::Symbol(needle)],
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
        vec![ConstValue::Symbol(text), ConstValue::Symbol(prefix)],
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
        vec![ConstValue::Symbol(text), ConstValue::Symbol(suffix)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
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
        vec![ConstValue::I64(20)],
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
            taint: Taint::Clean,
        }]
        .into_boxed_slice(),
    )?;

    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Exists];
    let result = eval_expr_ops_with_store(&ops, vec![SlotValue::Object(obj)], vec![], &mut store)?;
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
    let result = eval_expr_ops_with_store(&ops, vec![SlotValue::List(list)], vec![], &mut store)?;
    assert_eq!(result, SlotValue::I64(3));
    Ok(())
}

#[test]
fn empty_detects_empty_list() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let list = store.insert_list(vec![].into_boxed_slice())?;

    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Empty];
    let result = eval_expr_ops_with_store(&ops, vec![SlotValue::List(list)], vec![], &mut store)?;
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
        vec![ConstValue::I64(2)],
        &mut store,
    )?;
    let result_list_id = match result {
        SlotValue::List(id) => id,
        other => {
            return Err(EngineError::TypeMismatch {
                expected: "list",
                found: other.type_name(),
            });
        }
    };
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
        vec![ConstValue::I64(2), ConstValue::Bool(true)],
        &mut store,
    )?;
    let result_list_id = match result {
        SlotValue::List(id) => id,
        other => {
            return Err(EngineError::TypeMismatch {
                expected: "list",
                found: other.type_name(),
            });
        }
    };
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
            taint: Taint::Clean,
        }]
        .into_boxed_slice(),
    )?;
    let obj2 = store.insert_object(
        vec![ObjectField {
            key: sym2,
            value: SlotValue::I64(2),
            taint: Taint::Clean,
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
    let result_obj_id = match result {
        SlotValue::Object(id) => id,
        other => {
            return Err(EngineError::TypeMismatch {
                expected: "object",
                found: other.type_name(),
            });
        }
    };
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
    let result = eval_expr_ops_with_store(&ops, vec![SlotValue::List(list)], vec![], &mut store)?;
    assert_eq!(result, SlotValue::I64(6));
    Ok(())
}

#[test]
fn count_computes_list_length() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let list = store.insert_list(vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice())?;

    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Count];
    let result = eval_expr_ops_with_store(&ops, vec![SlotValue::List(list)], vec![], &mut store)?;
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
    let result = eval_expr_ops_with_store(&ops, vec![SlotValue::List(list)], vec![], &mut store)?;
    let result_list_id = match result {
        SlotValue::List(id) => id,
        other => {
            return Err(EngineError::TypeMismatch {
                expected: "list",
                found: other.type_name(),
            });
        }
    };
    let items = store.list(result_list_id)?;
    assert_eq!(items.len(), 2);
    Ok(())
}
