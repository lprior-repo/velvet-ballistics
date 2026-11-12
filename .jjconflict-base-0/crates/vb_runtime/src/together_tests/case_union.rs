use vb_core::{EngineSignal, FiniteF64, ListId};

type UnionResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn join_one(fixture: &mut Fixture, accumulator: SlotIdx, output: SlotIdx) -> UnionResult {
    assert_eq!(
        fixture.join(
            1,
            (accumulator, Some(output)),
            (Some(StepIdx::new(5)), StepIdx::ZERO)
        ),
        Ok(EngineSignal::Continue)
    );
    Ok(())
}

fn union_list_id(fixture: &Fixture, slot: SlotIdx) -> UnionResult<ListId> {
    let value = fixture.run.read_slot(slot)?;
    match *value {
        SlotValue::List(id) => Ok(id),
        other => Err(format!("expected list but found {:?}", other).into()),
    }
}

fn assert_tainted_pair(
    store: &ValueStore,
    id: ListId,
    index: u32,
    expected: (SlotValue, Taint),
) -> UnionResult {
    assert_eq!(store.list_item_with_taint(id, index)?, expected);
    Ok(())
}

#[test]
fn together_join_with_distinct_null_result_appends_null() -> UnionResult {
    let mut fixture = Fixture::fresh();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    fixture.seed(accumulator, vec![SlotValue::I64(10)]);
    fixture.run.write_slot(output, SlotValue::Null)?;
    fixture.run.add_parallel_in_flight(1)?;
    join_one(&mut fixture, accumulator, output)?;
    let result = union_list_id(&fixture, output)?;
    let items = fixture.store.list(result)?;
    assert_eq!(items, &[SlotValue::I64(10), SlotValue::Null]);
    Ok(())
}

#[test]
fn together_join_with_distinct_list_result_appends_nested_list() -> UnionResult {
    let mut fixture = Fixture::fresh();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    fixture.seed(accumulator, vec![SlotValue::I64(10)]);
    let inner = fixture
        .store
        .insert_list(vec![SlotValue::I64(20)].into_boxed_slice())?;
    fixture.run.write_slot(output, SlotValue::List(inner))?;
    fixture.run.add_parallel_in_flight(1)?;
    join_one(&mut fixture, accumulator, output)?;
    let result = union_list_id(&fixture, output)?;
    let items = fixture.store.list(result)?;
    assert_eq!(items, &[SlotValue::I64(10), SlotValue::List(inner)]);
    Ok(())
}

#[test]
fn together_join_legacy_aliased_accumulator_does_not_append_itself() -> UnionResult {
    let mut fixture = Fixture::fresh();
    let slot = SlotIdx::new(0);
    let value = SlotValue::F64(FiniteF64::new(2.5)?);
    let id = fixture.store.insert_list_with_taint(
        Box::from([SlotValue::I64(7), value]),
        Box::from([Taint::Secret, Taint::DerivedFromSecret]),
    )?;
    fixture
        .run
        .write_slot_with_taint(slot, SlotValue::List(id), Taint::Secret)?;
    fixture.run.add_parallel_in_flight(1)?;
    join_one(&mut fixture, slot, slot)?;
    assert_alias_union(&fixture, slot, id, value)?;
    Ok(())
}

#[test]
fn together_join_distinct_slots_with_same_list_handle_still_append() -> UnionResult {
    let mut fixture = Fixture::fresh();
    let accumulator = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let id = fixture.store.insert_list_with_taint(
        Box::from([SlotValue::I64(7)]),
        Box::from([Taint::DerivedFromSecret]),
    )?;
    fixture.run.write_slot_with_taint(
        accumulator,
        SlotValue::List(id),
        Taint::DerivedFromSecret,
    )?;
    fixture
        .run
        .write_slot_with_taint(output, SlotValue::List(id), Taint::Secret)?;
    fixture.run.add_parallel_in_flight(1)?;
    join_one(&mut fixture, accumulator, output)?;
    assert_distinct_union(&fixture, output, id)?;
    Ok(())
}
fn assert_alias_union(
    fixture: &Fixture,
    slot: SlotIdx,
    id: ListId,
    value: SlotValue,
) -> UnionResult {
    assert_eq!(*fixture.run.read_slot(slot)?, SlotValue::List(id));
    assert_eq!(fixture.store.list(id)?, &[SlotValue::I64(7), value]);
    assert_tainted_pair(&fixture.store, id, 0, (SlotValue::I64(7), Taint::Secret))?;
    assert_tainted_pair(&fixture.store, id, 1, (value, Taint::DerivedFromSecret))?;
    assert_eq!(fixture.run.read_taint(slot)?, Taint::Secret);
    assert_eq!(fixture.run.parallel_in_flight(), 0);
    Ok(())
}

fn assert_distinct_union(fixture: &Fixture, output: SlotIdx, id: ListId) -> UnionResult {
    let result = union_list_id(&fixture, output)?;
    assert_eq!(
        fixture.store.list(result)?,
        &[SlotValue::I64(7), SlotValue::List(id)]
    );
    assert_tainted_pair(
        &fixture.store,
        result,
        0,
        (SlotValue::I64(7), Taint::DerivedFromSecret),
    )?;
    assert_tainted_pair(
        &fixture.store,
        result,
        1,
        (SlotValue::List(id), Taint::Secret),
    )?;
    assert_eq!(fixture.run.read_taint(output)?, Taint::Secret);
    assert_eq!(fixture.run.parallel_in_flight(), 0);
    Ok(())
}
