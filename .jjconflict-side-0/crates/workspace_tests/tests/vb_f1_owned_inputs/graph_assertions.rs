use super::super::support::input_slot;
use super::TestResult;
use vb_core::{InputSlotKind, ListId, SlotValue, Taint, ValueStore};

pub(super) fn assert_owned_graph(
    loaded: &vb_storage::OwnedRunInputsRecordV1,
    workflow: &vb_core::CompiledWorkflow,
) -> TestResult {
    let bundle = loaded.restore(workflow)?;
    assert!(bundle.matches_workflow(&workflow));
    assert_eq!(bundle.input_slots(), workflow.input_slots());
    assert_eq!(*bundle.resource_contract(), workflow.resource_contract());
    let entries = input_slot(workflow, InputSlotKind::List)?;
    assert_eq!(
        bundle.root(entries),
        Some((SlotValue::List(ListId::new(2)), Taint::Secret))
    );
    let store = ValueStore::from_checkpoint_v1(bundle.checkpoint().clone())?;
    assert_list_values_and_taints(&store)?;
    Ok(())
}

fn assert_list_values_and_taints(store: &ValueStore) -> TestResult {
    assert_eq!(
        store.list(ListId::new(0))?,
        &[SlotValue::I64(7), SlotValue::I64(1), SlotValue::I64(7)]
    );
    assert_eq!(
        store.list_item_with_taint(ListId::new(0), 1)?,
        (SlotValue::I64(1), Taint::DerivedFromSecret)
    );
    assert_eq!(
        store.list_item_with_taint(ListId::new(0), 2)?,
        (SlotValue::I64(7), Taint::Secret)
    );
    Ok(())
}
