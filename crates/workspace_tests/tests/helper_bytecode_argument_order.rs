#![forbid(unsafe_code)]

use vb_core::ids::SymbolId;
use vb_core::value_store::{ObjectField, ValueStore};
use vb_core::{ExprOp, ExprProgram, SlotIdx, SlotValue};
use vb_expr::eval::eval_expr_program_with_store;

struct Symbols {
    text: SymbolId,
    prefix: SymbolId,
    suffix: SymbolId,
    shared: SymbolId,
    left_only: SymbolId,
    right_only: SymbolId,
}

struct Fixture {
    store: ValueStore,
    slots: Vec<Option<SlotValue>>,
    symbols: Symbols,
}

fn insert_symbol(store: &mut ValueStore, text: &str) -> Result<SymbolId, String> {
    store
        .insert_symbol(text)
        .map_err(|error| format!("{error:?}"))
}

fn insert_symbols(store: &mut ValueStore) -> Result<Symbols, String> {
    Ok(Symbols {
        text: insert_symbol(store, "woodpecker")?,
        prefix: insert_symbol(store, "wood")?,
        suffix: insert_symbol(store, "cker")?,
        shared: insert_symbol(store, "shared")?,
        left_only: insert_symbol(store, "left-only")?,
        right_only: insert_symbol(store, "right-only")?,
    })
}

fn insert_objects(
    store: &mut ValueStore,
    symbols: &Symbols,
) -> Result<(SlotValue, SlotValue), String> {
    let left = store
        .insert_object(
            vec![
                ObjectField::clean(symbols.shared, SlotValue::I64(11)),
                ObjectField::clean(symbols.left_only, SlotValue::I64(4)),
            ]
            .into_boxed_slice(),
        )
        .map_err(|error| format!("{error:?}"))?;
    let right = store
        .insert_object(
            vec![
                ObjectField::clean(symbols.shared, SlotValue::I64(22)),
                ObjectField::clean(symbols.right_only, SlotValue::I64(9)),
            ]
            .into_boxed_slice(),
        )
        .map_err(|error| format!("{error:?}"))?;
    Ok((SlotValue::Object(left), SlotValue::Object(right)))
}

fn insert_list(store: &mut ValueStore, items: Vec<SlotValue>) -> Result<SlotValue, String> {
    let list = store
        .insert_list(items.into_boxed_slice())
        .map_err(|error| format!("{error:?}"))?;
    Ok(SlotValue::List(list))
}

fn build_fixture_slots(
    symbols: &Symbols,
    contains_list: SlotValue,
    append_list: SlotValue,
    left: SlotValue,
    right: SlotValue,
) -> Vec<Option<SlotValue>> {
    vec![
        Some(contains_list),
        Some(SlotValue::I64(11)),
        Some(SlotValue::Symbol(symbols.text)),
        Some(SlotValue::Symbol(symbols.prefix)),
        Some(SlotValue::Symbol(symbols.suffix)),
        Some(append_list),
        Some(SlotValue::I64(8)),
        Some(SlotValue::Bool(true)),
        Some(left),
        Some(right),
    ]
}

fn fixture() -> Result<Fixture, String> {
    let mut store = ValueStore::new();
    let symbols = insert_symbols(&mut store)?;
    let contains_list = insert_list(&mut store, vec![SlotValue::I64(5), SlotValue::I64(11)])?;
    let append_list = insert_list(&mut store, vec![SlotValue::I64(3)])?;
    let (left, right) = insert_objects(&mut store, &symbols)?;
    let slots = build_fixture_slots(&symbols, contains_list, append_list, left, right);
    Ok(Fixture {
        store,
        slots,
        symbols,
    })
}

fn evaluate(ops: Vec<ExprOp>, fixture: &mut Fixture) -> Result<SlotValue, String> {
    let program =
        ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|error| format!("{error:?}"))?;
    eval_expr_program_with_store(&program, &fixture.slots, &[], &mut fixture.store)
        .map_err(|error| format!("{error:?}"))
}

fn assert_appended(fixture: &mut Fixture, op: ExprOp) -> Result<(), String> {
    let result = evaluate(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(5)),
            ExprOp::LoadSlot(SlotIdx::new(6)),
            op,
        ],
        fixture,
    )?;
    let SlotValue::List(list) = result else {
        return Err(format!("expected List, got {result:?}"));
    };
    let items = fixture
        .store
        .list(list)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(items, &[SlotValue::I64(3), SlotValue::I64(8)]);
    Ok(())
}

fn assert_field_value(fields: &[ObjectField], key: SymbolId, expected: SlotValue) {
    assert_eq!(
        fields
            .iter()
            .find(|field| field.key == key)
            .map(|field| field.value),
        Some(expected)
    );
}

fn assert_merge_fields(
    store: &ValueStore,
    object_id: vb_core::ids::ObjectId,
    symbols: &Symbols,
) -> Result<(), String> {
    let fields = store
        .object(object_id)
        .map_err(|error| format!("{error:?}"))?;
    assert_field_value(&fields, symbols.shared, SlotValue::I64(22));
    assert_field_value(&fields, symbols.left_only, SlotValue::I64(4));
    assert_field_value(&fields, symbols.right_only, SlotValue::I64(9));
    Ok(())
}

#[test]
fn contains_preserves_source_order() -> Result<(), String> {
    let mut fixture = fixture()?;
    let result = evaluate(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Contains,
        ],
        &mut fixture,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn starts_with_preserves_source_order() -> Result<(), String> {
    let mut fixture = fixture()?;
    let result = evaluate(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(2)),
            ExprOp::LoadSlot(SlotIdx::new(3)),
            ExprOp::StartsWith,
        ],
        &mut fixture,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn ends_with_preserves_source_order() -> Result<(), String> {
    let mut fixture = fixture()?;
    let result = evaluate(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(2)),
            ExprOp::LoadSlot(SlotIdx::new(4)),
            ExprOp::EndsWith,
        ],
        &mut fixture,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn append_preserves_source_order() -> Result<(), String> {
    let mut fixture = fixture()?;
    assert_appended(&mut fixture, ExprOp::Append)
}

#[test]
fn merge_preserves_source_order_and_right_wins() -> Result<(), String> {
    let mut fixture = fixture()?;
    let result = evaluate(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(8)),
            ExprOp::LoadSlot(SlotIdx::new(9)),
            ExprOp::Merge,
        ],
        &mut fixture,
    )?;
    let SlotValue::Object(object) = result else {
        return Err(format!("expected Object, got {result:?}"));
    };
    assert_merge_fields(&fixture.store, object, &fixture.symbols)
}

#[test]
fn append_if_preserves_source_order() -> Result<(), String> {
    let mut fixture = fixture()?;
    let result = evaluate(
        vec![
            ExprOp::LoadSlot(SlotIdx::new(5)),
            ExprOp::LoadSlot(SlotIdx::new(6)),
            ExprOp::LoadSlot(SlotIdx::new(7)),
            ExprOp::AppendIf,
        ],
        &mut fixture,
    )?;
    let SlotValue::List(list) = result else {
        return Err(format!("expected List, got {result:?}"));
    };
    let items = fixture
        .store
        .list(list)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(items, &[SlotValue::I64(3), SlotValue::I64(8)]);
    Ok(())
}
