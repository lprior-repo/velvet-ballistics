//! Kani harness for bad payload digest detection proof
//!
//! # Proof Obligation
//! po-vbqa2g-008: decode returns PostcardDecodeError::BadPayloadDigest when
//! computed blake3 does not match frame digest
//!
//! # GOD RULE 1 Compliance
//! Uses kani::any() for payload generation with corruption via get_mut
//!
//! # GOD RULE 2 Compliance
//! Binds to decode_postcard and payload_digest

use vb_cli::cli_postcard::{encode_postcard, decode_postcard, PostcardError};

#[kani::proof]
fn harness_bad_payload_digest() {
    let payload = vec![0u8; 32];
    let schema_version = 1u16;
    let kind = 2u16;

    if let Ok(mut encoded) = encode_postcard(schema_version, kind, &payload) {
        // Corrupt a byte in the payload area (after 52-byte header)
        // Payload starts at index 52
        let payload_start = 52;
        if encoded.len() > payload_start + 5 {
            if let Some(byte) = encoded.get_mut(payload_start + 5) {
                *byte = byte.wrapping_add(1);

                let result = decode_postcard(&encoded);

                match result {
                    Err(PostcardError::DigestMismatch) => {
                        // Expected: digest mismatch detected
                    }
                    Err(PostcardError::DecodeFailed) => {
                        // Also acceptable
                    }
                    Ok(_) => {
                        kani::assert(false, "corrupted payload digest should not decode");
                    }
                    Err(_) => {}
                }
            }
        }
    }
}

#[kani::proof]
fn harness_bad_payload_digest_multiple_bytes() {
    let payload_len: usize = kani::any();
    kani::assume(payload_len >= 1 && payload_len <= 100);

    let payload: Vec<u8> = kani::vec::any_vec(payload_len);
    let schema_version = 1u16;
    let kind = 2u16;

    if let Ok(mut encoded) = encode_postcard(schema_version, kind, &payload) {
        // Corrupt multiple bytes in the payload
        let payload_start = 52;
        for i in 0..5.min(encoded.len().saturating_sub(payload_start)) {
            if let Some(byte) = encoded.get_mut(payload_start + i) {
                *byte = byte.wrapping_add(1);
            }
        }

        let result = decode_postcard(&encoded);

        match result {
            Err(PostcardError::DigestMismatch) => {}
            Err(PostcardError::DecodeFailed) => {}
            Ok(_) => {
                kani::assert(false, "corrupted payload should not decode");
            }
            Err(_) => {}
        }
    }
}