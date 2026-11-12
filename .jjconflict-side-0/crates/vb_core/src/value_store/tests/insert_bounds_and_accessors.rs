#[test]
fn insert_list_rejects_payload_over_hard_bound() -> Result<(), String> {
    let mut store = ValueStore::new();
    let values =
        vec![SlotValue::Null; MAX_LIST_ITEMS_PER_VALUE.saturating_add(1)].into_boxed_slice();

    match store.insert_list(values) {
        Err(CoreError::ResourceLimitExceeded {
            resource: "list_items",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn insert_object_rejects_payload_over_hard_bound() -> Result<(), String> {
    let mut store = ValueStore::new();
    let field = ObjectField {
        key: SymbolId::new(0),
        value: SlotValue::Null,
        taint: Taint::Clean,
    };
    let fields = vec![field; MAX_OBJECT_FIELDS_PER_VALUE.saturating_add(1)].into_boxed_slice();

    match store.insert_object(fields) {
        Err(CoreError::ResourceLimitExceeded {
            resource: "object_fields",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn insert_symbol_rejects_payload_over_hard_bound() -> Result<(), String> {
    let mut store = ValueStore::new();
    let value = "x".repeat(MAX_SYMBOL_BYTES_PER_VALUE.saturating_add(1));

    match store.insert_symbol(value.into_boxed_str()) {
        Err(CoreError::ResourceLimitExceeded {
            resource: "symbol_bytes",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn insert_blob_rejects_payload_over_hard_bound() -> Result<(), String> {
    let mut store = ValueStore::new();
    let bytes = vec![0u8; MAX_BLOB_BYTES_PER_VALUE.saturating_add(1)];

    match store.insert_blob(Bytes::from(bytes)) {
        Err(CoreError::ResourceLimitExceeded {
            resource: "blob_bytes",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn symbol_and_blob_accessors_return_payloads() -> Result<(), String> {
    let mut store = ValueStore::new();
    let symbol = store
        .insert_symbol(Box::<str>::from("alpha"))
        .map_err(|error| error.to_string())?;
    let blob = store
        .insert_blob(Bytes::from_static(b"payload"))
        .map_err(|error| error.to_string())?;

    if store.symbol(symbol).map_err(|error| error.to_string())? != "alpha" {
        return Err(String::from("unexpected symbol payload"));
    }
    if store.blob(blob).map_err(|error| error.to_string())? != b"payload" {
        return Err(String::from("unexpected blob payload"));
    }
    Ok(())
}

#[test]
fn arena_accessors_report_handle_bounds() -> Result<(), String> {
    let store = ValueStore::new();

    if store.symbol(SymbolId::new(0))
        != Err(CoreError::SymbolOutOfBounds {
            symbol: SymbolId::new(0),
        })
    {
        return Err(String::from("expected symbol bounds error"));
    }
    if store.list(ListId::new(0))
        != Err(CoreError::ListOutOfBounds {
            list: ListId::new(0),
        })
    {
        return Err(String::from("expected list bounds error"));
    }
    if store.object(ObjectId::new(0))
        != Err(CoreError::ObjectOutOfBounds {
            object: ObjectId::new(0),
        })
    {
        return Err(String::from("expected object bounds error"));
    }
    if store.blob(BlobId::new(0))
        != Err(CoreError::BlobOutOfBounds {
            blob: BlobId::new(0),
        })
    {
        return Err(String::from("expected blob bounds error"));
    }
    Ok(())
}

#[test]
fn list_item_and_object_field_accessors_are_checked() -> Result<(), String> {
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![SlotValue::I64(10)].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    let object = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(4),
                value: SlotValue::Bool(true),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|error| error.to_string())?;

    if store
        .list_item(list, 0)
        .map_err(|error| error.to_string())?
        != SlotValue::I64(10)
    {
        return Err(String::from("unexpected list item"));
    }
    if store
        .object_field(object, SymbolId::new(4))
        .map_err(|error| error.to_string())?
        != SlotValue::Bool(true)
    {
        return Err(String::from("unexpected object field"));
    }
    if store.list_item(list, 1) != Err(CoreError::ListIndexOutOfBounds { index: 1 }) {
        return Err(String::from("expected list item bounds error"));
    }
    if store.object_field(object, SymbolId::new(5))
        != Err(CoreError::ObjectFieldNotFound {
            field: SymbolId::new(5),
        })
    {
        return Err(String::from("expected missing object field error"));
    }
    Ok(())
}
