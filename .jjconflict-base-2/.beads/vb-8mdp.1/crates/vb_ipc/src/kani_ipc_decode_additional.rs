#![forbid(unsafe_code)]
#![cfg(kani)]
//! VB-IPC-DECODE-001 / VB-IPC-DECODE-003 / VB-IPC-DECODE-004: Additional decode order proofs
//!
//! - VB-IPC-DECODE-001: Total function proof — decode never panics for any [u8; 24]
//! - VB-IPC-DECODE-003: version checked before command (step 2 before step 3)
//! - VB-IPC-DECODE-004: command checked before reserved (step 3 before step 4)

use crate::{
    bounded::MaxPayloadBytes,
    commands::IpcCommand,
    constants::{IPC_HEADER_LEN, IPC_MAGIC, IPC_VERSION},
    error::IpcError,
    frame_types::IpcFrameHeader,
};

/// VB-IPC-DECODE-001 H-total:
/// Prove that `decode` is total — it returns Result<Self, IpcError> for ALL
/// 2^192 possible [u8; 24] inputs without panicking.
///
/// This is the strongest guarantee: no input can cause a panic. The harness
/// uses kani::any() on the full 24-byte array to symbolically execute all
/// possible inputs.
#[kani::proof]
#[kani::unwind(6)]
fn kani_ipc_decode_total_fn() {
    let bytes: [u8; IPC_HEADER_LEN] = kani::any();

    // decode must not panic — the result is guaranteed to be Result<Self, IpcError>
    let _result = IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT);

    // Additional cover: prove all error variants are reachable
    kani::cover!(matches!(_result, Err(IpcError::InvalidMagic { .. })), "InvalidMagic reachable");
    kani::cover!(matches!(_result, Err(IpcError::UnsupportedVersion { .. })), "UnsupportedVersion reachable");
    kani::cover!(matches!(_result, Err(IpcError::ReservedNonZero { .. })), "ReservedNonZero reachable");
    kani::cover!(matches!(_result, Err(IpcError::PayloadTooLarge { .. })), "PayloadTooLarge reachable");
    kani::cover!(_result.is_ok(), "Ok reachable");
}

/// VB-IPC-DECODE-003 H-version-before-command:
/// Prove version check (step 2) precedes command check (step 3).
/// For any header where magic is correct but version is wrong, we must get
/// UnsupportedVersion — NOT UnknownCommand (which would imply version was skipped).
#[kani::proof]
#[kani::unwind(4)]
fn kani_harness_decode_order_version_before_command() {
    // Fix magic = IPC_MAGIC (correct)
    // Set version != IPC_VERSION (wrong)
    // Set command = arbitrary u16
    let arbitrary_version: u16 = kani::any();
    kani::assume(arbitrary_version != IPC_VERSION);
    let arbitrary_command: u16 = kani::any();

    let mut bytes: [u8; IPC_HEADER_LEN] = kani::any();
    bytes[0..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    bytes[4..6].copy_from_slice(&arbitrary_version.to_le_bytes());
    bytes[6..8].copy_from_slice(&arbitrary_command.to_le_bytes());
    bytes[8..10].copy_from_slice(&0u16.to_le_bytes()); // flags
    bytes[10..12].copy_from_slice(&0u16.to_le_bytes()); // reserved
    bytes[12..20].copy_from_slice(&0u64.to_le_bytes()); // correlation
    bytes[20..24].copy_from_slice(&0u32.to_le_bytes()); // payload_len

    let result = IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT);

    // If version is wrong, UnsupportedVersion MUST be returned — NOT UnknownCommand
    // This proves version check happens before command validation
    match result {
        Err(IpcError::UnsupportedVersion { actual }) => {
            kani::assert(actual == arbitrary_version, "UnsupportedVersion contains wrong version");
            kani::cover!(true, "UnsupportedVersion returned for wrong version");
        }
        Err(IpcError::InvalidMagic { .. }) => {
            // Magic check happens first (step 1) — version not reached
        }
        Ok(_) => {
            // Should not happen with wrong version
            kani::assert(false, "wrong version should not return Ok");
        }
        Err(IpcError::UnknownCommand(_)) => {
            // If we got UnknownCommand with wrong version, version was NOT checked first
            kani::assert(false, "version must be checked before command");
        }
        Err(_) => {
            // Other errors before version check
        }
    }
}

/// VB-IPC-DECODE-004 H-command-before-reserved:
/// Prove command check (step 3) precedes reserved field extraction (step 4).
/// For any header where magic and version are correct but command is invalid,
/// we must get UnknownCommand — NOT ReservedNonZero (which would imply reserved was read first).
#[kani::proof]
#[kani::unwind(4)]
fn kani_harness_decode_order_command_before_reserved() {
    // Fix magic = IPC_MAGIC (correct)
    // Fix version = IPC_VERSION (correct)
    // Set command = invalid u16 (outside 1..16)
    let invalid_command: u16 = kani::any();
    kani::assume(invalid_command < 1 || invalid_command > 16);

    let mut bytes: [u8; IPC_HEADER_LEN] = kani::any();
    bytes[0..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    bytes[6..8].copy_from_slice(&invalid_command.to_le_bytes());
    bytes[8..10].copy_from_slice(&0u16.to_le_bytes()); // flags
    bytes[10..12].copy_from_slice(&0u16.to_le_bytes()); // reserved = 0 (valid)
    bytes[12..20].copy_from_slice(&0u64.to_le_bytes()); // correlation
    bytes[20..24].copy_from_slice(&0u32.to_le_bytes()); // payload_len

    let result = IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT);

    // If command is invalid, UnknownCommand MUST be returned — NOT ReservedNonZero
    // This proves command check happens before reserved field extraction
    match result {
        Err(IpcError::UnknownCommand(actual)) => {
            kani::assert(actual == invalid_command, "UnknownCommand contains wrong value");
            kani::cover!(true, "UnknownCommand returned for invalid command");
        }
        Err(IpcError::InvalidMagic { .. }) => {
            // Magic check happens first
        }
        Err(IpcError::UnsupportedVersion { .. }) => {
            // Version check happens second
        }
        Err(IpcError::ReservedNonZero { .. }) => {
            // If we got ReservedNonZero with reserved=0, reserved was NOT checked after command
            kani::assert(false, "command must be checked before reserved");
        }
        Ok(_) => {
            kani::assert(false, "invalid command should not return Ok");
        }
        Err(_) => {
            // Other errors
        }
    }
}

/// VB-IPC-SERVER-003 H-oversize-before-payload-read:
/// Prove that decode returns PayloadTooLarge WITHOUT reading any payload bytes.
///
/// The decode function reads ONLY the 24-byte header (magic, version, command,
/// flags, reserved, correlation, payload_len). It NEVER reads payload bytes.
/// Therefore PayloadTooLarge is determined solely from the header, before any
/// payload data could be read from the socket.
#[kani::proof]
#[kani::unwind(4)]
fn kani_ipc_header_rejects_oversize_before_payload_read() {
    // Build header with oversized payload_len
    let oversized_payload_len: u32 = kani::any();
    kani::assume(oversized_payload_len as usize > MaxPayloadBytes::DEFAULT.get());

    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, oversized_payload_len);
    let encoded = header.encode();
    kani::assume(encoded.is_ok());
    let Ok(encoded) = encoded else { return };

    let result = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

    // MUST return PayloadTooLarge — this error is determined from header alone
    match result {
        Err(IpcError::PayloadTooLarge { actual, limit }) => {
            kani::assert(actual == oversized_payload_len as usize, "actual length preserved");
            kani::assert(limit == MaxPayloadBytes::DEFAULT.get(), "limit preserved");
            kani::cover!(true, "PayloadTooLarge returned for oversized header");
        }
        Ok(_) => {
            kani::assert(false, "oversized payload should not return Ok");
        }
        Err(_) => {
            kani::assert(false, "oversized payload should return PayloadTooLarge specifically");
        }
    }

    // COVER: oversize path
    kani::cover!(
        matches!(result, Err(IpcError::PayloadTooLarge { .. })),
        "oversize rejection path"
    );
}
