#![cfg(test)]
#![forbid(unsafe_code)]

use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;

pub(crate) fn fresh_frame(step_count: u16, slot_count: u16) -> RunFrame {
    RunFrame::new(RunId::new(1), StepIdx::ZERO, step_count, slot_count)
        .ok()
        .unwrap_or_else(|| panic!("fresh_frame({step_count}, {slot_count})"))
}

pub(crate) fn list_in_slot(
    run: &mut RunFrame,
    store: &mut ValueStore,
    slot: SlotIdx,
    items: Vec<SlotValue>,
) {
    let id = store
        .insert_list(items.into_boxed_slice())
        .ok()
        .unwrap_or_else(|| panic!("insert_list"));
    run.write_slot(slot, SlotValue::List(id))
        .ok()
        .unwrap_or_else(|| panic!("write_slot list"));
}

/// Inserts a source list and writes a 2-element `(source_id, cursor)`
/// iterator-state list into `slot`. Returns the source `ListId`.
/// Mirrors the new RP-016 compact cursor state used by ForEach/Reduce.
pub(crate) fn iterator_state_in_slot(
    run: &mut RunFrame,
    store: &mut ValueStore,
    slot: SlotIdx,
    items: Vec<SlotValue>,
    cursor: usize,
) -> vb_core::ids::ListId {
    let source_id = store
        .insert_list(items.into_boxed_slice())
        .ok()
        .unwrap_or_else(|| panic!("iterator_state_in_slot: insert_list source"));
    let state_id = store
        .insert_list(
            vec![
                SlotValue::I64(source_id.get() as i64),
                SlotValue::I64(cursor as i64),
            ]
            .into_boxed_slice(),
        )
        .ok()
        .unwrap_or_else(|| panic!("iterator_state_in_slot: insert_list state"));
    run.write_slot(slot, SlotValue::List(state_id))
        .ok()
        .unwrap_or_else(|| panic!("iterator_state_in_slot: write_slot"));
    source_id
}
