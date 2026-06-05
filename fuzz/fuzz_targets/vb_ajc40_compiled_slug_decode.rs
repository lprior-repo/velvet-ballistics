#![no_main]
#![forbid(unsafe_code)]
//! libFuzzer target for vb-ajc40 PO-005.

use libfuzzer_sys::fuzz_target;
use vb_core::workflow::compiled_slug::from_bytes_compiled_slugs;

fuzz_target!(|data: &[u8]| {
    let budget = budget_from_prefix(data);
    let _ = from_bytes_compiled_slugs(data, budget);
});

fn budget_from_prefix(data: &[u8]) -> u64 {
    let mut bytes = [0_u8; 8];
    for (slot, value) in bytes.iter_mut().zip(data.iter().copied()) {
        *slot = value;
    }
    u64::from_le_bytes(bytes)
}
