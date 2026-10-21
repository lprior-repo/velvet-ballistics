#![forbid(unsafe_code)]
//! Store-aware and end-to-end integration tests for the expression evaluator.
//!
//! Split from `integration.rs` so every file in `expr_eval/tests/` stays
//! under the 1500-line `test_in_src` cap. This half owns all tests that
//! exercise `ValueStore`-aware helpers (`Empty`, `Unique`, `Length`, `Sum`,
//! `Count`, `Contains`, `StartsWith`, `EndsWith`, `Has`, `Append`,
//! `AppendIf`, `Merge`, `Exists`) plus the lex -> parse -> compile -> eval
//! pipeline cases that close out the integration suite.

use vb_core::value_store::ValueStore;
use vb_core::{ConstIdx, ConstValue, ExprOp, ExprProgram, SlotIdx, SlotValue};
use vb_core::value::Taint;

use crate::bytecode;
use crate::lexer::lex_expr;
use crate::parser::parse_expr;
use crate::{ExprError, ExprResult};

use crate::eval::{
    eval_expr_program, eval_expr_program_with_store, eval_helper_with_store, ExprHelper,
};

// ===== Store-aware helper tests =====

#[test]
fn eval_helper_with_store_empty_returns_true_for_null() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let args = [SlotValue::Null];
    let result = eval_helper_with_store(ExprHelper::Empty, &args, &mut store)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_helper_with_store_empty_returns_true_for_empty_list() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![].into_boxed_slice())
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::List(list)];
    let result = eval_helper_with_store(ExprHelper::Empty, &args, &mut store)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_helper_with_store_empty_returns_false_for_nonempty_list() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::List(list)];
    let result = eval_helper_with_store(ExprHelper::Empty, &args, &mut store)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn eval_helper_with_store_empty_returns_true_for_empty_symbol() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let sym = store
        .insert_symbol(Box::<str>::from(""))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::Symbol(sym)];
    let result = eval_helper_with_store(ExprHelper::Empty, &args, &mut store)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_helper_with_store_empty_returns_false_for_nonempty_symbol() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let sym = store
        .insert_symbol(Box::<str>::from("hello"))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::Symbol(sym)];
    let result = eval_helper_with_store(ExprHelper::Empty, &args, &mut store)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn eval_helper_with_store_empty_returns_true_for_empty_object() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let obj = store
        .insert_object(vec![].into_boxed_slice())
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::Object(obj)];
    let result = eval_helper_with_store(ExprHelper::Empty, &args, &mut store)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_helper_with_store_empty_returns_type_mismatch_for_i64() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let args = [SlotValue::I64(42)];
    let result = eval_helper_with_store(ExprHelper::Empty, &args, &mut store);
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for empty(42) with store".into(),
        });
    };
    assert_eq!(expected, "text, list, object, or null");
    assert_eq!(found, "number");
    Ok(())
}

#[test]
fn eval_helper_with_store_unique_deduplicates_list() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(1)]
                .into_boxed_slice(),
        )
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::List(list)];
    let result = eval_helper_with_store(ExprHelper::Unique, &args, &mut store)?;
    let SlotValue::List(unique_id) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected List from unique".into(),
        });
    };
    let items = store.list(unique_id).map_err(|_| ExprError::UnexpectedEof)?;
    assert_eq!(items.len(), 2);
    assert_eq!(items[0], SlotValue::I64(1));
    assert_eq!(items[1], SlotValue::I64(2));
    Ok(())
}

#[test]
fn eval_helper_with_store_unique_preserves_order() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(
            vec![
                SlotValue::I64(3),
                SlotValue::I64(1),
                SlotValue::I64(3),
                SlotValue::I64(2),
                SlotValue::I64(1),
            ]
            .into_boxed_slice(),
        )
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::List(list)];
    let result = eval_helper_with_store(ExprHelper::Unique, &args, &mut store)?;
    let SlotValue::List(unique_id) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected List from unique".into(),
        });
    };
    let items = store.list(unique_id).map_err(|_| ExprError::UnexpectedEof)?;
    assert_eq!(items.len(), 3);
    assert_eq!(items[0], SlotValue::I64(3));
    assert_eq!(items[1], SlotValue::I64(1));
    assert_eq!(items[2], SlotValue::I64(2));
    Ok(())
}

#[test]
fn eval_helper_with_store_unique_returns_empty_list_for_empty_input() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![].into_boxed_slice())
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::List(list)];
    let result = eval_helper_with_store(ExprHelper::Unique, &args, &mut store)?;
    let SlotValue::List(unique_id) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected List from unique".into(),
        });
    };
    let items = store.list(unique_id).map_err(|_| ExprError::UnexpectedEof)?;
    assert!(items.is_empty());
    Ok(())
}

#[test]
fn eval_helper_with_store_unique_rejects_non_list() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let args = [SlotValue::I64(42)];
    let result = eval_helper_with_store(ExprHelper::Unique, &args, &mut store);
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for unique(42) with store".into(),
        });
    };
    assert_eq!(expected, "list");
    assert_eq!(found, "number");
    Ok(())
}

#[test]
fn eval_helper_with_store_length_returns_list_length() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)].into_boxed_slice())
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::List(list)];
    let result = eval_helper_with_store(ExprHelper::Length, &args, &mut store)?;
    assert_eq!(result, SlotValue::I64(3));
    Ok(())
}

#[test]
fn eval_helper_with_store_length_returns_symbol_length() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let sym = store
        .insert_symbol(Box::<str>::from("hello"))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::Symbol(sym)];
    let result = eval_helper_with_store(ExprHelper::Length, &args, &mut store)?;
    assert_eq!(result, SlotValue::I64(5));
    Ok(())
}

#[test]
fn eval_helper_with_store_length_returns_object_field_count() -> ExprResult<()> {
    use vb_core::value_store::ObjectField;
    let mut store = ValueStore::new();
    let obj = store
        .insert_object(
            vec![
                ObjectField { key: vb_core::ids::SymbolId::new(0), value: SlotValue::I64(1), taint: Taint::Clean },
                ObjectField { key: vb_core::ids::SymbolId::new(1), value: SlotValue::I64(2), taint: Taint::Clean },
            ]
            .into_boxed_slice(),
        )
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::Object(obj)];
    let result = eval_helper_with_store(ExprHelper::Length, &args, &mut store)?;
    assert_eq!(result, SlotValue::I64(2));
    Ok(())
}

#[test]
fn eval_helper_with_store_sum_sums_list_elements() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)].into_boxed_slice())
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::List(list)];
    let result = eval_helper_with_store(ExprHelper::Sum, &args, &mut store)?;
    assert_eq!(result, SlotValue::I64(60));
    Ok(())
}

#[test]
fn eval_helper_with_store_count_returns_list_length() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice())
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::List(list)];
    let result = eval_helper_with_store(ExprHelper::Count, &args, &mut store)?;
    assert_eq!(result, SlotValue::I64(2));
    Ok(())
}

#[test]
fn eval_helper_with_store_contains_checks_substring() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let haystack = store
        .insert_symbol(Box::<str>::from("hello world"))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let needle = store
        .insert_symbol(Box::<str>::from("world"))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::Symbol(haystack), SlotValue::Symbol(needle)];
    let result = eval_helper_with_store(ExprHelper::Contains, &args, &mut store)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_helper_with_store_contains_returns_false_for_absent_substring() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let haystack = store
        .insert_symbol(Box::<str>::from("hello world"))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let needle = store
        .insert_symbol(Box::<str>::from("xyz"))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::Symbol(haystack), SlotValue::Symbol(needle)];
    let result = eval_helper_with_store(ExprHelper::Contains, &args, &mut store)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn eval_helper_with_store_starts_with_checks_prefix() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let text = store
        .insert_symbol(Box::<str>::from("hello world"))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let prefix = store
        .insert_symbol(Box::<str>::from("hello"))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::Symbol(text), SlotValue::Symbol(prefix)];
    let result = eval_helper_with_store(ExprHelper::StartsWith, &args, &mut store)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_helper_with_store_ends_with_checks_suffix() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let text = store
        .insert_symbol(Box::<str>::from("hello world"))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let suffix = store
        .insert_symbol(Box::<str>::from("world"))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::Symbol(text), SlotValue::Symbol(suffix)];
    let result = eval_helper_with_store(ExprHelper::EndsWith, &args, &mut store)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_helper_with_store_has_checks_object_key() -> ExprResult<()> {
    use vb_core::value_store::ObjectField;
    let mut store = ValueStore::new();
    let key = vb_core::ids::SymbolId::new(42);
    let obj = store
        .insert_object(
            vec![ObjectField { key, value: SlotValue::I64(100) }].into_boxed_slice(),
        )
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::Object(obj), SlotValue::Symbol(key)];
    let result = eval_helper_with_store(ExprHelper::Has, &args, &mut store)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_helper_with_store_has_returns_false_for_missing_key() -> ExprResult<()> {
    use vb_core::value_store::ObjectField;
    let mut store = ValueStore::new();
    let key_present = vb_core::ids::SymbolId::new(1);
    let key_absent = vb_core::ids::SymbolId::new(99);
    let obj = store
        .insert_object(
            vec![ObjectField { key: key_present, value: SlotValue::I64(1), taint: Taint::Clean }].into_boxed_slice(),
        )
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::Object(obj), SlotValue::Symbol(key_absent)];
    let result = eval_helper_with_store(ExprHelper::Has, &args, &mut store)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn eval_helper_with_store_append_adds_item_to_list() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::List(list), SlotValue::I64(2)];
    let result = eval_helper_with_store(ExprHelper::Append, &args, &mut store)?;
    let SlotValue::List(new_list_id) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected List from append".into(),
        });
    };
    let items = store.list(new_list_id).map_err(|_| ExprError::UnexpectedEof)?;
    assert_eq!(items.len(), 2);
    assert_eq!(items[0], SlotValue::I64(1));
    assert_eq!(items[1], SlotValue::I64(2));
    Ok(())
}

#[test]
fn eval_helper_with_store_append_if_adds_item_when_true() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::List(list), SlotValue::I64(2), SlotValue::Bool(true)];
    let result = eval_helper_with_store(ExprHelper::AppendIf, &args, &mut store)?;
    let SlotValue::List(new_list_id) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected List from append_if".into(),
        });
    };
    let items = store.list(new_list_id).map_err(|_| ExprError::UnexpectedEof)?;
    assert_eq!(items.len(), 2);
    Ok(())
}

#[test]
fn eval_helper_with_store_append_if_skips_item_when_false() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::List(list), SlotValue::I64(2), SlotValue::Bool(false)];
    let result = eval_helper_with_store(ExprHelper::AppendIf, &args, &mut store)?;
    let SlotValue::List(new_list_id) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected List from append_if".into(),
        });
    };
    let items = store.list(new_list_id).map_err(|_| ExprError::UnexpectedEof)?;
    assert_eq!(items.len(), 1);
    Ok(())
}

#[test]
fn eval_helper_with_store_merge_combines_objects() -> ExprResult<()> {
    use vb_core::value_store::ObjectField;
    let mut store = ValueStore::new();
    let key_a = vb_core::ids::SymbolId::new(1);
    let key_b = vb_core::ids::SymbolId::new(2);
    let left = store
        .insert_object(
            vec![ObjectField { key: key_a, value: SlotValue::I64(10), taint: Taint::Clean }].into_boxed_slice(),
        )
        .map_err(|_| ExprError::UnexpectedEof)?;
    let right = store
        .insert_object(
            vec![ObjectField { key: key_b, value: SlotValue::I64(20), taint: Taint::Clean }].into_boxed_slice(),
        )
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::Object(left), SlotValue::Object(right)];
    let result = eval_helper_with_store(ExprHelper::Merge, &args, &mut store)?;
    let SlotValue::Object(merged_id) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected Object from merge".into(),
        });
    };
    let fields = store.object(merged_id).map_err(|_| ExprError::UnexpectedEof)?;
    assert_eq!(fields.len(), 2);
    Ok(())
}

#[test]
fn eval_expr_program_with_store_empty_list_returns_true() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![].into_boxed_slice())
        .map_err(|_| ExprError::UnexpectedEof)?;
    let program = ExprProgram {
        ops: vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Empty].into_boxed_slice(),
        max_stack: 1,
    };
    let slots = vec![Some(SlotValue::List(list))];
    let result = eval_expr_program_with_store(&program, &slots, &[], &mut store)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_expr_program_with_store_unique_deduplicates() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(
            vec![SlotValue::I64(1), SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice(),
        )
        .map_err(|_| ExprError::UnexpectedEof)?;
    let program = ExprProgram {
        ops: vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Unique].into_boxed_slice(),
        max_stack: 1,
    };
    let slots = vec![Some(SlotValue::List(list))];
    let result = eval_expr_program_with_store(&program, &slots, &[], &mut store)?;
    let SlotValue::List(unique_id) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected List from unique".into(),
        });
    };
    let items = store.list(unique_id).map_err(|_| ExprError::UnexpectedEof)?;
    assert_eq!(items.len(), 2);
    Ok(())
}

#[test]
fn eval_expr_program_with_store_length_returns_correct_count() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)].into_boxed_slice())
        .map_err(|_| ExprError::UnexpectedEof)?;
    let program = ExprProgram {
        ops: vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Length].into_boxed_slice(),
        max_stack: 1,
    };
    let slots = vec![Some(SlotValue::List(list))];
    let result = eval_expr_program_with_store(&program, &slots, &[], &mut store)?;
    assert_eq!(result, SlotValue::I64(3));
    Ok(())
}

#[test]
fn eval_expr_program_with_store_sum_computes_total() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)].into_boxed_slice())
        .map_err(|_| ExprError::UnexpectedEof)?;
    let program = ExprProgram {
        ops: vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Sum].into_boxed_slice(),
        max_stack: 1,
    };
    let slots = vec![Some(SlotValue::List(list))];
    let result = eval_expr_program_with_store(&program, &slots, &[], &mut store)?;
    assert_eq!(result, SlotValue::I64(60));
    Ok(())
}

#[test]
fn eval_helper_with_store_exists_returns_false_for_null() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let args = [SlotValue::Null];
    let result = eval_helper_with_store(ExprHelper::Exists, &args, &mut store)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn eval_helper_with_store_exists_returns_true_for_non_null() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let args = [SlotValue::I64(1)];
    let result = eval_helper_with_store(ExprHelper::Exists, &args, &mut store)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_helper_with_store_sum_returns_integer_overflow_on_overflow() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![SlotValue::I64(i64::MAX), SlotValue::I64(1)].into_boxed_slice())
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::List(list)];
    let result = eval_helper_with_store(ExprHelper::Sum, &args, &mut store);
    let Err(ExprError::IntegerOverflow) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected IntegerOverflow for sum overflow".into(),
        });
    };
    Ok(())
}

// ===== Store-aware: length on empty objects and edge cases =====

#[test]
fn eval_helper_with_store_length_returns_zero_for_empty_list() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![].into_boxed_slice())
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::List(list)];
    let result = eval_helper_with_store(ExprHelper::Length, &args, &mut store)?;
    assert_eq!(result, SlotValue::I64(0));
    Ok(())
}

#[test]
fn eval_helper_with_store_length_returns_zero_for_empty_symbol() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let sym = store
        .insert_symbol(Box::<str>::from(""))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::Symbol(sym)];
    let result = eval_helper_with_store(ExprHelper::Length, &args, &mut store)?;
    assert_eq!(result, SlotValue::I64(0));
    Ok(())
}

#[test]
fn eval_helper_with_store_length_returns_zero_for_empty_object() -> ExprResult<()> {
    use vb_core::value_store::ObjectField;
    let mut store = ValueStore::new();
    let obj = store
        .insert_object(vec![].into_boxed_slice())
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::Object(obj)];
    let result = eval_helper_with_store(ExprHelper::Length, &args, &mut store)?;
    assert_eq!(result, SlotValue::I64(0));
    Ok(())
}

#[test]
fn eval_helper_with_store_length_rejects_i64() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let args = [SlotValue::I64(1)];
    let result = eval_helper_with_store(ExprHelper::Length, &args, &mut store);
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for length on i64".into(),
        });
    };
    assert_eq!(expected, "text, list, or object");
    assert_eq!(found, "number");
    Ok(())
}

// ===== Store-aware: count tests =====

#[test]
fn eval_helper_with_store_count_returns_zero_for_empty_list() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![].into_boxed_slice())
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::List(list)];
    let result = eval_helper_with_store(ExprHelper::Count, &args, &mut store)?;
    assert_eq!(result, SlotValue::I64(0));
    Ok(())
}

#[test]
fn eval_helper_with_store_count_rejects_non_list() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let args = [SlotValue::I64(1)];
    let result = eval_helper_with_store(ExprHelper::Count, &args, &mut store);
    assert!(matches!(result, Err(ExprError::TypeMismatch { .. })));
    Ok(())
}

// ===== Store-aware: contains edge cases =====

#[test]
fn eval_helper_with_store_contains_returns_true_for_empty_needle() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let haystack = store
        .insert_symbol(Box::<str>::from("hello"))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let needle = store
        .insert_symbol(Box::<str>::from(""))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::Symbol(haystack), SlotValue::Symbol(needle)];
    let result = eval_helper_with_store(ExprHelper::Contains, &args, &mut store)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_helper_with_store_contains_returns_false_for_empty_haystack() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let haystack = store
        .insert_symbol(Box::<str>::from(""))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let needle = store
        .insert_symbol(Box::<str>::from("x"))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::Symbol(haystack), SlotValue::Symbol(needle)];
    let result = eval_helper_with_store(ExprHelper::Contains, &args, &mut store)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

// ===== Store-aware: starts_with / ends_with edge cases =====

#[test]
fn eval_helper_with_store_starts_with_returns_false_for_non_prefix() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let text = store
        .insert_symbol(Box::<str>::from("hello world"))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let prefix = store
        .insert_symbol(Box::<str>::from("xyz"))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::Symbol(text), SlotValue::Symbol(prefix)];
    let result = eval_helper_with_store(ExprHelper::StartsWith, &args, &mut store)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn eval_helper_with_store_ends_with_returns_false_for_non_suffix() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let text = store
        .insert_symbol(Box::<str>::from("hello world"))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let suffix = store
        .insert_symbol(Box::<str>::from("xyz"))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::Symbol(text), SlotValue::Symbol(suffix)];
    let result = eval_helper_with_store(ExprHelper::EndsWith, &args, &mut store)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn eval_helper_with_store_starts_with_returns_true_for_empty_prefix() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let text = store
        .insert_symbol(Box::<str>::from("hello"))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let prefix = store
        .insert_symbol(Box::<str>::from(""))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::Symbol(text), SlotValue::Symbol(prefix)];
    let result = eval_helper_with_store(ExprHelper::StartsWith, &args, &mut store)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_helper_with_store_ends_with_returns_true_for_empty_suffix() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let text = store
        .insert_symbol(Box::<str>::from("hello"))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let suffix = store
        .insert_symbol(Box::<str>::from(""))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::Symbol(text), SlotValue::Symbol(suffix)];
    let result = eval_helper_with_store(ExprHelper::EndsWith, &args, &mut store)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

// ===== Store-aware: merge overlapping keys =====

#[test]
fn eval_helper_with_store_merge_overwrites_overlapping_keys() -> ExprResult<()> {
    use vb_core::value_store::ObjectField;
    let mut store = ValueStore::new();
    let key = vb_core::ids::SymbolId::new(1);
    let left = store
        .insert_object(
            vec![ObjectField {
                key,
                value: SlotValue::I64(10),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|_| ExprError::UnexpectedEof)?;
    let right = store
        .insert_object(
            vec![ObjectField {
                key,
                value: SlotValue::I64(99),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::Object(left), SlotValue::Object(right)];
    let result = eval_helper_with_store(ExprHelper::Merge, &args, &mut store)?;
    let SlotValue::Object(merged_id) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected Object from merge".into(),
        });
    };
    let fields = store
        .object(merged_id)
        .map_err(|_| ExprError::UnexpectedEof)?;
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].value, SlotValue::I64(99));
    Ok(())
}

// ===== Store-aware: unique variations =====

#[test]
fn eval_helper_with_store_unique_all_same_returns_one_element() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(
            vec![SlotValue::I64(7), SlotValue::I64(7), SlotValue::I64(7)].into_boxed_slice(),
        )
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::List(list)];
    let result = eval_helper_with_store(ExprHelper::Unique, &args, &mut store)?;
    let SlotValue::List(unique_id) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected List from unique".into(),
        });
    };
    let items = store
        .list(unique_id)
        .map_err(|_| ExprError::UnexpectedEof)?;
    assert_eq!(items.len(), 1);
    assert_eq!(items[0], SlotValue::I64(7));
    Ok(())
}

#[test]
fn eval_helper_with_store_unique_already_unique_returns_same_count() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)].into_boxed_slice(),
        )
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::List(list)];
    let result = eval_helper_with_store(ExprHelper::Unique, &args, &mut store)?;
    let SlotValue::List(unique_id) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected List from unique".into(),
        });
    };
    let items = store
        .list(unique_id)
        .map_err(|_| ExprError::UnexpectedEof)?;
    assert_eq!(items.len(), 3);
    Ok(())
}

// ===== Store-aware: sum variations =====

#[test]
fn eval_helper_with_store_sum_empty_list_returns_zero() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![].into_boxed_slice())
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::List(list)];
    let result = eval_helper_with_store(ExprHelper::Sum, &args, &mut store)?;
    assert_eq!(result, SlotValue::I64(0));
    Ok(())
}

#[test]
fn eval_helper_with_store_sum_single_element_returns_element() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![SlotValue::I64(42)].into_boxed_slice())
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::List(list)];
    let result = eval_helper_with_store(ExprHelper::Sum, &args, &mut store)?;
    assert_eq!(result, SlotValue::I64(42));
    Ok(())
}

#[test]
fn eval_helper_with_store_sum_rejects_non_i64_element() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(
            vec![SlotValue::I64(1), SlotValue::Bool(true)].into_boxed_slice(),
        )
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::List(list)];
    let result = eval_helper_with_store(ExprHelper::Sum, &args, &mut store);
    assert!(matches!(result, Err(ExprError::TypeMismatch { .. })));
    Ok(())
}

#[test]
fn eval_helper_with_store_sum_rejects_non_list() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let args = [SlotValue::I64(1)];
    let result = eval_helper_with_store(ExprHelper::Sum, &args, &mut store);
    assert!(matches!(result, Err(ExprError::TypeMismatch { .. })));
    Ok(())
}

// ===== Store-aware: empty on non-empty objects =====

#[test]
fn eval_helper_with_store_empty_returns_false_for_nonempty_object() -> ExprResult<()> {
    use vb_core::value_store::ObjectField;
    let mut store = ValueStore::new();
    let obj = store
        .insert_object(
            vec![ObjectField {
                key: vb_core::ids::SymbolId::new(0),
                value: SlotValue::I64(1),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::Object(obj)];
    let result = eval_helper_with_store(ExprHelper::Empty, &args, &mut store)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

// ===== Store-aware: exists with store =====

#[test]
fn eval_helper_with_store_exists_returns_true_for_symbol() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let sym = store
        .insert_symbol(Box::<str>::from("hello"))
        .map_err(|_| ExprError::UnexpectedEof)?;
    let args = [SlotValue::Symbol(sym)];
    let result = eval_helper_with_store(ExprHelper::Exists, &args, &mut store)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

// ===== End-to-end: nested arithmetic with precedence =====

#[test]
fn eval_expr_program_nested_arithmetic_with_precedence() -> ExprResult<()> {
    let tokens = lex_expr("2 + 3 * 4 - 5")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants)?;
    assert_eq!(result, SlotValue::I64(9));
    Ok(())
}

#[test]
fn eval_expr_program_parenthesized_expression() -> ExprResult<()> {
    let tokens = lex_expr("(2 + 3) * 4")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants)?;
    assert_eq!(result, SlotValue::I64(20));
    Ok(())
}

#[test]
fn eval_expr_program_mixed_comparison_and_boolean() -> ExprResult<()> {
    let tokens = lex_expr("3 < 5 and 2 > 1")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_expr_program_negative_in_arithmetic() -> ExprResult<()> {
    let tokens = lex_expr("-5 + 10")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants)?;
    assert_eq!(result, SlotValue::I64(5));
    Ok(())
}

#[test]
fn eval_expr_program_triple_not() -> ExprResult<()> {
    let tokens = lex_expr("not not not true")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}
