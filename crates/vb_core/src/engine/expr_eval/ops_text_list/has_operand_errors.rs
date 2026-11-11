fn has_error_for(
    store: &ValueStore,
    receiver: SlotValue,
    key: SlotValue,
) -> Result<(), EngineError> {
    let mut stack = ExprStack::new(2)?;
    stack.push(receiver)?;
    stack.push(key)?;
    eval_has(&mut stack, store)
}

#[test]
fn has_rejects_boolean_receiver() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let key = store.insert_symbol("field")?;
    assert_eq!(
        has_error_for(&store, SlotValue::Bool(true), SlotValue::Symbol(key)),
        Err(EngineError::TypeMismatch {
            expected: "object",
            found: "boolean",
        })
    );
    Ok(())
}

#[test]
fn has_rejects_numeric_key() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let object = store.insert_object(Vec::<crate::ObjectField>::new().into_boxed_slice())?;
    assert_eq!(
        has_error_for(&store, SlotValue::Object(object), SlotValue::I64(7)),
        Err(EngineError::TypeMismatch {
            expected: "text",
            found: "number",
        })
    );
    Ok(())
}

#[test]
fn has_checks_receiver_type_before_key_type() -> Result<(), EngineError> {
    let store = ValueStore::new();
    assert_eq!(
        has_error_for(&store, SlotValue::Bool(true), SlotValue::I64(7)),
        Err(EngineError::TypeMismatch {
            expected: "object",
            found: "boolean",
        })
    );
    Ok(())
}

#[test]
fn has_checks_key_type_before_object_lookup() -> Result<(), EngineError> {
    let store = ValueStore::new();
    assert_eq!(
        has_error_for(
            &store,
            SlotValue::Object(ObjectId::new(u32::MAX)),
            SlotValue::I64(7),
        ),
        Err(EngineError::TypeMismatch {
            expected: "text",
            found: "number",
        })
    );
    Ok(())
}

#[test]
fn has_rejects_invalid_object_handle() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let key = store.insert_symbol("field")?;
    assert_eq!(
        has_error_for(
            &store,
            SlotValue::Object(ObjectId::new(u32::MAX)),
            SlotValue::Symbol(key),
        ),
        Err(EngineError::ObjectOutOfBounds {
            object: ObjectId::new(u32::MAX),
        })
    );
    Ok(())
}

#[test]
fn has_rejects_invalid_symbol_handle_before_membership() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let object = store.insert_object(Vec::<crate::ObjectField>::new().into_boxed_slice())?;
    assert_eq!(
        has_error_for(
            &store,
            SlotValue::Object(object),
            SlotValue::Symbol(SymbolId::new(u32::MAX)),
        ),
        Err(EngineError::SymbolOutOfBounds {
            symbol: SymbolId::new(u32::MAX),
        })
    );
    Ok(())
}

#[test]
fn has_checks_object_before_symbol_handle() -> Result<(), EngineError> {
    let store = ValueStore::new();
    assert_eq!(
        has_error_for(
            &store,
            SlotValue::Object(ObjectId::new(u32::MAX)),
            SlotValue::Symbol(SymbolId::new(u32::MAX)),
        ),
        Err(EngineError::ObjectOutOfBounds {
            object: ObjectId::new(u32::MAX),
        })
    );
    Ok(())
}
