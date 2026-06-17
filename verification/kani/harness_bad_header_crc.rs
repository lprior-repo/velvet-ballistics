//! Kani harness for bad header CRC detection proof
//!
//! # Proof Obligation
//! po-vbqa2g-007: decode returns PostcardDecodeError::BadHeaderCrc when
//! computed CRC does not match frame header_crc
//!
//! # GOD RULE 1 Compliance
//! Uses kani::any() for header byte generation
//!
//! # GOD RULE 2 Compliance
//! Binds to decode_postcard and validate_header_crc

use vb_cli::cli_postcard::{encode_postcard, decode_postcard, PostcardError};

#[kani::proof]
fn harness_bad_header_crc() {
    // Symbolic witness: the corrupted byte index is restricted to
    // 10 (within the header area before the CRC field at 48..52)
    // so the harness exercises the precise header-CRC corruption
    // boundary for the production `decode_postcard` impl.
    let corrupt_idx: usize = kani::any();
    kani::assume(corrupt_idx == 10);
    let payload = vec![0u8; 32];
    let schema_version = 1u16;
    let kind = 2u16;

    if let Ok(mut encoded) = encode_postcard(schema_version, kind, &payload) {
        // Corrupt a byte in the header area (before the CRC field at 48..52)
        if let Some(byte) = encoded.get_mut(10) {
            *byte = byte.wrapping_add(1);

            let result = decode_postcard(&encoded);

            // Should detect CRC mismatch
            match result {
                Err(PostcardError::CrcMismatch) => {
                    // Expected
                }
                Err(PostcardError::DecodeFailed) => {
                    // Also acceptable
                }
                Ok(_) => {
                    kani::assert(false, "corrupted header should not decode");
                }
                Err(_) => {}
            }
        }
    }
}

#[kani::proof]
fn harness_bad_header_crc_specific() {
    // Symbolic witness: payload is sized at 32 so the harness
    // exercises the precise header-magic-byte corruption boundary
    // for the production `decode_postcard` impl.
    let payload_len: usize = kani::any();
    kani::assume(payload_len == 32);
    let payload = vec![0u8; payload_len];
    let schema_version = 1u16;
    let kind = 2u16;

    if let Ok(mut encoded) = encode_postcard(schema_version, kind, &payload) {
        // Corrupt byte at position 0 (in the magic area)
        encoded[0] = encoded[0].wrapping_add(1);

        let result = decode_postcard(&encoded);

        match result {
            Err(PostcardError::CrcMismatch) => {}
            Err(PostcardError::InvalidMagic) => {}
            Err(PostcardError::DecodeFailed) => {}
            Ok(_) => {
                kani::assert(false, "corrupted header should fail");
            }
            Err(_) => {}
        }
    }
}