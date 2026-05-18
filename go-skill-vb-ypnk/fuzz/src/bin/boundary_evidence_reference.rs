#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_boundary_inventory::boundary_inventory::validate_evidence_reference_bytes;

fuzz_target!(|data: &[u8]| {
    let _result = validate_evidence_reference_bytes(data);
});
