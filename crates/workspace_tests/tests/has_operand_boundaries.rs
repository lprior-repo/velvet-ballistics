use std::error::Error;

use vb_core::SlotValue;
use vb_core::ids::{ObjectId, SymbolId};
use vb_core::value_store::ValueStore;
use vb_expr::eval::ExprHelper;
use vb_expr::{ExprError, eval_helper, eval_helper_with_store};

#[test]
fn has_checks_receiver_before_wrong_key_type() -> Result<(), Box<dyn Error>> {
    let mut store = ValueStore::new();
    let args = [SlotValue::Bool(true), SlotValue::I64(42)];

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
fn has_rejects_invalid_object_with_valid_stored_symbol() -> Result<(), Box<dyn Error>> {
    let mut store = ValueStore::new();
    let key = store.insert_symbol("key")?;
    let bad_object = ObjectId::new(u32::MAX);
    let args = [SlotValue::Object(bad_object), SlotValue::Symbol(key)];

    let result = eval_helper_with_store(ExprHelper::Has, &args, &mut store);
    assert_eq!(
        result,
        Err(ExprError::InvalidReference {
            reference: format!("object:{bad_object:?}")
        })
    );
    Ok(())
}

#[test]
fn context_free_has_rejects_one_argument() -> Result<(), Box<dyn Error>> {
    let result = eval_helper(ExprHelper::Has, &[SlotValue::Object(ObjectId::new(0))]);
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
fn context_free_has_rejects_three_arguments() -> Result<(), Box<dyn Error>> {
    let args = [
        SlotValue::Object(ObjectId::new(0)),
        SlotValue::Symbol(SymbolId::new(0)),
        SlotValue::Null,
    ];
    let result = eval_helper(ExprHelper::Has, &args);
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
fn context_free_has_requires_store_for_tagged_handles() -> Result<(), Box<dyn Error>> {
    let args = [
        SlotValue::Object(ObjectId::new(0)),
        SlotValue::Symbol(SymbolId::new(0)),
    ];
    let result = eval_helper(ExprHelper::Has, &args);
    assert_eq!(
        result,
        Err(ExprError::TypeMismatch {
            expected: "value-store context required for object field lookup".to_string(),
            found: "object handle without store".to_string()
        })
    );
    Ok(())
}
