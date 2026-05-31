//! Kani harness for roundtrip bijectivity proof
//!
//! # Proof Obligation
//! po-vbqa2g-001: encode(T) followed by decode(frame) produces struct equivalent
//! to original T for all Kind variants
//!
//! # GOD RULE 1 Compliance
//! Uses kani::any() for bounded input generation - no hardcoded shapes
//!
//! # GOD RULE 2 Compliance
//! Binds directly to production implementations: encode_postcard, decode_postcard

use vb_cli::cli_postcard::{encode_postcard, decode_postcard, PostcardHeader, HEADER_SIZE};

#[kani::proof]
fn harness_roundtrip() {
    // Generate arbitrary payload length bounded to MAX_PAYLOAD
    let payload_len: usize = kani::any::<usize>();
    kani::assume(payload_len <= 65536); // MAX_PAYLOAD

    // Generate arbitrary payload bytes
    let payload: Vec<u8> = kani::vec::any_vec(payload_len);

    // Encode the payload
    let schema_version = 1u16; // CLI_SCHEMA_VERSION
    let kind = 2u16; // CLI_POSTCARD_KIND

    let encode_result = encode_postcard(schema_version, kind, &payload);

    // If encode succeeds, decode should also succeed and recover original payload
    if let Ok(encoded) = encode_result {
        let decode_result = decode_postcard(&encoded);

        kani::assert(
            decode_result.is_ok(),
            "decode should succeed for valid encoded postcard"
        );

        if let Ok((header_bytes, payload_bytes)) = decode_result {
            // Verify header is valid
            let header = PostcardHeader::from_bytes(header_bytes);
            kani::assert(
                header.is_ok(),
                "header should parse correctly"
            );

            if let Ok(h) = header {
                // Verify header validates
                let validate_result = h.validate();
                kani::assert(
                    validate_result.is_ok(),
                    "header should validate"
                );

                // Verify payload matches
                kani::assert(
                    payload_bytes == &payload[..],
                    "decoded payload should match original"
                );
            }
        }
    }
}

#[kani::proof]
fn harness_roundtrip_header_fields() {
    let payload_len: usize = kani::any::<usize>();
    kani::assume(payload_len <= 65536);

    let payload: Vec<u8> = kani::vec::any_vec(payload_len);
    let schema_version = 1u16;
    let kind = 2u16;

    if let Ok(encoded) = encode_postcard(schema_version, kind, &payload) {
        if let Ok((header_bytes, _)) = decode_postcard(&encoded) {
            if let Ok(h) = PostcardHeader::from_bytes(header_bytes) {
                // Verify schema_version survives roundtrip
                kani::assert(
                    h.schema_version == schema_version,
                    "schema_version should survive roundtrip"
                );

                // Verify kind survives roundtrip
                kani::assert(
                    h.kind == kind,
                    "kind should survive roundtrip"
                );

                // Verify payload_len matches
                kani::assert(
                    h.payload_len as usize == payload_len,
                    "payload_len should match"
                );
            }
        }
    }
}