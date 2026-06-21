#![forbid(unsafe_code)]
//! Parsed slot-write extra envelope.
//!
//! Replaces the previous schrödinized `Option<Vec<u8>>` field on
//! [`SlotWrittenEvent`] with a typed [`SlotWriteExtra`] sum type that
//! distinguishes the versioned envelope from legacy raw frame-extra bytes
//! at the type level. The legacy variant preserves the wire-format
//! migration path so previously persisted journals remain readable.

use crate::error::JournalError;
use crate::slot_extra::{DecodedSlotWrittenExtra, SlotWrittenExtraEnvelope, decode_slot_written_extra};
#[cfg(test)]
pub(crate) use crate::slot_extra::{
    SLOT_WRITTEN_EXTRA_PREFIX, SlotWrittenExtraError, encode_slot_written_extra,
};

/// Parsed slot-write extra payload carried by `SlotWrittenEvent`.
///
/// # Variants
///
/// - [`SlotWriteExtra::Versioned`] — current storage produces these
///   envelopes: a versioned [`SlotWrittenExtraEnvelope`] tagged with
///   [`SLOT_WRITTEN_EXTRA_PREFIX`].
/// - [`SlotWriteExtra::Legacy`] — legacy frame-extra bytes with no
///   embedded taint metadata, kept so older journals remain readable.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum SlotWriteExtra {
    /// Versioned slot-write extra envelope produced by current adapters.
    Versioned(SlotWrittenExtraEnvelope),
    /// Legacy frame-extra bytes with no embedded taint metadata.
    Legacy(Vec<u8>),
}

impl SlotWriteExtra {
    /// Parse the raw extra bytes carried by a `SlotWrittenEvent`.
    ///
    /// Recognises the [`SLOT_WRITTEN_EXTRA_PREFIX`] envelope produced by
    /// current adapters and falls back to [`SlotWriteExtra::Legacy`] for
    /// any other byte sequence. The empty payload is rejected with
    /// [`JournalError::InvalidEvent`] because every persisted slot write
    /// with a non-`None` extra must carry at least one byte of payload
    /// (the legacy five-byte frame-extra minimum or the prefix bytes).
    ///
    /// # Errors
    ///
    /// - [`JournalError::InvalidEvent`] when `bytes` is empty.
    /// - [`JournalError::PostcardDecodeFailed`] when `bytes` starts with
    ///   the v1 prefix but the trailing postcard payload is corrupt or
    ///   truncated.
    pub fn parse(bytes: &[u8]) -> Result<Self, JournalError> {
        if bytes.is_empty() {
            return Err(JournalError::InvalidEvent);
        }
        match decode_slot_written_extra(bytes) {
            Ok(DecodedSlotWrittenExtra::Envelope(envelope)) => Ok(Self::Versioned(envelope)),
            Ok(DecodedSlotWrittenExtra::LegacyFrameExtra(payload)) => {
                Ok(Self::Legacy(payload.to_vec()))
            }
            Err(_) => Err(JournalError::PostcardDecodeFailed),
        }
    }

    /// Frame-extra bytes carried by either the versioned envelope or the
    /// legacy payload, when present.
    #[must_use]
    pub fn frame_extra(&self) -> Option<&[u8]> {
        match self {
            Self::Versioned(envelope) => envelope.frame_extra.as_deref(),
            Self::Legacy(bytes) => Some(bytes.as_slice()),
        }
    }
}

#[cfg(test)]
mod tests;
