// Fuzz target: fuzz_finish_lowering.rs
// Bead: vb-njib
// PO: ps-01 (fuzzing for finish lowering)
// Verifier: cargo-fuzz
// Command: cargo fuzz run fuzz_finish_lowering
//
// Fuzzing strategy:
// - Generate arbitrary step and result slot indices
// - Test that lower_finish handles all inputs without panicking
// - Cover boundary slot values from 0 to u16::MAX
//
// Stage-split harness for the finish (terminal) lowering primitive.

#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_compile::{SlotCompiler, lower_finish};
use vb_core::ids::{SlotIdx, StepIdx};

/// Fuzz target for lower_finish panic-freedom.
///
/// Generates arbitrary step ids and result slot ids to test that
/// terminal lowering is panic-free across the full id space.
fn fuzz_finish_lowering(id_raw: u16, result_raw: u16) {
    let id = StepIdx::new(id_raw);
    let result = SlotIdx::new(result_raw);

    let mut builder = SlotCompiler::new();
    let _result = lower_finish(id, result, &mut builder);
    // Crash-only oracle: any panic in lowering is a bug.
}

fuzz_target!(|data: (u16, u16)| {
    let (id_raw, result_raw) = data;
    fuzz_finish_lowering(id_raw, result_raw);
});
