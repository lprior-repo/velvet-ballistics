use crate::eval::{ExprHelper, eval_helper};
use crate::{ExprError, ExprResult};
use vb_core::SlotValue;
use vb_core::value::FiniteF64;
use vb_core::value_store::ValueStore;

#[test]
fn no_store_contains_checks_receiver_before_float_needle() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let symbol = store.insert_symbol("receiver")?;
    let needle = SlotValue::F64(FiniteF64::new(2.5)?);
    let args = [SlotValue::Symbol(symbol), needle];
    assert_eq!(
        eval_helper(ExprHelper::Contains, &args),
        Err(ExprError::TypeMismatch {
            expected: "list".into(),
            found: "symbol".into(),
        })
    );
    Ok(())
}

#[test]
fn no_store_contains_list_with_float_reports_missing_store_context() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store.insert_list(vec![].into_boxed_slice())?;
    let needle = SlotValue::F64(FiniteF64::new(2.5)?);
    let args = [SlotValue::List(list), needle];
    assert_eq!(
        eval_helper(ExprHelper::Contains, &args),
        Err(ExprError::TypeMismatch {
            expected: "value-store context required for list contains check".into(),
            found: "list handle without store".into(),
        })
    );
    Ok(())
}

#[test]
fn no_store_contains_float_receiver_reports_list_mismatch() -> ExprResult<()> {
    let receiver = SlotValue::F64(FiniteF64::new(2.5)?);
    let args = [receiver, SlotValue::I64(7)];
    assert_eq!(
        eval_helper(ExprHelper::Contains, &args),
        Err(ExprError::TypeMismatch {
            expected: "list".into(),
            found: "number".into(),
        })
    );
    Ok(())
}
