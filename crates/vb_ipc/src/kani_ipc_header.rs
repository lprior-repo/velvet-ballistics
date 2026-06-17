#![forbid(unsafe_code)]
//! VB-IPC-DECODE-001 and VB-IPC-DECODE-003: IPC header decode verification
//!
//! Property: `IpcFrameHeader::decode` validates magic, version, command,
//! flags, reserved field, correlation, and payload_len correctly without panicking.
//!
//! This harness verifies panic-free header decoding for valid inputs.

use crate::{IPC_HEADER_LEN, IPC_MAGIC, IPC_VERSION, IpcCommand, IpcFrameHeader, MaxPayloadBytes};

/// VB-IPC-DECODE-001/003 H1: decode valid header succeeds
#[kani::proof]
fn kani_ipc_header_decode_valid() {
    let cmd_raw: u16 = kani::any();
    kani::assume(cmd_raw >= 1 && cmd_raw <= 11);
    let command = match IpcCommand::from_u16(cmd_raw) {
        Ok(c) => c,
        Err(_) => return,
    };
    let flags: u16 = kani::any();
    let correlation: u64 = kani::any();
    let payload_len: u32 = kani::any();
    kani::assume(payload_len <= MaxPayloadBytes::DEFAULT.get() as u32);

    let header = IpcFrameHeader::new(command, flags, correlation, payload_len);
    let encoded = header.encode();
    kani::assume(encoded.is_ok());
    let Ok(encoded) = encoded else { return };

    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);
    kani::assert(decoded.is_ok(, "assertion failed"), "valid header decodes successfully");

    if let Ok(h) = decoded {
        , "valid header decodes successfully");

    if let Ok(h) = decoded {
        kani::assert(h.command == command, "decoded command is preserved");
        kani::assert(h.flags == flags, "decoded flags are preserved");
        kani::assert(
            h.correlation == correlation,
            "decoded correlation is preserved",
        );
        kani::assert(
            h.payload_len == payload_len,
            "decoded payload length is preserved",
        );
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
    kani::assert(decoded.is_err(, "assertion failed"), "invalid magic should return error");
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
    kani::assert(decoded.is_err(, "assertion failed"), "unsupported version should return error");
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
    kani::assert(decoded.is_err(, "assertion failed"), "non-zero reserved should return error");
}

/// VB-IPC-DECODE-001/003 H5: decode with various valid commands
#[kani::proof]
#[kani::unwind(6)]
fn kani_ipc_header_decode_various_commands() {
    let cmd_raw: u16 = kani::any();
    kani::assume(cmd_raw >= 1 && cmd_raw <= 11);
    let command = match IpcCommand::from_u16(cmd_raw) {
        Ok(c) => c,
        Err(_) => return,
    };

    let header = IpcFrameHeader::new(command, 0, 0, 0);
    let encoded = header.encode();
    kani::assume(encoded.is_ok());
    let Ok(encoded) = encoded else { return };

    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);
    kani::assert(decoded.is_ok(, "assertion failed"), "command decodes successfully");
}

/// VB-IPC-DECODE-001/003 H6: decode preserves all header fields
#[kani::proof]
fn kani_ipc_header_preserves_all_fields() {
    let cmd_raw: u16 = kani::any();
    kani::assume(cmd_raw >= 1 && cmd_raw <= 11);
    let command = match IpcCommand::from_u16(cmd_raw) {
        Ok(c) => c,
        Err(_) => return,
    };
    let flags: u16 = kani::any();
    let correlation: u64 = kani::any();
    let payload_len: u32 = kani::any();
    kani::assume(payload_len <= MaxPayloadBytes::DEFAULT.get() as u32);

    let header = IpcFrameHeader::new(command, flags, correlation, payload_len);
    let encoded = header.encode();
    kani::assume(encoded.is_ok());
    let Ok(encoded) = encoded else { return };

    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);
    kani::assume(decoded.is_ok());
    let Ok(decoded) = decoded else { return };

    , "command decodes successfully");
}

/// VB-IPC-DECODE-001/003 H6: decode preserves all header fields
#[kani::proof]
fn kani_ipc_header_preserves_all_fields() {
    let cmd_raw: u16 = kani::any();
    kani::assume(cmd_raw >= 1 && cmd_raw <= 11);
    let command = match IpcCommand::from_u16(cmd_raw) {
        Ok(c) => c,
        Err(_) => return,
    };
    let flags: u16 = kani::any();
    let correlation: u64 = kani::any();
    let payload_len: u32 = kani::any();
    kani::assume(payload_len <= MaxPayloadBytes::DEFAULT.get() as u32);

    let header = IpcFrameHeader::new(command, flags, correlation, payload_len);
    let encoded = header.encode();
    kani::assume(encoded.is_ok());
    let Ok(encoded) = encoded else { return };

    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);
    kani::assume(decoded.is_ok());
    let Ok(decoded) = decoded else { return };

    kani::assert(decoded.command == command, "decoded command is preserved");
    decoded.command == command, "decoded command is preserved");
    kani::assert(decoded.flags == flags, "decoded flags are preserved");
    decoded.flags == flags, "decoded flags are preserved");
    kani::assert(
        decoded.correlation == correlation,
        "decoded correlation is preserved",
    );
    
        decoded.correlation == correlation,
        "decoded correlation is preserved",
    );
    kani::assert(
        decoded.payload_len == payload_len,
        "decoded payload length is preserved",
    );
}
