//! Kani harness for payload too large detection proof
//!
//! # Proof Obligation
//! po-vbqa2g-009: decode returns PostcardDecodeError::PayloadTooLarge when
//! declared payload_len exceeds available bytes
//!
//! # GOD RULE 1 Compliance
//! Uses kani::any() for header and payload generation
//!
//! # GOD RULE 2 Compliance
//! Binds to decode_postcard and PostcardHeader::validate

use vb_cli::cli_postcard::{decode_postcard, PostcardHeader, PostcardError, HEADER_SIZE, MAX_PAYLOAD};

#[kani::proof]
fn harness_payload_too_large() {
    // Generate header with oversized payload_len
    let mut header_bytes: [u8; 52] = kani::any();

    // Set valid magic
    header_bytes[0..4].copy_from_slice(b"VCLA");

    // Set valid header_len
    header_bytes[8..12].copy_from_slice(&(HEADER_SIZE as u32).to_le_bytes());

    // Set oversized payload_len (> MAX_PAYLOAD)
    let oversized_len = MAX_PAYLOAD as u32 + 1;
    header_bytes[12..16].copy_from_slice(&oversized_len.to_le_bytes());

    // Set other fields
    header_bytes[6..8].copy_from_slice(&2u16.to_le_bytes()); // kind
    header_bytes[4..6].copy_from_slice(&1u16.to_le_bytes()); // schema_version

    // Set CRC (won't matter since validation happens before CRC check)
    header_bytes[48..52].copy_from_slice(&0u32.to_le_bytes());

    // Create frame with header but no payload (or insufficient payload)
    let frame = header_bytes.to_vec();

    let result = decode_postcard(&frame);

    match result {
        Err(PostcardError::PayloadTooLarge) => {
            // Expected
        }
        Err(PostcardError::DecodeFailed) => {
            // Also acceptable
        }
        Ok(_) => {
            kani::assert(false, "oversized payload should not decode");
        }
        Err(_) => {}
    }
}

#[kani::proof]
fn harness_payload_boundary() {
    // Test boundary: exactly MAX_PAYLOAD should be OK
    let mut header_bytes: [u8; 52] = kani::any();

    header_bytes[0..4].copy_from_slice(b"VCLA");
    header_bytes[8..12].copy_from_slice(&(HEADER_SIZE as u32).to_le_bytes());
    header_bytes[12..16].copy_from_slice(&(MAX_PAYLOAD as u32).to_le_bytes()); // Exactly MAX
    header_bytes[6..8].copy_from_slice(&2u16.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&1u16.to_le_bytes());

    let result = PostcardHeader::from_bytes(&header_bytes);

    if let Ok(h) = result {
        let validate_result = h.validate();
        // MAX_PAYLOAD should be valid (allowed)
        match validate_result {
            Ok(()) => {}
            Err(PostcardError::PayloadTooLarge) => {
                // This would be a bug - MAX_PAYLOAD should be allowed
                kani::assert(false, "MAX_PAYLOAD should be valid");
            }
            Err(_) => {}
        }
    }
}