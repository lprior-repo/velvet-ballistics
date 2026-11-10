use super::*;

#[test]
fn list_payloads_reject_invalid_handles_without_mutating_store() -> CoreResult<()> {
    let mut store = ValueStore::new();
    let source =
        store.insert_list_with_taint(Box::from([SlotValue::Null]), Box::from([Taint::Secret]))?;
    let invalid = ListId::new(99);
    let expected = CoreError::ListOutOfBounds { list: invalid };
    assert_eq!(
        store.prepare_list_append(invalid, None),
        Err(expected.clone())
    );
    assert_eq!(store.prepare_list_unique(invalid), Err(expected.clone()));
    assert_eq!(
        store.list_contains_with_taint(invalid, SlotValue::Null),
        Err(expected)
    );
    assert_eq!(store.list_count(), 1);
    assert_eq!(
        store.list_item_with_taint(source, 0)?,
        (SlotValue::Null, Taint::Secret)
    );
    Ok(())
}

#[test]
fn append_payload_obeys_hard_limit_and_false_copy_preserves_all_items() -> CoreResult<()> {
    let mut store = ValueStore::new();
    let count = crate::limits::MAX_LIST_ITEMS_PER_VALUE;
    let source = store.insert_list_with_taint(
        vec![SlotValue::I64(7); count].into_boxed_slice(),
        vec![Taint::Random; count].into_boxed_slice(),
    )?;
    assert_eq!(
        store.prepare_list_append(source, Some((SlotValue::Null, Taint::Secret))),
        Err(CoreError::ResourceLimitExceeded {
            resource: "list_items"
        })
    );
    let (values, taints) = store.prepare_list_append(source, None)?;
    assert_eq!(values.as_ref(), store.list(source)?);
    assert_eq!(taints.as_ref(), vec![Taint::Random; count].as_slice());
    assert_eq!(store.list_count(), 1);
    let copy = store.insert_list_with_taint(values, taints)?;
    assert_ne!(source, copy);
    assert_eq!(store.list_count(), 2);
    assert_eq!(
        store.list_items_with_taint(source)?.collect::<Vec<_>>(),
        store.list_items_with_taint(copy)?.collect::<Vec<_>>()
    );
    Ok(())
}
