use vb_core::{FiniteF64, ListId, SlotValue, ValueStore};
use vb_expr::ExprError;
use vb_expr::eval::eval_helper_with_store;
use vb_expr::parser::ExprHelper;

fn list_with(values: &[SlotValue]) -> Result<(ValueStore, ListId), String> {
    list_with_in_store(ValueStore::new(), values)
}

fn list_with_in_store(
    mut store: ValueStore,
    values: &[SlotValue],
) -> Result<(ValueStore, ListId), String> {
    let id = store
        .insert_list(values.to_vec().into_boxed_slice())
        .map_err(|error| error.to_string())?;
    Ok((store, id))
}

fn contains(
    store: &mut ValueStore,
    list: ListId,
    needle: SlotValue,
) -> Result<SlotValue, ExprError> {
    eval_helper_with_store(
        ExprHelper::Contains,
        &[SlotValue::List(list), needle],
        store,
    )
}

fn finite(value: f64) -> Result<FiniteF64, String> {
    FiniteF64::new(value).map_err(|error| error.to_string())
}

#[test]
fn contains_returns_true_for_same_f64_tag_and_payload() -> Result<(), String> {
    let value = SlotValue::F64(finite(7.0)?);
    let (mut store, list) = list_with(&[value])?;
    assert_eq!(contains(&mut store, list, value), Ok(SlotValue::Bool(true)));
    Ok(())
}

#[test]
fn contains_returns_false_for_equal_numeric_payload_with_different_tags() -> Result<(), String> {
    let value = SlotValue::F64(finite(7.0)?);
    let (mut store, list) = list_with(&[SlotValue::I64(7)])?;
    assert_eq!(
        contains(&mut store, list, value),
        Ok(SlotValue::Bool(false))
    );
    Ok(())
}

#[test]
fn contains_returns_true_for_null_needle() -> Result<(), String> {
    let (mut store, list) = list_with(&[SlotValue::Null])?;
    assert_eq!(
        contains(&mut store, list, SlotValue::Null),
        Ok(SlotValue::Bool(true))
    );
    Ok(())
}

#[test]
fn contains_returns_true_for_boolean_needle() -> Result<(), String> {
    let (mut store, list) = list_with(&[SlotValue::Bool(true)])?;
    assert_eq!(
        contains(&mut store, list, SlotValue::Bool(true)),
        Ok(SlotValue::Bool(true))
    );
    Ok(())
}

#[test]
fn contains_returns_true_for_same_symbol_id() -> Result<(), String> {
    let mut store = ValueStore::new();
    let symbol = store
        .insert_symbol("same text")
        .map_err(|error| error.to_string())?;
    let (mut store, list) = list_with_in_store(store, &[SlotValue::Symbol(symbol)])?;
    assert_eq!(
        contains(&mut store, list, SlotValue::Symbol(symbol)),
        Ok(SlotValue::Bool(true))
    );
    Ok(())
}

#[test]
fn contains_returns_false_for_equal_symbol_text_with_distinct_ids() -> Result<(), String> {
    let mut store = ValueStore::new();
    let first = store
        .insert_symbol("same text")
        .map_err(|error| error.to_string())?;
    let second = store
        .insert_symbol("same text")
        .map_err(|error| error.to_string())?;
    let list = store
        .insert_list(vec![SlotValue::Symbol(first)].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    assert_eq!(
        contains(&mut store, list, SlotValue::Symbol(second)),
        Ok(SlotValue::Bool(false))
    );
    Ok(())
}

#[test]
fn contains_returns_true_for_same_invalid_raw_list_handle() -> Result<(), String> {
    let invalid = SlotValue::List(ListId::new(u32::MAX));
    let (mut store, list) = list_with(&[invalid])?;
    assert_eq!(
        contains(&mut store, list, invalid),
        Ok(SlotValue::Bool(true))
    );
    Ok(())
}

#[test]
fn contains_returns_false_for_distinct_invalid_raw_list_handles() -> Result<(), String> {
    let stored = SlotValue::List(ListId::new(u32::MAX));
    let needle_id = u32::MAX
        .checked_sub(1)
        .ok_or_else(|| "distinct invalid handle representable".to_owned())?;
    let needle = SlotValue::List(ListId::new(needle_id));
    let (mut store, list) = list_with(&[stored])?;
    assert_eq!(
        contains(&mut store, list, needle),
        Ok(SlotValue::Bool(false))
    );
    Ok(())
}

#[test]
fn contains_returns_false_for_invalid_raw_handle_needle_in_empty_list() -> Result<(), String> {
    let invalid = SlotValue::List(ListId::new(u32::MAX));
    let (mut store, list) = list_with(&[])?;
    assert_eq!(
        contains(&mut store, list, invalid),
        Ok(SlotValue::Bool(false))
    );
    Ok(())
}

#[test]
fn contains_returns_false_for_invalid_stored_handle_and_scalar_needle() -> Result<(), String> {
    let invalid = SlotValue::List(ListId::new(u32::MAX));
    let (mut store, list) = list_with(&[invalid])?;
    assert_eq!(
        contains(&mut store, list, SlotValue::I64(7)),
        Ok(SlotValue::Bool(false))
    );
    Ok(())
}

#[test]
fn contains_returns_exact_type_mismatch_for_wrong_receiver() -> Result<(), String> {
    let mut store = ValueStore::new();
    let receiver = SlotValue::I64(7);
    let result = eval_helper_with_store(
        ExprHelper::Contains,
        &[receiver, SlotValue::Null],
        &mut store,
    );
    assert_eq!(
        result,
        Err(ExprError::TypeMismatch {
            expected: "list".into(),
            found: "number".into(),
        })
    );
    Ok(())
}

#[test]
fn contains_returns_exact_invalid_reference_for_invalid_receiver_id() -> Result<(), String> {
    let mut store = ValueStore::new();
    let id = ListId::new(u32::MAX);
    let result = contains(&mut store, id, SlotValue::Null);
    assert_eq!(
        result,
        Err(ExprError::InvalidReference {
            reference: format!("list:{id:?}"),
        })
    );
    Ok(())
}
