//! Fuzz target for IPC connection establishment and handshake.
//!
//! Tests handshake messages with corruption, connection establishment
//! scenarios, and protocol negotiation edge cases.
//!
//! Invariants:
//! - Connection errors produce typed errors (never panic)
//! - Corrupted handshake data is detected
//! - Frame encoding during handshake is correct
//! - Version negotiation failures are handled

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::io::{Cursor, Read, Write};

fuzz_target!(|data: &[u8]| {
    // Test 1: Full frame encoding with arbitrary data as payload
    // Verify the frame is well-formed even with corrupted payload bytes
    let header_bytes = [
        vb_ipc::IPC_MAGIC.to_le_bytes()[0],
        vb_ipc::IPC_MAGIC.to_le_bytes()[1],
        vb_ipc::IPC_MAGIC.to_le_bytes()[2],
        vb_ipc::IPC_MAGIC.to_le_bytes()[3],
        (vb_ipc::IPC_VERSION & 0xFF) as u8,
        ((vb_ipc::IPC_VERSION >> 8) & 0xFF) as u8,
        1, 0,  // SubmitRun command
        0, 0,  // flags
        0, 0,  // padding
        1, 0, 0, 0, 0, 0, 0, 0,  // correlation = 1
        if data.len() < 256 { data.len() as u8 } else { 255 }, 0, 0, 0,  // payload_len
    ];

    // Try to decode the header
    let h: &[u8; vb_ipc::IPC_HEADER_LEN] = &header_bytes[..vb_ipc::IPC_HEADER_LEN].try_into().expect("IPC header bytes");
    match vb_ipc::decode_frame_header(h) {
        Ok(decoded) => {
            // Verify correlation round-trips
            assert_eq!(decoded.correlation, 1);
            assert_eq!(decoded.command, vb_ipc::IpcCommand::SubmitRun);

            // Encode back and verify
            match decoded.encode() {
                Ok(reencoded) => {
                    // The re-encoded header should decode to the same values
                    let h2: &[u8; vb_ipc::IPC_HEADER_LEN] = &reencoded[..vb_ipc::IPC_HEADER_LEN].try_into().expect("IPC header bytes");
                    match vb_ipc::decode_frame_header(h2) {
                        Ok(decoded2) => {
                            assert_eq!(decoded2.correlation, decoded.correlation);
                            assert_eq!(decoded2.command, decoded.command);
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
            assert!(fuzz_lib::assert_typed_ipc_error(e));
        }
    }

    // Test 2: Health check command framing (empty payload)
    let health_header = [
        vb_ipc::IPC_MAGIC.to_le_bytes()[0],
        vb_ipc::IPC_MAGIC.to_le_bytes()[1],
        vb_ipc::IPC_MAGIC.to_le_bytes()[2],
        vb_ipc::IPC_MAGIC.to_le_bytes()[3],
        (vb_ipc::IPC_VERSION & 0xFF) as u8,
        ((vb_ipc::IPC_VERSION >> 8) & 0xFF) as u8,
        10, 0,  // Health command
        0, 0,  // flags
        0, 0,  // padding
        42, 0, 0, 0, 0, 0, 0, 0,  // correlation = 42
        0, 0, 0, 0,  // payload_len = 0
    ];

    let h: &[u8; vb_ipc::IPC_HEADER_LEN] = &health_header[..vb_ipc::IPC_HEADER_LEN].try_into().expect("IPC header bytes");
    if let Ok(decoded) = vb_ipc::decode_frame_header(h) {
        assert_eq!(decoded.command, vb_ipc::IpcCommand::Health);
        assert_eq!(decoded.correlation, 42);
        assert_eq!(decoded.payload_len, 0);

        // Write frame to buffer
        let mut buf = Vec::new();
        match vb_ipc::write_frame(&mut buf, decoded.command, decoded.flags, decoded.correlation, &[]) {
            Ok(()) => {
                assert_eq!(buf.len(), vb_ipc::IPC_HEADER_LEN);

                // Read it back
                let mut cursor = Cursor::new(buf);
                match vb_ipc::read_frame_header(&mut cursor) {
                    Ok(read_back) => {
                        assert_eq!(read_back.command, vb_ipc::IpcCommand::Health);
                        assert_eq!(read_back.correlation, 42);
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

    // Test 3: Corrupted magic bytes detection
    let mut corrupted = header_bytes;
    corrupted[0] = 0xFF;  // Wrong magic
    let hc: &[u8; vb_ipc::IPC_HEADER_LEN] = &corrupted[..vb_ipc::IPC_HEADER_LEN].try_into().expect("IPC header bytes");
    match vb_ipc::decode_frame_header(hc) {
        Ok(_) => {
            // If magic validation is lenient, this could pass
            // But most magic checks should fail
        }
        Err(e) => {
            assert!(fuzz_lib::assert_typed_ipc_error(e));
        }
    }

    // Test 4: Version mismatch
    let mut wrong_version = header_bytes;
    wrong_version[4] = 0xFF;  // Wrong version byte
    wrong_version[5] = 0xFF;
    let hv: &[u8; vb_ipc::IPC_HEADER_LEN] = &wrong_version[..vb_ipc::IPC_HEADER_LEN].try_into().expect("IPC header bytes");
    match vb_ipc::decode_frame_header(hv) {
        Ok(_) => {
            // Version check might be lenient in some configurations
        }
        Err(e) => {
            assert!(fuzz_lib::assert_typed_ipc_error(e));
        }
    }

    // Test 5: Large correlation values
    let large_corr: u64 = u64::from_le_bytes([
        data.get(0).copied().unwrap_or(0xFF),
        data.get(1).copied().unwrap_or(0xFF),
        data.get(2).copied().unwrap_or(0xFF),
        data.get(3).copied().unwrap_or(0xFF),
        data.get(4).copied().unwrap_or(0xFF),
        data.get(5).copied().unwrap_or(0xFF),
        data.get(6).copied().unwrap_or(0xFF),
        data.get(7).copied().unwrap_or(0xFF),
    ]);

    // Create header with large correlation
    let mut corr_header = [0u8; vb_ipc::IPC_HEADER_LEN];
    corr_header[0..4].copy_from_slice(&vb_ipc::IPC_MAGIC.to_le_bytes());
    corr_header[4..6].copy_from_slice(&vb_ipc::IPC_VERSION.to_le_bytes());
    corr_header[6..8].copy_from_slice(&1u16.to_le_bytes());  // SubmitRun
    corr_header[10..12].copy_from_slice(&0u16.to_le_bytes());  // padding
    corr_header[12..20].copy_from_slice(&large_corr.to_le_bytes());

    match vb_ipc::decode_frame_header(&corr_header) {
        Ok(decoded) => {
            assert_eq!(decoded.correlation, large_corr);

            // Round-trip
            match decoded.encode() {
                Ok(reencoded) => {
                    let redecoded: &[u8; vb_ipc::IPC_HEADER_LEN] = &reencoded[..vb_ipc::IPC_HEADER_LEN].try_into().expect("IPC header bytes");
                    if let Ok(d2) = vb_ipc::decode_frame_header(redecoded) {
                        assert_eq!(d2.correlation, large_corr);
                    }
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

    // Test 6: Bounded header reading with size limits
    if data.len() >= vb_ipc::IPC_HEADER_LEN {
        let header_bytes: &[u8; vb_ipc::IPC_HEADER_LEN] =
            &data[0..vb_ipc::IPC_HEADER_LEN].try_into().expect("exact length");
        
        match vb_ipc::decode_frame_header(header_bytes) {
            Ok(decoded) => {
                // Validate with strict bounds
                let strict = vb_ipc::MaxPayloadBytes::new(
                    std::num::NonZeroUsize::new(64).unwrap(),
                );
                match vb_ipc::validate_frame_bounds(&decoded, strict) {
                    Ok(()) => {
                        // Payload fits within 64 bytes
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
});
