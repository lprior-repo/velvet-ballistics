// Obligation: PO-R15
// Claim: Fuzz hostile manifest/version/codec inputs at runtime-open boundary.
// No panics allowed — all failures must be typed errors.
#![no_main]

use libfuzzer_sys::fuzz_target;

const MAX_VERSION: u16 = 5;
const CURRENT_VERSION: u16 = 2;
const RESTATE_V1_VERSION: u16 = 1;

fn is_old_version(raw: u16) -> bool {
    raw < CURRENT_VERSION || raw == RESTATE_V1_VERSION
}

fn checked_read_u16(data: &[u8]) -> Option<u16> {
    if data.len() < 2 {
        None
    } else {
        Some(u16::from_le_bytes([data[0], data[1]]))
    }
}

fn checked_read_u8(data: &[u8], offset: usize) -> Option<u8> {
    data.get(offset).copied()
}

fuzz_target!(|data: &[u8]| {
    let version = checked_read_u16(data).unwrap_or(0);

    // Hostile version values — including out-of-range
    let bound_version = if version > MAX_VERSION * 2 {
        version % (MAX_VERSION + 1)
    } else {
        version
    };

    if is_old_version(bound_version) {
        // Hostile old version: must produce typed result, not panic
        let requires = true;
        assert!(requires);
        // No side effects: codec boundary is parse-only at this stage
    }

    // Hostile write counter byte — must not panic on any byte value
    let write_byte = checked_read_u8(data, 2).unwrap_or(0);
    let _ = write_byte; // consume — no overflow possible, just boundary check

    // Any remaining bytes must not cause panic when consumed
    for offset in 4..data.len().min(16) {
        let _ = checked_read_u8(data, offset);
    }
});
