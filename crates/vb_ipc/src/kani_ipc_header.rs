#![forbid(unsafe_code)]
//! VB-IPC-DECODE-001 and VB-IPC-DECODE-003: IPC header decode verification
//! Replacement production-bound coverage for bead `vb-dzibx` also maps to the
//! `proof-obligations.planned.jsonl` IPC gap entries with obligation id
//! `P-EMPTY-BODY` (`ipc_capacity_bounds.rs`, `ipc_runtime_transitions.rs`, and
//! `ipc_strict_admission.rs`) plus replacement obligation `RPO-IPC-001`. The
//! harnesses below call production constants, types, and functions directly;
//! they do not define a mirror header model.
//!
//! Property: `IpcFrameHeader::decode` validates magic, version, command,
//! flags, reserved field, correlation, and payload_len correctly without panicking.
//!
//! This harness verifies panic-free header decoding for valid inputs.

use crate::{
    IPC_HEADER_LEN, IPC_MAGIC, IPC_VERSION, IpcCommand, IpcError, IpcFrameHeader, MaxPayloadBytes,
    decode_frame_header, encode_frame, validate_frame_magic,
};

fn any_production_command() -> IpcCommand {
    let raw_command: u16 = kani::any();
    let parsed = IpcCommand::from_u16(raw_command);
    kani::assert(
        parsed.is_ok(),
        "IpcCommand::from_u16 is total for u16 wire ids",
    );
    match parsed {
        Ok(command) => command,
        Err(_) => IpcCommand::UnknownCommand(raw_command),
    }
}

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
    kani::assert(decoded.is_ok(), "valid header decodes successfully");

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
    kani::assert(decoded.is_ok(), "command decodes successfully");
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
    kani::assert(decoded.flags == flags, "decoded flags are preserved");
    kani::assert(
        decoded.correlation == correlation,
        "decoded correlation is preserved",
    );
    kani::assert(
        decoded.payload_len == payload_len,
        "decoded payload length is preserved",
    );
}

/// RPO-IPC-001 + `P-EMPTY-BODY` IPC gap replacement:
/// production `IpcFrameHeader` encoding/decoding preserves arbitrary bounded
/// header fields and the actual fixed VBLT/v1/24-byte wire layout.
#[kani::proof]
fn vb_dzibx_ipc_header_codec_roundtrip() {
    let command = any_production_command();
    let flags: u16 = kani::any();
    let correlation: u64 = kani::any();
    let payload_len_raw: u16 = kani::any();
    let payload_len = u32::from(payload_len_raw);

    kani::assert(IPC_MAGIC == 0x5642_4C54, "production IPC_MAGIC is VBLT");
    kani::assert(IPC_VERSION == 1, "production IPC_VERSION is v1");
    kani::assert(IPC_HEADER_LEN == 24, "production header length is 24");

    let header = IpcFrameHeader::new(command, flags, correlation, payload_len);
    let encoded_result = header.encode();
    kani::assert(
        encoded_result.is_ok(),
        "production header encoder accepts symbolic bounded header",
    );
    let encoded = match encoded_result {
        Ok(value) => value,
        Err(_) => return,
    };

    let [
        magic_0,
        magic_1,
        magic_2,
        magic_3,
        version_0,
        version_1,
        command_0,
        command_1,
        flags_0,
        flags_1,
        reserved_0,
        reserved_1,
        correlation_0,
        correlation_1,
        correlation_2,
        correlation_3,
        correlation_4,
        correlation_5,
        correlation_6,
        correlation_7,
        payload_0,
        payload_1,
        payload_2,
        payload_3,
    ] = encoded;

    kani::assert(
        u32::from_le_bytes([magic_0, magic_1, magic_2, magic_3]) == IPC_MAGIC,
        "encoded header uses production magic bytes",
    );
    kani::assert(
        u16::from_le_bytes([version_0, version_1]) == IPC_VERSION,
        "encoded header uses production version bytes",
    );
    kani::assert(
        u16::from_le_bytes([command_0, command_1]) == command.as_u16(),
        "encoded header preserves production command wire id",
    );
    kani::assert(
        u16::from_le_bytes([flags_0, flags_1]) == flags,
        "encoded header preserves flags",
    );
    kani::assert(
        u16::from_le_bytes([reserved_0, reserved_1]) == 0,
        "encoded header zeros reserved field",
    );
    kani::assert(
        u64::from_le_bytes([
            correlation_0,
            correlation_1,
            correlation_2,
            correlation_3,
            correlation_4,
            correlation_5,
            correlation_6,
            correlation_7,
        ]) == correlation,
        "encoded header preserves correlation",
    );
    kani::assert(
        u32::from_le_bytes([payload_0, payload_1, payload_2, payload_3]) == payload_len,
        "encoded header preserves payload length",
    );

    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);
    kani::assert(
        decoded.is_ok(),
        "production decoder accepts production-encoded bounded header",
    );
    let decoded = match decoded {
        Ok(value) => value,
        Err(_) => return,
    };
    kani::assert(decoded.command == command, "decoded command roundtrips");
    kani::assert(decoded.flags == flags, "decoded flags roundtrip");
    kani::assert(
        decoded.correlation == correlation,
        "decoded correlation roundtrips",
    );
    kani::assert(
        decoded.payload_len == payload_len,
        "decoded payload_len roundtrips",
    );

    kani::cover!(payload_len_raw == 0, "zero-payload header covered");
    kani::cover!(payload_len_raw > 0, "nonzero-payload header covered");
    kani::cover!(
        matches!(command, IpcCommand::UnknownCommand(_)),
        "unknown-command header covered"
    );
}

/// VB-IPC-DECODE-001/002/003 + `P-EMPTY-BODY` IPC gap replacement:
/// actual `IPC_MAGIC`, actual `IPC_HEADER_LEN`, and exposed header wire shape.
#[kani::proof]
fn vb_dzibx_header_constants_and_encode_shape_are_production_bound() {
    let command = any_production_command();
    let flags: u16 = kani::any();
    let correlation: u64 = kani::any();
    let payload_len_raw: u16 = kani::any();
    let payload_len = u32::from(payload_len_raw);

    kani::assert(IPC_MAGIC == 0x5642_4C54, "IPC_MAGIC is actual VBLT value");
    kani::assert(
        IPC_HEADER_LEN == 24,
        "IPC_HEADER_LEN is actual 24-byte header",
    );
    kani::assert(IPC_VERSION == 1, "IPC_VERSION is actual v1 value");
    kani::assert(
        IPC_MAGIC.to_le_bytes() == [0x54, 0x4C, 0x42, 0x56],
        "IPC_MAGIC encodes VBLT little-endian bytes",
    );

    let header = IpcFrameHeader::new(command, flags, correlation, payload_len);
    let encoded_result = header.encode();
    kani::assert(
        encoded_result.is_ok(),
        "production header encode succeeds for symbolic bounded payload_len",
    );
    let encoded = match encoded_result {
        Ok(bytes) => bytes,
        Err(_) => return,
    };

    let [
        magic_0,
        magic_1,
        magic_2,
        magic_3,
        version_0,
        version_1,
        command_0,
        command_1,
        flags_0,
        flags_1,
        reserved_0,
        reserved_1,
        correlation_0,
        correlation_1,
        correlation_2,
        correlation_3,
        correlation_4,
        correlation_5,
        correlation_6,
        correlation_7,
        payload_0,
        payload_1,
        payload_2,
        payload_3,
    ] = encoded;

    kani::assert(
        u32::from_le_bytes([magic_0, magic_1, magic_2, magic_3]) == IPC_MAGIC,
        "encoded header bytes carry production IPC_MAGIC at bytes 0..4",
    );
    kani::assert(
        u16::from_le_bytes([version_0, version_1]) == IPC_VERSION,
        "encoded header bytes carry production IPC_VERSION at bytes 4..6",
    );
    kani::assert(
        u16::from_le_bytes([command_0, command_1]) == command.as_u16(),
        "encoded header bytes carry production command at bytes 6..8",
    );
    kani::assert(
        u16::from_le_bytes([flags_0, flags_1]) == flags,
        "encoded header bytes carry flags at bytes 8..10",
    );
    kani::assert(
        u16::from_le_bytes([reserved_0, reserved_1]) == 0,
        "encoded header bytes zero the reserved field at bytes 10..12",
    );
    kani::assert(
        u64::from_le_bytes([
            correlation_0,
            correlation_1,
            correlation_2,
            correlation_3,
            correlation_4,
            correlation_5,
            correlation_6,
            correlation_7,
        ]) == correlation,
        "encoded header bytes carry correlation at bytes 12..20",
    );
    kani::assert(
        u32::from_le_bytes([payload_0, payload_1, payload_2, payload_3]) == payload_len,
        "encoded header bytes carry payload_len at bytes 20..24",
    );

    kani::assert(
        validate_frame_magic(&encoded).is_ok(),
        "encoded production header passes production magic validator",
    );

    let decoded = decode_frame_header(&encoded);
    kani::assert(
        decoded.is_ok(),
        "decode_frame_header accepts production encoded bounded header",
    );
    let decoded = match decoded {
        Ok(value) => value,
        Err(_) => return,
    };
    kani::assert(decoded.command == command, "decoded command roundtrips");
    kani::assert(decoded.flags == flags, "decoded flags roundtrip");
    kani::assert(
        decoded.correlation == correlation,
        "decoded correlation roundtrips",
    );
    kani::assert(
        decoded.payload_len == payload_len,
        "decoded payload_len roundtrips",
    );

    kani::cover!(payload_len_raw == 0, "zero payload header covered");
    kani::cover!(payload_len_raw > 0, "non-zero payload header covered");
    kani::cover!(
        matches!(command, IpcCommand::UnknownCommand(_)),
        "unknown command covered"
    );
}

/// VB-IPC-DECODE-001 + `P-EMPTY-BODY` IPC gap replacement:
/// arbitrary 24-byte headers must route magic rejection through production
/// `validate_frame_magic` and production `IpcFrameHeader::decode` consistently.
#[kani::proof]
fn vb_dzibx_arbitrary_header_magic_gate_matches_decode() {
    let bytes: [u8; IPC_HEADER_LEN] = kani::any();

    let magic_gate = validate_frame_magic(&bytes);
    let decoded = IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT);

    match magic_gate {
        Err(IpcError::InvalidMagic { actual }) => match decoded {
            Err(IpcError::InvalidMagic {
                actual: decoded_actual,
            }) => {
                kani::assert(
                    decoded_actual == actual,
                    "header decode reports same invalid magic as production magic gate",
                );
            }
            _ => kani::assert(
                false,
                "header decode must reject bad magic before any later header field",
            ),
        },
        Ok(()) => {
            kani::assert(
                !matches!(decoded, Err(IpcError::InvalidMagic { .. })),
                "valid production magic must not decode as InvalidMagic",
            );
        }
        Err(_) => kani::assert(
            false,
            "24-byte header magic gate only succeeds or rejects magic",
        ),
    }

    kani::cover!(
        magic_gate.is_ok(),
        "arbitrary header with valid magic covered"
    );
    kani::cover!(
        matches!(magic_gate, Err(IpcError::InvalidMagic { .. })),
        "arbitrary header with invalid magic covered"
    );
}

/// VB-IPC-DECODE-003 + `P-EMPTY-BODY` IPC gap replacement:
/// complete frame encoding uses the production fixed header followed by the
/// caller-provided payload bytes for symbolic small payloads.
#[kani::proof]
#[kani::unwind(32)]
fn vb_dzibx_encode_frame_uses_actual_header_prefix() {
    let command = any_production_command();
    let flags: u16 = kani::any();
    let correlation: u64 = kani::any();
    let payload_len_raw: u8 = kani::any();
    kani::assume(payload_len_raw <= 4);
    let payload_seed: [u8; 4] = kani::any();
    let [byte_0, byte_1, byte_2, byte_3] = payload_seed;

    let mut payload = Vec::new();
    if payload_len_raw >= 1 {
        payload.push(byte_0);
    }
    if payload_len_raw >= 2 {
        payload.push(byte_1);
    }
    if payload_len_raw >= 3 {
        payload.push(byte_2);
    }
    if payload_len_raw >= 4 {
        payload.push(byte_3);
    }

    let frame = encode_frame(command, flags, correlation, payload.as_slice());
    kani::assert(
        frame.is_ok(),
        "encode_frame succeeds for symbolic small payloads",
    );
    let frame = match frame {
        Ok(value) => value,
        Err(_) => return,
    };

    let expected_len = match IPC_HEADER_LEN.checked_add(usize::from(payload_len_raw)) {
        Some(value) => value,
        None => {
            kani::assert(false, "small symbolic frame length cannot overflow");
            return;
        }
    };
    kani::assert(
        frame.len() == expected_len,
        "frame length is header plus payload",
    );
    kani::assert(
        validate_frame_magic(frame.as_slice()).is_ok(),
        "complete encoded frame has valid production magic prefix",
    );

    let header = IpcFrameHeader::new(command, flags, correlation, u32::from(payload_len_raw));
    let encoded_header = match header.encode() {
        Ok(value) => value,
        Err(_) => return,
    };
    kani::assert(
        frame.as_slice().starts_with(&encoded_header),
        "complete frame starts with production-encoded fixed header",
    );
    let decoded_header = decode_frame_header(&encoded_header);
    kani::assert(
        decoded_header.is_ok(),
        "expected header generated by production encoder decodes",
    );

    kani::cover!(payload_len_raw == 0, "empty frame payload covered");
    kani::cover!(payload_len_raw == 4, "maximum symbolic payload covered");
}
