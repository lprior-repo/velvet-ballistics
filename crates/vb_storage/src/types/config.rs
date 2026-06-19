#![forbid(unsafe_code)]
//! Storage configuration: limits, durability profiles, and keyspace tuning.

use crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES;

/// Storage write limits shared by direct and queued journal writers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageLimits {
    /// Maximum payload bytes accepted for a journal event.
    pub max_journal_event_payload_bytes: u32,
}

impl StorageLimits {
    /// Default storage limits.
    pub const DEFAULT: Self = Self {
        max_journal_event_payload_bytes: MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
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

/// Fjall-backed storage configuration.
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
