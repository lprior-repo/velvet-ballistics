// Fuzz target: fuzz_for_each_lowering.rs
// Bead: vb-njib
// PO: ps-01 (fuzzing for for_each lowering)
// Verifier: cargo-fuzz
// Command: cargo fuzz run fuzz_for_each_lowering
//
// Fuzzing strategy:
// - Generate arbitrary slot and step indices plus limit values
// - Test that lower_for_each handles all inputs without panicking
// - Cover limit values from 0 to u32::MAX to find boundary issues
//
// Stage-split harness for the for_each lowering primitive.

#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_compile::{SlotCompiler, lower_for_each};
use vb_core::ids::{SlotIdx, StepIdx};

/// Fuzz target for lower_for_each panic-freedom.
///
/// Generates arbitrary step ids, slot ids, and limits to test:
/// - zero limits (degenerate case, should not panic)
/// - arbitrary limits up to u32::MAX
/// - arbitrary step and slot index combinations
fn fuzz_for_each_lowering(
    step_ids: (u16, u16, u16),
    slot_ids: (u16, u16),
    limit: u32,
) {
    // Empty step/slot vectors are not possible with fixed tuples; no RemByZero
    // risk here. Only constraint is that step ids must remain distinct enough
    // that id+1 arithmetic (used elsewhere in lowering) does not collide.
    let (id_raw, body_raw, done_raw) = step_ids;
    let (input_raw, item_raw) = slot_ids;

    let id = StepIdx::new(id_raw);
    let input = SlotIdx::new(input_raw);
    let item_slot = SlotIdx::new(item_raw);
    let body = StepIdx::new(body_raw);
    let done = StepIdx::new(done_raw);

    let mut builder = SlotCompiler::new();
    let _result = lower_for_each(id, input, item_slot, limit, body, done, &mut builder);
    // Crash-only oracle: any panic in lowering is a bug.
}

fuzz_target!(|data: ((u16, u16, u16), (u16, u16), u32)| {
    let (step_ids, slot_ids, limit) = data;
    fuzz_for_each_lowering(step_ids, slot_ids, limit);
});
