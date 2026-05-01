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
