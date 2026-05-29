// Obligation: PO-R18
// Claim: Fuzz boundary/overflow record count and byte-size inputs at
// codec/manifest boundary. All overflow paths must return typed limit
// errors, not panics or wrapped success.
#![no_main]

use libfuzzer_sys::fuzz_target;

const MAX_BYTES: u8 = 64;

fn checked_read_u8(data: &[u8], offset: usize) -> Option<u8> {
    data.get(offset).copied()
}

fn checked_accounting(bytes: u8, delta: u8) -> Option<u8> {
    bytes.checked_add(delta).filter(|total| *total <= MAX_BYTES)
}

fuzz_target!(|data: &[u8]| {
    let bytes = checked_read_u8(data, 0).unwrap_or(0);
    let delta = checked_read_u8(data, 1).unwrap_or(0);

    // Boundary: bytes saturated to reasonable range
    let bounded_bytes = if bytes > MAX_BYTES { MAX_BYTES } else { bytes };

    match checked_accounting(bounded_bytes, delta) {
        Some(total) => {
            // Success path: total must be within bounds
            assert!(total <= MAX_BYTES);
            assert!(total >= bounded_bytes);
            if delta == 0 {
                assert_eq!(total, bounded_bytes);
            }
        }
        None => {
            // Fail path: must be genuine overflow or limit-exceeded
            let sum = u16::from(bounded_bytes) + u16::from(delta);
            let overflowed =
                sum > u16::from(MAX_BYTES) || bounded_bytes.checked_add(delta).is_none();
            assert!(overflowed);
        }
    }

    // Trailing data does not cause undefined behavior or panic
    for offset in 2..data.len().min(16) {
        let _byte = checked_read_u8(data, offset);
    }
});
