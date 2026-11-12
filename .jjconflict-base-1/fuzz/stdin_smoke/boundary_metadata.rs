#![no_main]

use boundary_inventory::{BoundaryCandidate, classify_boundary};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let marker = String::from_utf8_lossy(data);
    let candidate = BoundaryCandidate::new("crates/fuzzed/src/lib.rs", marker);
    let _result = classify_boundary(candidate);
});
