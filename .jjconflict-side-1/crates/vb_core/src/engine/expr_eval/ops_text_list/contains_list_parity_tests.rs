fn contains_list_hot_eval(
    store: &mut ValueStore,
    items: Vec<SlotValue>,
    needle: i64,
) -> Result<SlotValue, String> {
    let list = store
        .insert_list(items.into_boxed_slice())
        .map_err(|e| e.to_string())?;
    eval_ops_with_slots(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Contains,
        ],
        vec![SlotValue::List(list), SlotValue::I64(needle)],
        vec![],
        store,
    )
}

#[test]
fn contains_list_parity_found_element() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result =
        contains_list_hot_eval(&mut store, vec![SlotValue::I64(7), SlotValue::I64(11)], 7)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn contains_list_parity_missing_element() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result =
        contains_list_hot_eval(&mut store, vec![SlotValue::I64(7), SlotValue::I64(11)], 13)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn contains_list_parity_empty_list() -> Result<(), String> {
    let mut store = ValueStore::new();
    let result = contains_list_hot_eval(&mut store, vec![], 7)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}
