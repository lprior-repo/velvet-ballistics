#![forbid(unsafe_code)]
//! Record envelope, header, and key types for durable storage.

use vb_core::{ActionId, RunId, WorkflowId};

use super::EventSeq;

/// Decoded record envelope metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordEnvelope {
    /// Magic value identifying the record family.
    pub magic: u32,
    /// Schema version.
    pub schema_version: u16,
    /// Record kind identifier.
    pub record_kind: u16,
    /// Payload sequence number.
    pub sequence: u64,
}

/// Decoded 60-byte record header fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordHeader {
    /// Magic value identifying the record family.
    pub magic: u32,
    /// Schema version.
    pub schema_version: u16,
    /// Record kind identifier.
    pub record_kind: u16,
    /// Header length in bytes.
    pub header_len: u32,
    /// Payload length in bytes.
    pub payload_len: u32,
    /// Payload sequence number.
    pub sequence: u64,
    /// BLAKE3 digest of the payload bytes.
    pub payload_digest: [u8; crate::constants::DIGEST_BYTES],
    /// CRC32C of the header prefix before the checksum field.
    pub header_checksum: u32,
}

/// State marker byte for status index entries.
///
/// These values are encoded directly into the index key to allow
/// range queries filtered by state without decoding the full key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
pub enum IndexStatusState {
    /// Submitted — run has been accepted but not yet started.
    Submitted = 0,
    /// Active — run is currently executing.
    Active = 1,
    /// Completed — run finished successfully.
    Completed = 2,
    /// Unknown or custom state marker.
    Other(u8) = 255,
}

impl IndexStatusState {
    /// Construct a state from a raw u8 byte.
    #[must_use]
    pub const fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Submitted,
            1 => Self::Active,
            2 => Self::Completed,
            _ => Self::Other(value),
        }
    }

    /// Returns the raw u8 encoding used in storage keys.
    ///
    /// **Pre-storage use only.** For callers that are about to write
    /// the byte into an `index_status_key`, prefer [`Self::to_u8_checked`]
    /// which returns `Err(JournalError::IndexStatusStateCollision)`
    /// when `Other(v)` carries a byte in the collision range
    /// `0..MIN_OTHER_STATUS_BYTE`. The raw `to_u8` is preserved for
    /// round-trip tests and decoding paths that already have a valid
    /// byte in hand.
    #[must_use]
    pub const fn to_u8(self) -> u8 {
        match self {
            Self::Submitted => 0,
            Self::Active => 1,
            Self::Completed => 2,
            Self::Other(v) => v,
        }
    }

    /// Returns the raw u8 encoding, rejecting `Other(v)` whose byte
    /// collides with the named variants `Submitted` (0), `Active` (1),
    /// or `Completed` (2).
    ///
    /// Used by the `index_status_key` encoder so an `Other(0|1|2)`
    /// emitted by a caller cannot silently round-trip to the wrong
    /// named variant on read (SC-001 / vb-f1xkn).
    pub fn to_u8_checked(self) -> Result<u8, crate::error::JournalError> {
        let byte = self.to_u8();
        if let Self::Other(_) = self
            && byte < crate::constants::MIN_OTHER_STATUS_BYTE
        {
            return Err(crate::error::JournalError::IndexStatusStateCollision {
                byte,
                min: crate::constants::MIN_OTHER_STATUS_BYTE,
            });
        }
        Ok(byte)
    }

    /// Compile-time exhaustiveness assertion for `to_u8` / `to_u8_checked`.
    ///
    /// `IndexStatusState` is `#[non_exhaustive]` and defined in this
    /// same crate. The match below enumerates every currently-known
    /// variant explicitly. If a new variant is added, both `to_u8` and
    /// `to_u8_checked` must be updated; this assertion block fails to
    /// compile until they are.
    #[allow(dead_code)]
    const _INDEX_STATUS_STATE_EXHAUSTIVE: () = {
        fn _check(state: IndexStatusState) -> u8 {
            match state {
                IndexStatusState::Submitted => 0,
                IndexStatusState::Active => 1,
                IndexStatusState::Completed => 2,
                IndexStatusState::Other(v) => v,
            }
        }
    };
}

/// Key variants supported by the durable storage contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StorageKey {
    /// Workflow source bytes by digest.
    WorkflowSource {
        digest: [u8; crate::constants::DIGEST_BYTES],
    },
    /// Compiled IR bytes by digest.
    CompiledIr {
        digest: [u8; crate::constants::DIGEST_BYTES],
    },
    /// Run metadata by run id.
    RunHeader { run: RunId },
    /// Run event by run id and sequence.
    RunEvent { run: RunId, seq: EventSeq },
    /// Run snapshot by run id and sequence.
    RunSnapshot { run: RunId, seq: EventSeq },
    /// Blob bytes by digest.
    Blob {
        digest: [u8; crate::constants::DIGEST_BYTES],
    },
    /// Status index marker.
    IndexStatus {
        /// State marker; use `IndexStatusState` for type-safe construction.
        state: IndexStatusState,
        timestamp: u64,
        run: RunId,
    },
    /// Workflow/run index marker.
    IndexWorkflow { workflow: WorkflowId, run: RunId },
    /// Pending action index marker.
    IndexAction {
        action: ActionId,
        run: RunId,
        step: vb_core::StepIdx,
    },
}
