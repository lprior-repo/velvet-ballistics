#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_storage::recovery::hydrate::hydrate_dimensions_positive;

fuzz_target!(|data: &[u8]| {
    let step = data.first().copied().map_or(1, u16::from).saturating_add(1);
    let slot = data.get(1).copied().map_or(1, u16::from).saturating_add(1);
    assert!(hydrate_dimensions_positive(step, slot));
});
