// Fuzz target: fuzz_wait_lowering.rs
// Bead: vb-njib
// PO: ps-01 (fuzzing for wait lowering)
// Verifier: cargo-fuzz
// Command: cargo fuzz run fuzz_wait_lowering
//
// Fuzzing strategy:
// - Generate arbitrary slot ids and shape discriminator (Until vs Event)
// - Test that lower_wait handles both WaitKind variants without panicking
// - Cover Event with and without timeout slot
//
// Stage-split harness for the wait lowering primitive.

#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_compile::{SlotCompiler, WaitKind, lower_wait};
use vb_core::ids::{SlotIdx, StepIdx};

/// Fuzz target for lower_wait panic-freedom.
///
/// Exercises both WaitKind::Until (deadline slot only) and
/// WaitKind::Event (event slot with optional timeout) shapes.
fn fuzz_wait_lowering(id_raw: u16, is_event: bool, primary_slot: u16, timeout_slot: Option<u16>) {
    let id = StepIdx::new(id_raw);

    let kind = if is_event {
        let event = SlotIdx::new(primary_slot);
        let timeout = timeout_slot.map(SlotIdx::new);
        WaitKind::Event { event, timeout }
    } else {
        let deadline = SlotIdx::new(primary_slot);
        WaitKind::Until { deadline }
    };

    let mut builder = SlotCompiler::new();
    let _result = lower_wait(id, kind, &mut builder);
    // Crash-only oracle: any panic in lowering is a bug.
}

fuzz_target!(|data: (u16, bool, u16, Option<u16>)| {
    let (id_raw, is_event, primary_slot, timeout_slot) = data;
    fuzz_wait_lowering(id_raw, is_event, primary_slot, timeout_slot);
});
