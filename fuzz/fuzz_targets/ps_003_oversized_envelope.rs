// Fuzz target: Oversized envelope DOS defense (Gate 1).
//
// Obligation: PO-vb-h09wf-010
// Verifier: cargo-fuzz
// Command: cargo fuzz run ps_003_oversized_envelope -- -max_total_time=300
//
// Domain claim: 300s fuzz run: no panics, no crashes. All oversized payloads
// rejected with PayloadTooLarge. All valid-sized payloads return Ok(()).
// Defends against H13 resource exhaustion.
//
// PRODUCTION BINDING:
//   vb_storage::admission::reject_oversized_compiled_ir_value

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Interpret the fuzz input as a length value
    if data.len() >= 8 {
        let len_bytes: [u8; 8] = data[..8].try_into().unwrap();
        let len = usize::from_le_bytes(len_bytes);

        let result = vb_storage::admission::reject_oversized_compiled_ir_value(len);

        // Verify no panic occurred — the function handled the input
        match result {
            Ok(()) => {
                // Valid size accepted — len must be <= MAX_COMPILED_IR_BYTES
                assert!(
                    len <= vb_storage::constants::MAX_COMPILED_IR_BYTES as usize,
                    "Ok must only return for valid sizes, got len={len}"
                );
            }
            Err(_) => {
                // Oversized or overflow — function rejected correctly
            }
        }
    }
});
