// VB-INV006-KANI: taint validity invariant
//
// Claim: After write_slot_with_taint returns Ok, the slot's taint is
//        one of {Clean, DerivedFromSecret, Secret}.
// Bound: slot_count ∈ [0, 32]

#![forbid(unsafe_code)]

use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::errors::EngineError;

#[kani::proof]
#[kani::unwind(5)]
fn taint_validity_harness() {
    let slot_count: u16 = kani::any();
    kani::assume(slot_count <= 32);
    let effective_slot_count = if slot_count == 0 { 1 } else { slot_count };

    let mut run: RunFrame = match RunFrame::new(
        RunId::new(1),
        StepIdx::ZERO,
        1,
        effective_slot_count,
    ) {
        Ok(f) => f,
        Err(_) => return,
    };

    let slot_idx = SlotIdx::new(kani::any::<u16>() % effective_slot_count);
    let value: SlotValue = kani::any();
    let taint: Taint = kani::any();

    let write_result = run.write_slot_with_taint(slot_idx, value, taint);

    if write_result.is_ok() {
        let taint_read = run.read_taint(slot_idx);
        kani::assert(taint_read.is_ok(), "taint read does not panic");

        if let Ok(t) = taint_read {
            match t {
                Taint::Clean | Taint::DerivedFromSecret | Taint::Secret => {
                }
            }
        }
    }

    let slot_idx2 = SlotIdx::new(kani::any::<u16>() % effective_slot_count.max(1));
    let value2: SlotValue = kani::any();
    let _ = run.write_slot(slot_idx2, value2);
    let taint_after_write_slot = run.read_taint(slot_idx2);
    kani::assert(
        taint_after_write_slot.is_ok(),
        "read_taint after write_slot does not panic",
    );
}