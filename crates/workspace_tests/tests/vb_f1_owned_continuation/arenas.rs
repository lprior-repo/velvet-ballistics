use super::support::{ROOT, TAINTS, TestResult, VALUES};
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::value_store::ObjectField;
use vb_core::{CompiledWorkflow, InputSlotKind, SlotIdx, SlotValue, Taint, ValueStore};

pub(super) fn original(workflow: &CompiledWorkflow) -> TestResult<(ValueStore, SlotIdx)> {
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
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
    assert_eq!(middle, vb_core::ListId::new(1));
    let root = store.insert_list_with_taint(
        VALUES.to_vec().into_boxed_slice(),
        TAINTS.to_vec().into_boxed_slice(),
    )?;
    assert_eq!(root, ROOT);
    let input = workflow.input_slots().first().ok_or("input missing")?;
    assert_eq!(input.kind, InputSlotKind::List);
    Ok((store, input.slot))
}

pub(super) fn bundle(
    workflow: &CompiledWorkflow,
    store: &ValueStore,
    input: SlotIdx,
) -> TestResult<OwnedRootBundle> {
    Ok(OwnedRootBundle::new(
        Box::from([(input, SlotValue::List(ROOT), Taint::Secret)]),
        store.checkpoint_v1(),
        workflow,
    )?)
}
