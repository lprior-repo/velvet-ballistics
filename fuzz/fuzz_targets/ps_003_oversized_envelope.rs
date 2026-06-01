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
//   vb_storage::admission::fuzz_access::reject_oversized_compiled_ir_value

#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_storage::JournalError;

fuzz_target!(|data: &[u8]| {
    let max = vb_storage::constants::MAX_COMPILED_IR_BYTES;
    let max_usize = usize::try_from(max).map_or(usize::MAX, |value| value);

    assert_len_contract(0, max, max_usize);
    assert_len_contract(max_usize, max, max_usize);
    if let Some(oversized) = max_usize.checked_add(1) {
        assert_len_contract(oversized, max, max_usize);
    }

    // Interpret the fuzz input as a length value
    if data.len() >= 8 {
        let mut len_bytes = [0_u8; 8];
        len_bytes.copy_from_slice(&data[..8]);
        let len = usize::from_le_bytes(len_bytes);

        assert_len_contract(len, max, max_usize);
    }
});

fn assert_len_contract(len: usize, max: u32, max_usize: usize) {
    let result = vb_storage::admission::fuzz_access::reject_oversized_compiled_ir_value(len);

    match result {
        Ok(()) => {
            assert!(
                len <= max_usize,
                "Ok must only return for valid sizes, got len={len}"
            );
        }
        Err(JournalError::PayloadTooLarge {
            len: actual_len,
            max: actual_max,
        }) => {
            assert!(
                len > max_usize,
                "PayloadTooLarge must only reject oversized inputs, got len={len}"
            );
            assert_eq!(actual_len, payload_len_for_error(len));
            assert_eq!(actual_max, max);
        }
        Err(error) => panic!("unexpected oversized-envelope error: {error}"),
    }
}

fn payload_len_for_error(len: usize) -> u32 {
    u32::try_from(len).map_or(u32::MAX, |value| value)
}
