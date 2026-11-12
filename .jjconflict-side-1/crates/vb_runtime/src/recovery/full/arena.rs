use vb_core::value::SlotValue;
use vb_storage::recovery::RecoveryFrameSeed;

/// Returns true when every arena handle in the recovered seed slots and the
/// retained initial inputs resolves against the restored value store.
///
/// A run with no `List`/`Object`/`Blob` values needs no store (`None` is
/// sufficient). A run carrying arena values resumes only when a restored
/// store is present and every handle resolves; otherwise the frame would
/// hold dangling handles and recovery must stay fail-closed (`store_missing`).
pub(super) fn recovered_arena_handles_resolve(
    seed: &RecoveryFrameSeed,
    initial_inputs: &[(vb_core::SlotIdx, vb_core::SlotValue, vb_core::Taint)],
    store: Option<&vb_core::ValueStore>,
) -> bool {
    let mut handles = seed
        .slots
        .iter()
        .map(|entry| entry.value)
        .chain(initial_inputs.iter().map(|(_, value, _)| *value))
        .filter(|value| slot_value_requires_value_store(*value));
    let Some(store) = store else {
        return handles.next().is_none();
    };
    handles.all(|value| arena_handle_resolves(store, value))
}

const fn slot_value_requires_value_store(value: SlotValue) -> bool {
    matches!(
        value,
        SlotValue::List(_) | SlotValue::Object(_) | SlotValue::Blob(_)
    )
}

fn arena_handle_resolves(store: &vb_core::ValueStore, value: SlotValue) -> bool {
    match value {
        SlotValue::List(id) => store.list(id).is_ok(),
        SlotValue::Object(id) => store.object(id).is_ok(),
        SlotValue::Blob(id) => store.blob(id).is_ok(),
        _ => true,
    }
}
