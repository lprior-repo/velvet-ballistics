#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_boundary_inventory::boundary_inventory::{BoundaryCandidate, classify_boundary};

fuzz_target!(|data: &[u8]| {
    let marker = String::from_utf8_lossy(data);
    let candidate = BoundaryCandidate::new("crates/fuzzed/src/lib.rs", marker);
    let _result = classify_boundary(candidate);
});
