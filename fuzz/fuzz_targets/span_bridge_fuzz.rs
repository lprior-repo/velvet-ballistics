//! Fuzz target for span_bridge (clamp_u32 + span_from_source_span).
//!
//! Verifies that usize→u32 clamping is panic-free and that the span bridge
//! produces valid output for all inputs (contract C9.3).
//!
//! Corpus seeds are maintained in `fuzz/corpus/span_bridge_fuzz/`.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_lib::fuzz_span_bridge(data);
});
