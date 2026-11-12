fn has_parity_replay_eval(
    store: &mut ValueStore,
    fields: Vec<crate::ObjectField>,
    key: SymbolId,
) -> Result<SlotValue, CoreError> {
    let object = store.insert_object(fields.into_boxed_slice())?;
    eval_stack_helper(
        ExprOp::Has,
        store,
        &[SlotValue::Object(object), SlotValue::Symbol(key)],
    )
}

#[test]
fn has_parity_replay_valid_key_true() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let name_key = store.insert_symbol("name")?;
    let age_key = store.insert_symbol("age")?;
    let fields = vec![
        crate::ObjectField::clean(name_key, SlotValue::I64(1)),
        crate::ObjectField::clean(age_key, SlotValue::I64(2)),
    ];
    let result = has_parity_replay_eval(&mut store, fields, name_key)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn has_parity_replay_missing_key_false() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let name_key = store.insert_symbol("name")?;
    let age_key = store.insert_symbol("age")?;
    let missing_key = store.insert_symbol("missing")?;
    let fields = vec![
        crate::ObjectField::clean(name_key, SlotValue::I64(1)),
        crate::ObjectField::clean(age_key, SlotValue::I64(2)),
    ];
    let result = has_parity_replay_eval(&mut store, fields, missing_key)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn has_parity_replay_empty_object_false() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let key = store.insert_symbol("key")?;
    let result = has_parity_replay_eval(&mut store, vec![], key)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn has_parity_replay_one_other_field_false() -> Result<(), CoreError> {
    let mut store = ValueStore::new();
    let other_key = store.insert_symbol("other")?;
    let wanted_key = store.insert_symbol("wanted")?;
    let fields = vec![crate::ObjectField::clean(other_key, SlotValue::I64(1))];
    let result = has_parity_replay_eval(&mut store, fields, wanted_key)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}
