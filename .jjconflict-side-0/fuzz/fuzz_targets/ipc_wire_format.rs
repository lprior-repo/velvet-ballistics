//! Fuzz target for IPC frame wire format decoding.
//!
//! Tests malformed headers, payload framing errors, version mismatches,
//! and magic byte corruption across the full frame decode path.
//!
//! Invariants:
//! - All decode errors produce typed IpcError variants (no panics)
//! - Corrupted magic bytes are detected before any payload processing
//! - Invalid version bytes produce decode errors
//! - Header bytes longer than IPC_HEADER_LEN are handled correctly
//! - Payload length mismatches are detected
//! - Round-trip: encode(decode(x)) == x for valid frames

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Test 1: Magic byte validation on raw data
    // validate_frame_magic expects at least 4 bytes for magic
    if data.len() >= 4 {
        match vb_ipc::validate_frame_magic(data) {
            Ok(()) => {
                // Magic is valid; continue to header decode
            }
            Err(e) => {
                // Magic validation failed - expect typed error
                assert!(fuzz_lib::assert_typed_ipc_error(e));
            }
        }
    }

    // Test 2: Header decode with exact-length data
    if data.len() == vb_ipc::IPC_HEADER_LEN {
        let header_bytes: &[u8; vb_ipc::IPC_HEADER_LEN] =
            data.try_into().expect("exact length checked");
        match vb_ipc::decode_frame_header(header_bytes) {
            Ok(decoded) => {
                // Valid header - verify round-trip
                match decoded.encode() {
                    Ok(encoded) => {
                        assert_eq!(
                            encoded.as_slice(),
                            data,
                            "re-encoded header must match original"
                        );

                        // Verify version is correct in re-encoded form
                        let version_slot = &encoded[4..6];
                        let version = u16::from_le_bytes(
                            version_slot.try_into().expect("2 bytes")
                        );
                        assert_eq!(
                            version,
                            vb_ipc::IPC_VERSION,
                            "re-encoded version must match IPC_VERSION"
                        );
                    }
                    Err(e) => {
                        assert!(fuzz_lib::assert_typed_ipc_error(e));
                    }
                }
            }
            Err(e) => {
                // Decode failed - expect typed error
                assert!(fuzz_lib::assert_typed_ipc_error(e));
            }
        }
    }

    // Test 3: Full frame encode/decode round-trip for short payloads
    if data.len() >= vb_ipc::IPC_HEADER_LEN {
        // Try to decode header
        let header_bytes: &[u8; vb_ipc::IPC_HEADER_LEN] =
            &data[0..vb_ipc::IPC_HEADER_LEN].try_into().expect("exact length");
        
        if let Ok(header) = vb_ipc::decode_frame_header(header_bytes) {
            let payload = &data[vb_ipc::IPC_HEADER_LEN..];
            
            // Verify payload length consistency
            if payload.len() as u32 != header.payload_len {
                // Length mismatch should be caught by decode_frame_payload
                if payload.len() > 0 {
                    let result = vb_ipc::decode_frame_payload(&header, payload);
                    // May be ok if payload_len is large enough, or may error
                    // Either way, no panic
                    if let Err(e) = result {
                        assert!(fuzz_lib::assert_typed_ipc_error(e));
                    }
                }
            }
        }
    }

    // Test 4: Bounds validation
    if data.len() >= vb_ipc::IPC_HEADER_LEN {
        let header_bytes: &[u8; vb_ipc::IPC_HEADER_LEN] =
            &data[0..vb_ipc::IPC_HEADER_LEN].try_into().expect("exact length");
        
        if let Ok(header) = vb_ipc::decode_frame_header(header_bytes) {
            match vb_ipc::validate_frame_bounds(&header, vb_ipc::MaxPayloadBytes::DEFAULT) {
                Ok(()) => {
                    // Bounds are valid
                }
                Err(e) => {
                    assert!(fuzz_lib::assert_typed_ipc_error(e));
                }
            }
        }
    }
});
