#![no_main]

use boundary_inventory::validate_evidence_reference_bytes;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _result = validate_evidence_reference_bytes(data);
});
