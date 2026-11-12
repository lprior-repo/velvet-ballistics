use super::support::TestResult;
use vb_core::value_store::ObjectField;
use vb_core::{CompiledWorkflow, ListId, SlotValue, Taint, ValueStore};

pub(super) const VALUES: [SlotValue; 5] = [
    SlotValue::I64(7),
    SlotValue::Object(vb_core::ObjectId::new(0)),
    SlotValue::List(ListId::new(1)),
    SlotValue::I64(7),
    SlotValue::I64(1),
];
pub(super) const TAINTS: [Taint; 5] = [
    Taint::Clean,
    Taint::Secret,
    Taint::Clean,
    Taint::DerivedFromSecret,
    Taint::Clean,
];

pub(super) fn original(
    workflow: &CompiledWorkflow,
    empty: bool,
) -> TestResult<(ValueStore, ListId)> {
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    if empty {
        let root = store.insert_list(Vec::<SlotValue>::new().into_boxed_slice())?;
        return Ok((store, root));
    }
    let key = store.insert_symbol("child")?;
    let blob = store.insert_blob(vec![3u8, 1, 4])?;
    let child = store.insert_list_with_taint(
        Box::from([SlotValue::I64(7), SlotValue::I64(1), SlotValue::I64(7)]),
        Box::from([Taint::Clean, Taint::DerivedFromSecret, Taint::Secret]),
    )?;
    let object = store.insert_object(Box::from([ObjectField::with_taint(
        key,
        SlotValue::List(child),
        Taint::Secret,
    )]))?;
    let middle = store.insert_list(Box::from([SlotValue::Blob(blob)]))?;
    assert_eq!(object, vb_core::ObjectId::new(0));
    assert_eq!(middle, ListId::new(1));
    let root = store.insert_list_with_taint(
        VALUES.to_vec().into_boxed_slice(),
        TAINTS.to_vec().into_boxed_slice(),
    )?;
    assert_eq!(root, ListId::new(2));
    Ok((store, root))
}
