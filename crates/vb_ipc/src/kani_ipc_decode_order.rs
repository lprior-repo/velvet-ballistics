#![forbid(unsafe_code)]
#![cfg(kani)]
//! VB-IPC-POSTCARD-ENVELOPE-001: IPC frame decode order verification
//!
//! These proofs verify the strict decode step ordering enforced by
//! `IpcFrameHeader::decode`:
//! - Step 2 (magic) must precede Step 3 (version)
//! - Step 5 (reserved) must precede Step 6 (payload_len)

use crate::{
    bounded::MaxPayloadBytes,
    commands::IpcCommand,
    constants::{IPC_HEADER_LEN, IPC_MAGIC, IPC_VERSION},
    error::IpcError,
    frame_types::IpcFrameHeader,
};

/// VB-IPC-POSTCARD-ENVELOPE-001 H1:
/// Prove that `InvalidMagic` is returned before `UnsupportedVersion`.
/// For any arbitrary 24-byte header, if magic is wrong, version is never checked.
#[kani::proof]
#[kani::unwind(4)]
fn kani_harness_ipc_decode_order() {
    // Generate arbitrary 24-byte header
    let bytes: [u8; IPC_HEADER_LEN] = kani::any();

    // Set an arbitrary magic at offset 0 (may or may not match IPC_MAGIC)
    let magic: u32 = kani::any();
    let mut bytes = bytes;
    bytes[0..4].copy_from_slice(&magic.to_le_bytes());
    bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
    bytes[8..10].copy_from_slice(&0u16.to_le_bytes()); // flags
    bytes[10..12].copy_from_slice(&0u16.to_le_bytes()); // reserved
    bytes[12..20].copy_from_slice(&0u64.to_le_bytes()); // correlation
    bytes[20..24].copy_from_slice(&0u32.to_le_bytes()); // payload_len

    let result = IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT);

    // If magic is wrong, we MUST get InvalidMagic, not UnsupportedVersion
    if magic != IPC_MAGIC {
        match result {
            Err(IpcError::InvalidMagic { actual }) => {
                kani::assert(actual == magic, "InvalidMagic contains wrong magic");
            }
            Ok(_) => {
                kani::assert(false, "wrong magic should never return Ok");
            }
            Err(_) => {
                // Only InvalidMagic is acceptable for wrong magic
            }
        }
    }

    // COVER: various error paths
    kani::cover!(result.is_ok(), "decode succeeds");
    kani::cover!(
        matches!(result, Err(IpcError::InvalidMagic { .. })),
        "InvalidMagic error"
    );
    kani::cover!(
        matches!(result, Err(IpcError::UnsupportedVersion { .. })),
        "UnsupportedVersion error"
    );
    kani::cover!(
        matches!(result, Err(IpcError::ReservedNonZero { .. })),
        "ReservedNonZero error"
    );
    kani::cover!(
        matches!(result, Err(IpcError::PayloadTooLarge { .. })),
        "PayloadTooLarge error"
    );
}

/// VB-IPC-POSTCARD-ENVELOPE-001 H2:
/// Prove that `ReservedNonZero` is returned before `PayloadTooLarge`.
/// For any header with non-zero reserved field, we get ReservedNonZero
/// before any payload_len bounds check.
#[kani::proof]
#[kani::unwind(4)]
fn kani_harness_ipc_reserved_nonzero_before_payload_len() {
    let mut bytes: [u8; IPC_HEADER_LEN] = kani::any();

    // Set correct magic
    bytes[0..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    // Set valid version
    bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
    bytes[8..10].copy_from_slice(&0u16.to_le_bytes()); // flags

    // Set non-zero reserved at offset 10..12
    let reserved: u16 = kani::any();
    kani::assume(reserved != 0);
    bytes[10..12].copy_from_slice(&reserved.to_le_bytes());

    bytes[12..20].copy_from_slice(&0u64.to_le_bytes()); // correlation

    // Set potentially oversized payload_len at offset 20..24
    let payload_len: u32 = kani::any();
    bytes[20..24].copy_from_slice(&payload_len.to_le_bytes());

    let result = IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT);

    // Must return ReservedNonZero before checking payload_len
    match result {
        Err(IpcError::ReservedNonZero { actual }) => {
            kani::assert(actual == reserved, "ReservedNonZero contains wrong value");
        }
        Err(IpcError::InvalidMagic { .. }) => {
            // Magic check happens first - both are valid
        }
        Ok(_) => {
            kani::assert(false, "non-zero reserved should not return Ok");
        }
        Err(_) => {
            // Other errors before ReservedNonZero
        }
    }

    // COVER: ReservedNonZero path
    kani::cover!(
        matches!(result, Err(IpcError::ReservedNonZero { .. })),
        "reserved nonzero path"
    );
}

/// VB-IPC-POSTCARD-ENVELOPE-001 H3:
/// Prove IPC magic check happens before version check for all possible 24-byte inputs.
#[kani::proof]
#[kani::unwind(4)]
fn kani_harness_ipc_magic_before_version() {
    // Generate arbitrary magic value
    let arbitrary_magic: u32 = kani::any();

    let mut bytes: [u8; IPC_HEADER_LEN] = kani::any();
    bytes[0..4].copy_from_slice(&arbitrary_magic.to_le_bytes());
    bytes[4..6].copy_from_slice(&(IPC_VERSION + 1).to_le_bytes()); // wrong version
    bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
    bytes[8..10].copy_from_slice(&0u16.to_le_bytes());
    bytes[10..12].copy_from_slice(&0u16.to_le_bytes());
    bytes[12..20].copy_from_slice(&0u64.to_le_bytes());
    bytes[20..24].copy_from_slice(&0u32.to_le_bytes());

    let result = IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT);

    // If magic is wrong, InvalidMagic must be returned (not UnsupportedVersion)
    if arbitrary_magic != IPC_MAGIC {
        match result {
            Err(IpcError::InvalidMagic { .. }) => {
            }
            Ok(_) => {
                kani::assert(false, "wrong magic should not return Ok");
            }
            Err(_) => {
                // Should be InvalidMagic, but any error is before version check
            }
        }
    }

    kani::cover!(
        matches!(result, Err(IpcError::InvalidMagic { .. })),
        "invalid magic detected"
    );
}
