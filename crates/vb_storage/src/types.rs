//! Core storage types: configuration, profiles, and sequencing.

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, Copy)]
pub struct FjallConfig {
    /// Cache size in bytes.
    pub cache_size_bytes: u64,
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

/// Key variants supported by the durable storage contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageKey {
    /// Workflow source bytes by digest.
    WorkflowSource { digest: [u8; crate::constants::DIGEST_BYTES] },
    /// Compiled IR bytes by digest.
    CompiledIr { digest: [u8; crate::constants::DIGEST_BYTES] },
    /// Run metadata by run id.
    RunHeader { run: RunId },
    /// Run event by run id and sequence.
    RunEvent { run: RunId, seq: EventSeq },
    /// Run snapshot by run id and sequence.
    RunSnapshot { run: RunId, seq: EventSeq },
    /// Blob bytes by digest.
    Blob { digest: [u8; crate::constants::DIGEST_BYTES] },
    /// Status index marker.
    IndexStatus {
        state: u8,
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
