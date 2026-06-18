//! Magic byte validation for IPC frame headers.
//!
//! Validates the 4-byte magic prefix before allocating a large read buffer.

use super::super::IpcError;
use crate::IPC_MAGIC;

/// Maximum bytes to read while still in AwaitingMagic state.
pub(crate) const AWAITING_MAGIC_MAX_BYTES: usize = 4;

/// Magic validation state for IPC frame header parsing.
///
/// Tracks whether we have validated the magic bytes at the start of a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MagicValidationState {
    /// Have not yet validated magic bytes.
    AwaitingMagic,
    /// Magic bytes validated successfully.
    MagicValidated,
}

/// Validates magic bytes early, before allocating a large read buffer.
///
/// Returns `MagicValidated` if the magic matches `IPC_MAGIC`.
/// Returns `AwaitingMagic` if not enough bytes have been collected.
/// Returns an `IpcError::InvalidMagic` if the magic bytes are present but do not match.
pub(crate) fn validate_magic_early(bytes: &[u8]) -> Result<MagicValidationState, IpcError> {
    let Some(prefix) = bytes.get(..AWAITING_MAGIC_MAX_BYTES) else {
        return Ok(MagicValidationState::AwaitingMagic);
    };
    let magic_bytes = <[u8; AWAITING_MAGIC_MAX_BYTES]>::try_from(prefix)
        .map_err(|_| IpcError::HeaderDecodeFailed)?;
    let magic = u32::from_le_bytes(magic_bytes);
    if magic == IPC_MAGIC {
        Ok(MagicValidationState::MagicValidated)
    } else {
        Err(IpcError::InvalidMagic { actual: magic })
    }
}
