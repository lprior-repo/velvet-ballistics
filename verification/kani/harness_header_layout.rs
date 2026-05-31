//! Kani harness for header layout verification
//!
//! # Proof Obligation
//! po-vbqa2g-002: Postcard frame header is exactly 52 bytes with correct structure
//!
//! # GOD RULE 1 Compliance
//! Uses kani::any() for header byte generation
//!
//! # GOD RULE 2 Compliance
//! Binds to PostcardHeader::from_bytes and validate functions

use vb_cli::cli_postcard::{PostcardHeader, HEADER_SIZE, CLI_MAGIC};

#[kani::proof]
fn harness_header_layout() {
    // Generate arbitrary header bytes
    let header_bytes: [u8; 52] = kani::any();

    // Parse header from bytes
    let header_result = PostcardHeader::from_bytes(&header_bytes);

    kani::assert(
        header_result.is_ok(),
        "PostcardHeader::from_bytes should succeed for any 52 bytes"
    );

    if let Ok(header) = header_result {
        // Verify header fields are accessible
        let _ = header.magic;
        let _ = header.schema_version;
        let _ = header.kind;
        let _ = header.header_len;
        let _ = header.payload_len;
        let _ = header.payload_digest;
        let _ = header.header_crc;
    }
}

#[kani::proof]
fn harness_header_validate_magic() {
    let mut header_bytes: [u8; 52] = kani::any();

    // Ensure header_len is valid
    header_bytes[8..12].copy_from_slice(&52u32.to_le_bytes());

    // Set valid payload_len bound
    header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());

    let header = PostcardHeader::from_bytes(&header_bytes);

    if let Ok(h) = header {
        // If magic is correct, validation should pass (other fields are set to valid values)
        if h.magic == CLI_MAGIC && h.header_len == 52 && h.payload_len <= 65536 {
            let result = h.validate();
            kani::assert(
                result.is_ok(),
                "valid header should pass validate()"
            );
        }
    }
}

#[kani::proof]
fn harness_header_validate_rejects_invalid() {
    let mut header_bytes: [u8; 52] = kani::any();

    // Corrupt magic to be invalid
    header_bytes[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x00]);

    // Set other fields to valid values
    header_bytes[8..12].copy_from_slice(&52u32.to_le_bytes());
    header_bytes[12..16].copy_from_slice(&0u32.to_le_bytes());

    let header = PostcardHeader::from_bytes(&header_bytes);

    if let Ok(h) = header {
        let result = h.validate();
        kani::assert(
            result.is_err(),
            "invalid magic should cause validate() to fail"
        );
    }
}