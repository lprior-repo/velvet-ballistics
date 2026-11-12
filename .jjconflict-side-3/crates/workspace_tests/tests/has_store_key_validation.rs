use std::error::Error;

use vb_core::ids::{ObjectId, SymbolId};
use vb_core::value_store::ValueStore;
use vb_core::{ObjectField, SlotValue};
use vb_expr::eval::ExprHelper;
use vb_expr::{ExprError, eval_helper, eval_helper_with_store};

fn make_object_with_field(
    store: &mut ValueStore,
    key: &str,
    value: i64,
) -> Result<SlotValue, Box<dyn Error>> {
    let key_id = store.insert_symbol(key)?;
    let fields = vec![ObjectField::clean(key_id, SlotValue::I64(value))];
    let obj_id = store.insert_object(fields)?;
    Ok(SlotValue::Object(obj_id))
}

fn make_empty_object(store: &mut ValueStore) -> Result<SlotValue, Box<dyn Error>> {
    let obj_id = store.insert_object(Vec::<ObjectField>::new())?;
    Ok(SlotValue::Object(obj_id))
}

fn make_symbol(store: &mut ValueStore, text: &str) -> Result<SlotValue, Box<dyn Error>> {
    let sym_id = store.insert_symbol(text)?;
    Ok(SlotValue::Symbol(sym_id))
}

#[test]
fn has_store_true_when_field_present() -> Result<(), Box<dyn Error>> {
    let mut store = ValueStore::new();
    let age_id = store.insert_symbol("age")?;
    let other_id = store.insert_symbol("other")?;
    let fields = vec![
        ObjectField::clean(age_id, SlotValue::I64(1)),
        ObjectField::clean(other_id, SlotValue::I64(2)),
    ];
    let obj_id = store.insert_object(fields)?;
    let obj = SlotValue::Object(obj_id);
    let key = SlotValue::Symbol(age_id);

    let result = eval_helper_with_store(ExprHelper::Has, &[obj, key], &mut store)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn has_store_false_when_field_absent() -> Result<(), Box<dyn Error>> {
    let mut store = ValueStore::new();
    let age_id = store.insert_symbol("age")?;
    let other_id = store.insert_symbol("other")?;
    let name_id = store.insert_symbol("name")?;
    let fields = vec![
        ObjectField::clean(age_id, SlotValue::I64(1)),
        ObjectField::clean(other_id, SlotValue::I64(2)),
    ];
    let obj_id = store.insert_object(fields)?;
    let obj = SlotValue::Object(obj_id);
    let key = SlotValue::Symbol(name_id);

    let result = eval_helper_with_store(ExprHelper::Has, &[obj, key], &mut store)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn has_store_false_on_empty_object() -> Result<(), Box<dyn Error>> {
    let mut store = ValueStore::new();
    let obj = make_empty_object(&mut store)?;
    let key = make_symbol(&mut store, "anything")?;

    let result = eval_helper_with_store(ExprHelper::Has, &[obj, key], &mut store)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn has_store_false_on_other_field() -> Result<(), Box<dyn Error>> {
    let mut store = ValueStore::new();
    let obj = make_object_with_field(&mut store, "x", 1)?;
    let key = make_symbol(&mut store, "y")?;

    let result = eval_helper_with_store(ExprHelper::Has, &[obj, key], &mut store)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn has_store_invalid_symbol_empty_object() -> Result<(), Box<dyn Error>> {
    let mut store = ValueStore::new();
    let obj = make_empty_object(&mut store)?;
    let bad = SymbolId::new(u32::MAX);
    let args = [obj, SlotValue::Symbol(bad)];

    let result = eval_helper_with_store(ExprHelper::Has, &args, &mut store);
    assert_eq!(
        result,
        Err(ExprError::InvalidReference {
            reference: format!("symbol:{bad:?}")
        })
    );
    Ok(())
}

#[test]
fn has_store_invalid_symbol_nonempty_object() -> Result<(), Box<dyn Error>> {
    let mut store = ValueStore::new();
    let obj = make_object_with_field(&mut store, "age", 30)?;
    let bad = SymbolId::new(u32::MAX);
    let args = [obj, SlotValue::Symbol(bad)];

    let result = eval_helper_with_store(ExprHelper::Has, &args, &mut store);
    assert_eq!(
        result,
        Err(ExprError::InvalidReference {
            reference: format!("symbol:{bad:?}")
        })
    );
    Ok(())
}

#[test]
fn has_store_type_mismatch_wrong_receiver() -> Result<(), Box<dyn Error>> {
    let mut store = ValueStore::new();
    let key = make_symbol(&mut store, "age")?;
    let args = [SlotValue::Bool(true), key];

    let result = eval_helper_with_store(ExprHelper::Has, &args, &mut store);
    assert_eq!(
        result,
        Err(ExprError::TypeMismatch {
            expected: "object".to_string(),
            found: "boolean".to_string()
        })
    );
    Ok(())
}

#[test]
fn has_store_type_mismatch_wrong_key_type() -> Result<(), Box<dyn Error>> {
    let mut store = ValueStore::new();
    let obj = make_object_with_field(&mut store, "age", 30)?;
    let args = [obj, SlotValue::I64(42)];

    let result = eval_helper_with_store(ExprHelper::Has, &args, &mut store);
    assert_eq!(
        result,
        Err(ExprError::TypeMismatch {
            expected: "text".to_string(),
            found: "number".to_string()
        })
    );
    Ok(())
}

#[test]
fn has_store_key_type_checked_before_object_lookup() -> Result<(), Box<dyn Error>> {
    let mut store = ValueStore::new();
    let bad_obj = ObjectId::new(u32::MAX);
    let args = [SlotValue::Object(bad_obj), SlotValue::I64(42)];

    let result = eval_helper_with_store(ExprHelper::Has, &args, &mut store);
    assert_eq!(
        result,
        Err(ExprError::TypeMismatch {
            expected: "text".to_string(),
            found: "number".to_string()
        })
    );
    Ok(())
}

#[test]
fn has_store_object_reference_checked_first() -> Result<(), Box<dyn Error>> {
    let mut store = ValueStore::new();
    let bad_obj = ObjectId::new(u32::MAX);
    let bad_sym = SymbolId::new(u32::MAX);
    let args = [SlotValue::Object(bad_obj), SlotValue::Symbol(bad_sym)];

    let result = eval_helper_with_store(ExprHelper::Has, &args, &mut store);
    assert_eq!(
        result,
        Err(ExprError::InvalidReference {
            reference: format!("object:{bad_obj:?}")
        })
    );
    Ok(())
}

#[test]
fn has_store_helper_arity_zero_args() -> Result<(), Box<dyn Error>> {
    let mut store = ValueStore::new();
    let result = eval_helper_with_store(ExprHelper::Has, &[], &mut store);
    assert_eq!(
        result,
        Err(ExprError::HelperArityMismatch {
            helper: "has".to_string(),
            expected: 2,
            actual: 0
        })
    );
    Ok(())
}

#[test]
fn has_store_helper_arity_one_arg() -> Result<(), Box<dyn Error>> {
    let mut store = ValueStore::new();
    let obj = make_empty_object(&mut store)?;
    let result = eval_helper_with_store(ExprHelper::Has, &[obj], &mut store);
    assert_eq!(
        result,
        Err(ExprError::HelperArityMismatch {
            helper: "has".to_string(),
            expected: 2,
            actual: 1
        })
    );
    Ok(())
}

#[test]
fn has_store_helper_arity_three_args() -> Result<(), Box<dyn Error>> {
    let mut store = ValueStore::new();
    let obj = make_empty_object(&mut store)?;
    let key = make_symbol(&mut store, "x")?;
    let args = [obj, key, SlotValue::Null];
    let result = eval_helper_with_store(ExprHelper::Has, &args, &mut store);
    assert_eq!(
        result,
        Err(ExprError::HelperArityMismatch {
            helper: "has".to_string(),
            expected: 2,
            actual: 3
        })
    );
    Ok(())
}

#[test]
fn has_context_free_helper_arity_zero_args() -> Result<(), Box<dyn Error>> {
    let result = eval_helper(ExprHelper::Has, &[]);
    assert_eq!(
        result,
        Err(ExprError::HelperArityMismatch {
            helper: "has".to_string(),
            expected: 2,
            actual: 0
        })
    );
    Ok(())
}
