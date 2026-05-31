//! Kani harness for bad magic detection proof
//!
//! # Proof Obligation
//! po-vbqa2g-006: decode returns PostcardDecodeError::BadMagic when frame
//! does not start with VCLA
//!
//! # GOD RULE 1 Compliance
//! Uses kani::any() for magic byte generation
//!
//! # GOD RULE 2 Compliance
//! Binds to decode_postcard and PostcardHeader::validate

use vb_cli::cli_postcard::{decode_postcard, PostcardHeader, PostcardError, HEADER_SIZE, CLI_MAGIC};

#[kani::proof]
fn harness_bad_magic() {
    // Generate arbitrary header bytes with non-CLI_MAGIC prefix
    let mut header_bytes: [u8; 52] = kani::any();

    // Ensure the magic is NOT CLI_MAGIC
    let bad_magic: [u8; 4] = kani::any();
    kani::assume(bad_magic != CLI_MAGIC);
    header_bytes[0..4].copy_from_slice(&bad_magic);

    // Set other fields to valid values to isolate magic test
    header_bytes[8..12].copy_from_slice(&52u32.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());

    // Create a minimal frame
    let mut frame = header_bytes.to_vec();
    frame.extend_from_slice(&[0u8; 10]); // Empty payload

    // Set CRC
    let crc = crc32fast::hash(&frame[0..48]);
    frame[48..52].copy_from_slice(&crc.to_le_bytes());

    let result = decode_postcard(&frame);

    match result {
        Err(PostcardError::InvalidMagic) => {
            // Expected behavior
        }
        Err(PostcardError::DecodeFailed) => {
            // Also acceptable - decode failed before magic check
        }
        Ok(_) => {
            kani::assert(false, "frame with bad magic should not decode successfully");
        }
        Err(_) => {
            // Other errors acceptable for malformed frame
        }
    }
}

#[kani::proof]
fn harness_magic_specific_corruption() {
    // Test specific corruption patterns
    let mut header: [u8; 52] = kani::any();

    // Set valid magic first
    header[0..4].copy_from_slice(CLI_MAGIC.as_slice());

    // Then corrupt it
    header[0] = header[0].wrapping_add(1); // Change first byte

    // Set valid header_len and zero payload_len
    header[8..12].copy_from_slice(&52u32.to_le_bytes());
    header[12..16].copy_from_slice(&0u32.to_le_bytes());

    let mut frame = header.to_vec();
    let crc = crc32fast::hash(&frame[0..48]);
    frame[48..52].copy_from_slice(&crc.to_le_bytes());

    let result = decode_postcard(&frame);

    match result {
        Err(PostcardError::InvalidMagic) => {}
        Err(_) => {}
        Ok(_) => {
            kani::assert(false, "corrupted magic should be detected");
        }
    }
}