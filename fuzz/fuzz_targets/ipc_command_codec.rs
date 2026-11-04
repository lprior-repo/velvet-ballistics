//! Fuzz target for IPC command encoding/decoding property tests.
//!
//! Tests postcard encoding/decoding of IpcPayload variants, including:
//! - SubmitRunPayload round-trips
//! - IpcActionOutputPayload round-trips
//! - Corrupted payload bytes produce typed errors
//! - Payload size bounds are enforced
//! - Unknown command IDs are handled
//! - Flag validation for different command types
//!
//! Invariants:
//! - encode(decode(x)) == x for valid payloads
//! - Corrupted bytes produce IpcError (never panic)
//! - Postcard decoding of truncated data produces errors
//! - Max payload size is enforced during encoding

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Test 1: Decode arbitrary bytes as IpcPayload
    // Most byte sequences are not valid postcard-encoded payloads
    let bounded = vb_ipc::BoundedPayload::new(
        bytes::Bytes::from(data.to_vec()),
        vb_ipc::MaxPayloadBytes::DEFAULT,
    );
    match bounded {
        Ok(bp) => {
            match vb_ipc::codec::decode_payload(&bp) {
                Ok(decoded) => {
                    // Valid payload - verify round-trip through decode/encode
                    // Note: we cannot compare against original 'data' because
                    // random fuzz data may have trailing garbage after the valid payload.
                    // Instead, verify encode(decode(x)) == x by re-encoding and re-decoding.
                    match vb_ipc::codec::encode_payload(&decoded, vb_ipc::MaxPayloadBytes::DEFAULT) {
                        Ok(encoded) => {
                            // Re-decode the re-encoded payload to verify consistency
                            match vb_ipc::codec::decode_payload(&encoded) {
                                Ok(re_decoded) => {
                                    // Both decode paths produced identical values
                                    assert_eq!(
                                        decoded,
                                        re_decoded,
                                        "re-decoded payload must match original decoded value"
                                    );
                                }
                                Err(e) => {
                                    assert!(fuzz_lib::assert_typed_ipc_error(e));
                                }
                            }
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
        Err(e) => {
            // Bounded payload creation failed
            assert!(fuzz_lib::assert_typed_ipc_error(e));
        }
    }

    // Test 2: Command ID round-trip
    // Command IDs are u16 values; test serialization of each
    if !data.is_empty() {
        let cmd_id = u16::from_le_bytes([data[0], if data.len() > 1 { data[1] } else { 0 }]);
        match vb_ipc::IpcCommand::from_u16(cmd_id) {
            Ok(cmd) => {
                // Round-trip: as_u16 then from_u16
                let back = cmd.as_u16();
                let cmd2 = vb_ipc::IpcCommand::from_u16(back);
                assert!(cmd2.is_ok(), "to_u16/from_u16 round-trip failed for {:?}", cmd);
            }
            Err(e) => {
                // Unknown command IDs produce errors
                assert!(fuzz_lib::assert_typed_ipc_error(e));
            }
        }
    }

    // Test 3: Command flag validation
    if data.len() >= 4 {
        let cmd_id = u16::from_le_bytes([data[0], data[1]]);
        let flags = u16::from_le_bytes([data[2], data[3]]);
        
        if let Ok(cmd) = vb_ipc::IpcCommand::from_u16(cmd_id) {
            match vb_ipc::CommandFlags::validate(cmd, flags) {
                Ok(cf) => {
                    // Valid flags - verify raw value
                    assert_eq!(cf.as_u16(), flags);
                    assert_eq!(cf.command(), cmd);
                }
                Err(e) => {
                    // Invalid flags for command
                    assert!(fuzz_lib::assert_typed_ipc_error(e));
                }
            }
        }
    }

    // Test 4: Bounded payload size enforcement
    // Create a bounded payload and check it's enforced
    let bounded = vb_ipc::BoundedPayload::new(
        bytes::Bytes::from(data.to_vec()),
        vb_ipc::MaxPayloadBytes::DEFAULT,
    );
    match bounded {
        Ok(bp) => {
            assert!(bp.bytes().len() <= 1024);
        }
        Err(e) => {
            assert!(fuzz_lib::assert_typed_ipc_error(e));
        }
    }

    // Test 5: IpcFrame decode with corrupted header
    if data.len() >= vb_ipc::IPC_HEADER_LEN {
        let header_bytes: &[u8; vb_ipc::IPC_HEADER_LEN] =
            &data[0..vb_ipc::IPC_HEADER_LEN].try_into().expect("exact length");
        
        let payload_bytes = if data.len() > vb_ipc::IPC_HEADER_LEN {
            bytes::Bytes::from(data[vb_ipc::IPC_HEADER_LEN..].to_vec())
        } else {
            bytes::Bytes::new()
        };

        match vb_ipc::decode_frame(header_bytes, payload_bytes, vb_ipc::MaxPayloadBytes::DEFAULT) {
            Ok(frame) => {
                // Successfully decoded - verify payload bounds
                assert!(frame.payload().bytes().len() <= frame.header().payload_len as usize);
            }
            Err(e) => {
                assert!(fuzz_lib::assert_typed_ipc_error(e));
            }
        }
    }
});
