// Fuzz target: fuzz_collect_lowering.rs
// Bead: vb-njib
// PO: ps-01 (fuzzing for collect lowering)
// Verifier: cargo-fuzz
// Command: cargo fuzz run fuzz_collect_lowering
//
// Fuzzing strategy:
// - Generate arbitrary slot and step indices plus limit and page_size values
// - Test that lower_collect handles all inputs without panicking
// - Cover boundary values: limit=0, page_size=0, page_size>limit, max u32
//
// Stage-split harness for the collect (gather) lowering primitive.

#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_compile::{SlotCompiler, lower_collect};
use vb_core::ids::{SlotIdx, StepIdx};

/// Fuzz target for lower_collect panic-freedom.
///
/// Generates arbitrary slot and step indices plus pagination parameters
/// to test boundary issues around pagination semantics.
fn fuzz_collect_lowering(step_ids: (u16, u16, u16), source_raw: u16, limit: u32, page_size: u32) {
    let (id_raw, body_raw, done_raw) = step_ids;

    let id = StepIdx::new(id_raw);
    let source = SlotIdx::new(source_raw);
    let body = StepIdx::new(body_raw);
    let done = StepIdx::new(done_raw);

    let mut builder = SlotCompiler::new();
    let _result = lower_collect(id, source, limit, page_size, body, done, &mut builder);
    // Crash-only oracle: any panic in lowering is a bug.
}

fuzz_target!(|data: ((u16, u16, u16), u16, u32, u32)| {
    let (step_ids, source_raw, limit, page_size) = data;
    fuzz_collect_lowering(step_ids, source_raw, limit, page_size);
});
