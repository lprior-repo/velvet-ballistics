// Fuzz target: fuzz_set_lowering.rs
// Bead: vb-njib
// PO: ps-01 (fuzzing for set lowering)
// Verifier: cargo-fuzz
// Command: cargo fuzz run fuzz_set_lowering
//
// Fuzzing strategy:
// - Generate arbitrary step/slot/const indices and optional next pointer
// - Test that lower_set handles all inputs without panicking
// - Cover with and without next pointer (terminal vs continuation)
//
// Stage-split harness for the set (save) lowering primitive.

#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_compile::lower_set;
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx};

/// Fuzz target for lower_set panic-freedom.
///
/// Generates arbitrary step/slot/const indices and an optional next pointer
/// to test boundary issues around continuation handling.
fn fuzz_set_lowering(id_raw: u16, output_raw: u16, value_raw: u16, next: Option<u16>) {
    let id = StepIdx::new(id_raw);
    let output = SlotIdx::new(output_raw);
    let value = ConstIdx::new(value_raw);
    let next_step = next.map(StepIdx::new);

    let _result = lower_set(id, output, value, next_step);
    // Crash-only oracle: any panic in lowering is a bug.
}

fuzz_target!(|data: (u16, u16, u16, Option<u16>)| {
    let (id_raw, output_raw, value_raw, next) = data;
    fuzz_set_lowering(id_raw, output_raw, value_raw, next);
});
