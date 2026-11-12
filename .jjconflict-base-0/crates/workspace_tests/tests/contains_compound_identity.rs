#[path = "support/contains_three_routes.rs"]
mod contains_three_routes;

use contains_three_routes::{ContainsFixture, assert_routes, fixture_with_store};
use vb_core::{ObjectField, SlotValue, ValueStore};

fn object_fixture() -> Result<(ContainsFixture, SlotValue, SlotValue), String> {
    let mut store = ValueStore::new();
    let key = store
        .insert_symbol("field")
        .map_err(|error| format!("{error:?}"))?;
    let fields = vec![ObjectField::clean(key, SlotValue::I64(9))];
    let first = store
        .insert_object(fields.clone())
        .map_err(|error| format!("{error:?}"))?;
    let second = store
        .insert_object(fields)
        .map_err(|error| format!("{error:?}"))?;
    assert_ne!(first, second);
    assert_eq!(store.object(first), store.object(second));
    let fixture = fixture_with_store(store, vec![SlotValue::Object(first)])?;
    Ok((fixture, SlotValue::Object(first), SlotValue::Object(second)))
}

fn blob_fixture() -> Result<(ContainsFixture, SlotValue, SlotValue), String> {
    let mut store = ValueStore::new();
    let first = store
        .insert_blob(b"same-payload".as_slice())
        .map_err(|error| format!("{error:?}"))?;
    let second = store
        .insert_blob(b"same-payload".as_slice())
        .map_err(|error| format!("{error:?}"))?;
    assert_ne!(first, second);
    assert_eq!(store.blob(first), store.blob(second));
    let fixture = fixture_with_store(store, vec![SlotValue::Blob(first)])?;
    Ok((fixture, SlotValue::Blob(first), SlotValue::Blob(second)))
}

fn list_fixture() -> Result<(ContainsFixture, SlotValue, SlotValue), String> {
    let mut store = ValueStore::new();
    let payload = vec![SlotValue::I64(3), SlotValue::I64(8)];
    let first = store
        .insert_list(payload.clone())
        .map_err(|error| format!("{error:?}"))?;
    let second = store
        .insert_list(payload)
        .map_err(|error| format!("{error:?}"))?;
    assert_ne!(first, second);
    assert_eq!(store.list(first), store.list(second));
    let fixture = fixture_with_store(store, vec![SlotValue::List(first)])?;
    Ok((fixture, SlotValue::List(first), SlotValue::List(second)))
}

#[test]
fn same_object_handle_is_a_member_on_all_routes() -> Result<(), String> {
    let (mut fixture, matching, _) = object_fixture()?;
    assert_routes(&mut fixture, matching, true)
}

#[test]
fn equal_payload_distinct_object_handle_is_not_a_member_on_all_routes() -> Result<(), String> {
    let (mut fixture, _, distinct) = object_fixture()?;
    assert_routes(&mut fixture, distinct, false)
}

#[test]
fn same_blob_handle_is_a_member_on_all_routes() -> Result<(), String> {
    let (mut fixture, matching, _) = blob_fixture()?;
    assert_routes(&mut fixture, matching, true)
}

#[test]
fn equal_bytes_distinct_blob_handle_is_not_a_member_on_all_routes() -> Result<(), String> {
    let (mut fixture, _, distinct) = blob_fixture()?;
    assert_routes(&mut fixture, distinct, false)
}

#[test]
fn same_list_handle_is_a_member_on_all_routes() -> Result<(), String> {
    let (mut fixture, matching, _) = list_fixture()?;
    assert_routes(&mut fixture, matching, true)
}

#[test]
fn equal_contents_distinct_list_handle_is_not_a_member_on_all_routes() -> Result<(), String> {
    let (mut fixture, _, distinct) = list_fixture()?;
    assert_routes(&mut fixture, distinct, false)
}
