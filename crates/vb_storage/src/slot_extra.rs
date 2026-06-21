#![forbid(unsafe_code)]
//! Versioned slot-write extra envelope.

use vb_core::Taint;

use crate::constants::MAX_FRAME_EXTRA_BYTES;

/// Prefix that distinguishes the v1 slot-write envelope from legacy frame extra bytes.
pub const SLOT_WRITTEN_EXTRA_PREFIX: &[u8; 5] = b"VBSE\x01";

/// Errors while encoding or decoding the slot-write extra envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SlotWrittenExtraError {
    /// Envelope payload could not be encoded.
    EncodeFailed,
    /// Envelope output allocation failed.
    AllocationFailed,
    /// Envelope prefix was present, but the payload could not be decoded.
    DecodeFailed,
    /// Envelope payload exceeded [`MAX_FRAME_EXTRA_BYTES`] and was rejected
    /// before postcard could allocate a `Vec` from its varint length prefix.
    Oversized {
        /// Observed payload length after the prefix was stripped.
        len: usize,
        /// Configured cap from [`MAX_FRAME_EXTRA_BYTES`].
        max: usize,
    },
}

/// Decoded v1 slot-write extra envelope.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SlotWrittenExtraEnvelope {
    /// Taint recorded with the slot value.
    pub taint: Taint,
    /// Existing frame extra bytes, when the runtime had a frame-extra payload.
    pub frame_extra: Option<Vec<u8>>,
}

/// Decoded slot-write extra bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodedSlotWrittenExtra<'a> {
    /// v1 envelope produced by current runtime/storage adapters.
    Envelope(SlotWrittenExtraEnvelope),
    /// Legacy frame extra bytes with no embedded taint metadata.
    LegacyFrameExtra(&'a [u8]),
}

/// Encodes taint plus optional frame extra bytes into a versioned envelope.
pub fn encode_slot_written_extra(
    taint: Taint,
    frame_extra: Option<Vec<u8>>,
) -> Result<Vec<u8>, SlotWrittenExtraError> {
    let envelope = SlotWrittenExtraEnvelope { taint, frame_extra };
    let payload =
        postcard::to_allocvec(&envelope).map_err(|_| SlotWrittenExtraError::EncodeFailed)?;
    let capacity = SLOT_WRITTEN_EXTRA_PREFIX
        .len()
        .checked_add(payload.len())
        .ok_or(SlotWrittenExtraError::AllocationFailed)?;
    let mut out = Vec::with_capacity(capacity);
    out.extend_from_slice(SLOT_WRITTEN_EXTRA_PREFIX);
    out.extend_from_slice(&payload);
    Ok(out)
}

/// Decodes a slot-write extra envelope or classifies legacy frame extra bytes.
///
/// Before handing the bytes to postcard, this function enforces a payload
/// length cap from [`MAX_FRAME_EXTRA_BYTES`]. Without that cap, a hostile
/// Fjall image can carry a single varint length prefix that requests an
/// allocation up to `u32::MAX` bytes before any payload byte is validated.
pub fn decode_slot_written_extra(
    bytes: &[u8],
) -> Result<DecodedSlotWrittenExtra<'_>, SlotWrittenExtraError> {
    match bytes.strip_prefix(SLOT_WRITTEN_EXTRA_PREFIX) {
        Some(payload) => {
            if payload.len() > MAX_FRAME_EXTRA_BYTES {
                return Err(SlotWrittenExtraError::Oversized {
                    len: payload.len(),
                    max: MAX_FRAME_EXTRA_BYTES,
                });
            }
            postcard::from_bytes::<SlotWrittenExtraEnvelope>(payload)
                .map(DecodedSlotWrittenExtra::Envelope)
                .map_err(|_| SlotWrittenExtraError::DecodeFailed)
        }
        None => Ok(DecodedSlotWrittenExtra::LegacyFrameExtra(bytes)),
    }
}
