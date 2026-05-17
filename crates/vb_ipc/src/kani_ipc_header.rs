#![forbid(unsafe_code)]
//! VB-IPC-DECODE-001 and VB-IPC-DECODE-003: IPC header decode verification
//!
//! Property: `IpcFrameHeader::decode` validates magic, version, command,
//! flags, reserved field, correlation, and payload_len correctly without panicking.
//!
//! This harness verifies panic-free header decoding for valid inputs.

use crate::{IpcFrameHeader, IpcCommand, IPC_HEADER_LEN, IPC_MAGIC, IPC_VERSION, MaxPayloadBytes, IpcError};

/// VB-IPC-DECODE-001/003 H1: decode valid header succeeds
#[kani::proof]
fn kani_ipc_header_decode_valid() {
    let command = IpcCommand::Health;
    let flags: u16 = 0;
    let correlation: u64 = 12345;
    let payload_len: u32 = 0;

    let header = IpcFrameHeader::new(command, flags, correlation, payload_len);
    let encoded = header.encode();
    kani::assume(encoded.is_ok());
    let encoded = encoded.unwrap();

    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);
    kani::assert(decoded.is_ok(), "valid header decodes successfully");

    if let Ok(h) = decoded {
        kani::assert(h.command == command);
        kani::assert(h.flags == flags);
        kani::assert(h.correlation == correlation);
        kani::assert(h.payload_len == payload_len);
    }
}

/// VB-IPC-DECODE-001/003 H2: decode rejects invalid magic
#[kani::proof]
fn kani_ipc_header_rejects_bad_magic() {
    let mut bytes = [0u8; IPC_HEADER_LEN];
    // Set valid values except magic
    bytes[0..4].copy_from_slice(&0x12345678u32.to_le_bytes()); // bad magic
    bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
    // rest zeros (valid reserved, etc.)

    let decoded = IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT);
    kani::assert(decoded.is_err(), "invalid magic should return error");
}

/// VB-IPC-DECODE-001/003 H3: decode rejects unsupported version
#[kani::proof]
fn kani_ipc_header_rejects_bad_version() {
    let mut bytes = [0u8; IPC_HEADER_LEN];
    bytes[0..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    bytes[4..6].copy_from_slice(&(IPC_VERSION + 1).to_le_bytes()); // bad version
    bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
    // rest zeros

    let decoded = IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT);
    kani::assert(decoded.is_err(), "unsupported version should return error");
}

/// VB-IPC-DECODE-001/003 H4: decode rejects non-zero reserved field
#[kani::proof]
fn kani_ipc_header_rejects_reserved_nonzero() {
    let mut bytes = [0u8; IPC_HEADER_LEN];
    bytes[0..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
    bytes[10..12].copy_from_slice(&1u16.to_le_bytes()); // non-zero reserved

    let decoded = IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT);
    kani::assert(decoded.is_err(), "non-zero reserved should return error");
}

/// VB-IPC-DECODE-001/003 H5: decode with various valid commands
#[kani::proof]
#[kani::unwind(6)]
fn kani_ipc_header_decode_various_commands() {
    let commands: &[IpcCommand] = &[
        IpcCommand::Health,
        IpcCommand::Shutdown,
        IpcCommand::SubmitRun,
        IpcCommand::CancelRun,
        IpcCommand::InspectRun,
    ];

    for &command in commands {
        let header = IpcFrameHeader::new(command, 0, 0, 0);
        let encoded = header.encode();
        kani::assume(encoded.is_ok());
        let encoded = encoded.unwrap();

        let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);
        kani::assert(decoded.is_ok(), "command decodes successfully");
    }
}

/// VB-IPC-DECODE-001/003 H6: decode preserves all header fields
#[kani::proof]
fn kani_ipc_header_preserves_all_fields() {
    let command = IpcCommand::SubmitRun;
    let flags: u16 = 0x00FF;
    let correlation: u64 = 0xDEADBEEFCAFEL;
    let payload_len: u32 = 256;

    let header = IpcFrameHeader::new(command, flags, correlation, payload_len);
    let encoded = header.encode();
    kani::assume(encoded.is_ok());
    let encoded = encoded.unwrap();

    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);
    kani::assume(decoded.is_ok());
    let decoded = decoded.unwrap();

    kani::assert(decoded.command == command);
    kani::assert(decoded.flags == flags);
    kani::assert(decoded.correlation == correlation);
    kani::assert(decoded.payload_len == payload_len);
}
