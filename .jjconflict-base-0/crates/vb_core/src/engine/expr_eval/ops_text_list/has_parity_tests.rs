fn has_parity_hot_eval(
    store: &mut ValueStore,
    fields: Vec<crate::ObjectField>,
    key: crate::ids::SymbolId,
) -> Result<SlotValue, String> {
    let obj = store
        .insert_object(fields.into_boxed_slice())
        .map_err(|e| e.to_string())?;
    eval_ops_with_slots(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::Has,
        ],
        vec![SlotValue::Object(obj)],
        vec![ConstValue::Symbol(key)],
        store,
    )
}

#[test]
fn has_parity_hot_object_has_requested_key() -> Result<(), String> {
    let mut store = ValueStore::new();
    let existing_key = store.insert_symbol("name").map_err(|e| e.to_string())?;
    let other_key = store.insert_symbol("age").map_err(|e| e.to_string())?;
    let fields = vec![
        crate::ObjectField::clean(existing_key, SlotValue::I64(1)),
        crate::ObjectField::clean(other_key, SlotValue::I64(2)),
    ];
    let result = has_parity_hot_eval(&mut store, fields, existing_key)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn has_parity_hot_object_missing_key() -> Result<(), String> {
    let mut store = ValueStore::new();
    let key1 = store.insert_symbol("color").map_err(|e| e.to_string())?;
    let key2 = store.insert_symbol("size").map_err(|e| e.to_string())?;
    let asked_key = store.insert_symbol("name").map_err(|e| e.to_string())?;
    let fields = vec![
        crate::ObjectField::clean(key1, SlotValue::I64(1)),
        crate::ObjectField::clean(key2, SlotValue::I64(2)),
    ];
    let result = has_parity_hot_eval(&mut store, fields, asked_key)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn has_parity_hot_empty_object() -> Result<(), String> {
    let mut store = ValueStore::new();
    let asked_key = store.insert_symbol("anything").map_err(|e| e.to_string())?;
    let result = has_parity_hot_eval(&mut store, vec![], asked_key)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn has_parity_hot_object_other_key_only() -> Result<(), String> {
    let mut store = ValueStore::new();
    let present_key = store.insert_symbol("type").map_err(|e| e.to_string())?;
    let asked_key = store.insert_symbol("name").map_err(|e| e.to_string())?;
    let fields = vec![crate::ObjectField::clean(present_key, SlotValue::I64(1))];
    let result = has_parity_hot_eval(&mut store, fields, asked_key)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}
