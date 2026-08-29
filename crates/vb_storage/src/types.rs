#![forbid(unsafe_code)]
//! Core storage types: configuration, profiles, and sequencing.

use std::num::NonZeroUsize;

use vb_core::{ActionId, RunId, WorkflowId};

/// Storage write limits shared by direct and queued journal writers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageLimits {
    /// Maximum payload bytes accepted for a journal event.
    pub max_journal_event_payload_bytes: u32,
}

impl StorageLimits {
    /// Default storage limits.
    pub const DEFAULT: Self = Self {
        max_journal_event_payload_bytes: crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    };
}

/// Runtime/storage durability profile selected for journal writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DurabilityProfile {
    /// Keep runtime events in volatile memory only; do not write Fjall during the run.
    Volatile,
    /// Queue compact events for bounded group commit without a per-event sync barrier.
    Journaled,
    /// Queue compact events that require a strict persistence barrier when flushed.
    Strict,
}

/// Keyspace tuning profile for per-keyspace configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyspaceProfile {
    /// Small values, bloom filters enabled, no KV separation.
    /// Used for: run_event, index_status, index_workflow, index_action.
    Hot,
    /// Larger values, KV separation enabled.
    /// Used for: workflow_source, compiled_ir, run_snapshot.
    Cold,
    /// Mandatory KV separation for large blob values.
    /// Used for: blob.
    Blob,
}

/// Returns `KeyspaceCreateOptions` tuned for the given profile.
pub fn keyspace_options_for(kind: KeyspaceProfile) -> fjall::KeyspaceCreateOptions {
    use fjall::config::{BloomConstructionPolicy, FilterPolicy, FilterPolicyEntry};

    match kind {
        KeyspaceProfile::Hot => fjall::KeyspaceCreateOptions::default()
            .filter_policy(FilterPolicy::all(FilterPolicyEntry::Bloom(
                BloomConstructionPolicy::BitsPerKey(10.0),
            )))
            .expect_point_read_hits(false),
        KeyspaceProfile::Cold => fjall::KeyspaceCreateOptions::default().with_kv_separation(Some(
            fjall::KvSeparationOptions::default().separation_threshold(4096),
        )),
        KeyspaceProfile::Blob => fjall::KeyspaceCreateOptions::default().with_kv_separation(Some(
            fjall::KvSeparationOptions::default().separation_threshold(1024),
        )),
    }
}

/// Monotonic per-run event sequence.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[repr(transparent)]
pub struct EventSeq(u64);

impl EventSeq {
    /// Creates an event sequence.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw sequence value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Zero event sequence.
    pub const ZERO: Self = Self(0);
    /// Minimum event sequence.
    pub const MIN: Self = Self(0);
    /// Maximum event sequence.
    pub const MAX: Self = Self(u64::MAX);
}

#[cfg(kani)]
impl kani::Arbitrary for EventSeq {
    fn any() -> Self {
        Self::new(kani::any())
    }
}

/// Non-zero bounded capacity for the journal writer queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct JournalQueueCapacity(NonZeroUsize);

impl JournalQueueCapacity {
    /// Creates a queue-capacity contract from a proven non-zero value.
    #[must_use]
    pub const fn new(value: NonZeroUsize) -> Self {
        Self(value)
    }

    /// Validates a raw queue capacity.
    pub fn try_from_usize(value: usize) -> Result<Self, crate::JournalError> {
        NonZeroUsize::new(value)
            .map(Self::new)
            .ok_or(crate::JournalError::QueueCapacity)
    }

    /// Returns the raw capacity.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0.get()
    }
}

/// Non-zero bounded batch size for the journal writer queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct JournalBatchSize(NonZeroUsize);

impl JournalBatchSize {
    /// Creates a batch-size contract from a proven non-zero value.
    #[must_use]
    pub const fn new(value: NonZeroUsize) -> Self {
        Self(value)
    }

    /// Validates a raw batch size.
    pub fn try_from_usize(value: usize) -> Result<Self, crate::JournalError> {
        NonZeroUsize::new(value)
            .map(Self::new)
            .ok_or(crate::JournalError::QueueCapacity)
    }

    /// Returns the raw batch size.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0.get()
    }
}

/// Counts queued journal writes by durability profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalWriterQueueProfileCounts {
    /// Number of journaled pending writes.
    pub journaled: usize,
    /// Number of strict pending writes.
    pub strict: usize,
}

/// Result of flushing a bounded writer queue batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalWriterFlushReport {
    /// Number of queued events drained from memory.
    pub drained: usize,
    /// Number of events written to Fjall.
    pub written: usize,
}

/// Configuration for Fjall-backed storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FjallConfig {
    pub cache_size_bytes: u64,
}

impl FjallConfig {
    /// Minimum cache size: 1 MiB.
    pub const MIN_CACHE_SIZE_BYTES: u64 = 1 << 20;

    /// Maximum cache size: 64 GiB. Past this, Fjall bloom / cache sizing
    /// becomes pathological for a single-server embedded deployment.
    pub const MAX_CACHE_SIZE_BYTES: u64 = 64 << 30;

    /// Constructs a validated cache size, returning `None` for inputs
    /// outside `[MIN_CACHE_SIZE_BYTES, MAX_CACHE_SIZE_BYTES]`.
    #[must_use]
    pub const fn try_new(cache_size_bytes: u64) -> Option<Self> {
        if cache_size_bytes < Self::MIN_CACHE_SIZE_BYTES
            || cache_size_bytes > Self::MAX_CACHE_SIZE_BYTES
        {
            return None;
        }
        Some(Self { cache_size_bytes })
    }

    /// Returns the configured cache size in bytes.
    #[must_use]
    pub const fn cache_size_bytes(self) -> u64 {
        self.cache_size_bytes
    }
}

impl Default for FjallConfig {
    fn default() -> Self {
        Self {
            cache_size_bytes: 268_435_456, // 256 MiB
        }
    }
}

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewPayload {
    /// Raw value bytes (presented as hex in the CLI).
    Raw,
}

// Flux-rs compiler-injected refinements for sequence bounds.
// `flux_rs` is injected by `cargo flux` compiler plugin; no Cargo dependency.
#[cfg(flux)]
pub mod flux_sequence;
#[cfg(all(test, flux))]
mod flux_sequence_tests;
