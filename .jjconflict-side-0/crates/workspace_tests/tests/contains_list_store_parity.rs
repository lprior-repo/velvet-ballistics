use vb_core::{SlotValue, ValueStore};
use vb_expr::eval::{ExprHelper, eval_helper_with_store};

#[test]
fn list_contains_true() -> Result<(), String> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![SlotValue::I64(7), SlotValue::I64(11)])
        .map_err(|e| format!("{:?}", e))?;
    let args = [SlotValue::List(list), SlotValue::I64(7)];
    let result = eval_helper_with_store(ExprHelper::Contains, &args, &mut store)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn list_contains_false() -> Result<(), String> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![SlotValue::I64(7), SlotValue::I64(11)])
        .map_err(|e| format!("{:?}", e))?;
    let args = [SlotValue::List(list), SlotValue::I64(13)];
    let result = eval_helper_with_store(ExprHelper::Contains, &args, &mut store)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn empty_list_contains() -> Result<(), String> {
    let mut store = ValueStore::new();
    let list = store.insert_list(vec![]).map_err(|e| format!("{:?}", e))?;
    let args = [SlotValue::List(list), SlotValue::I64(7)];
    let result = eval_helper_with_store(ExprHelper::Contains, &args, &mut store)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}
