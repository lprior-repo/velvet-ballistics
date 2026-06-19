#![forbid(unsafe_code)]
//! Journal keyspace preview configuration and output types.

use std::num::NonZeroUsize;

use super::keys::StorageKey;

/// Configuration for bounded keyspace preview.
///
/// Caps the maximum number of records and total bytes included in a preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewConfig {
    max_records: NonZeroUsize,
    max_bytes: u32,
}

impl PreviewConfig {
    /// Creates a new preview configuration.
    ///
    /// Returns `JournalError::QueueCapacity` if `max_records` is zero.
    pub fn new(max_records: usize, max_bytes: u32) -> Result<Self, crate::JournalError> {
        let max_records =
            NonZeroUsize::new(max_records).ok_or(crate::JournalError::QueueCapacity)?;
        Ok(Self {
            max_records,
            max_bytes,
        })
    }

    /// Returns the maximum number of records to include.
    #[must_use]
    pub const fn max_records(&self) -> NonZeroUsize {
        self.max_records
    }

    /// Returns the maximum total value bytes to include.
    #[must_use]
    pub const fn max_bytes(&self) -> u32 {
        self.max_bytes
    }
}

/// Decoded preview of a journal keyspace range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedPreview {
    /// Decoded entries: (key, value_bytes, payload_type).
    pub entries: Vec<(StorageKey, Vec<u8>, PreviewPayload)>,
    /// Total number of records in the scanned keyspace range.
    pub total_keyspace_records: u64,
    /// Whether the output was truncated due to configured caps.
    pub truncated: bool,
}

/// Preview payload variant indicating how value bytes are presented.
///
/// `Raw` is intentionally the only variant today: preview output keeps the
/// encoded value bytes and leaves semantic decoding to explicit doctor paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewPayload {
    /// Raw value bytes (presented as hex in the CLI).
    Raw,
}
