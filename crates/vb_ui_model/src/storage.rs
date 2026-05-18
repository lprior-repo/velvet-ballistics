//! Storage doctor and diagnostic view types.

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use vb_core::ids::SeqNo;

use crate::ai::AiContextPanel;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageDoctorView {
    pub health: StorageHealthPanel,
    pub journal: JournalDoctorPanel,
    pub ai_context: AiContextPanel,
    pub evidence: EvidenceCardPanel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageHealthPanel {
    pub fjall_keyspaces: Vec<KeyspaceMetrics>,
    pub writer_queue: WriterQueueStatus,
    pub journal_batch: JournalBatchHealth,
    pub snapshot: SnapshotStatus,
    pub blob_store: BlobStoreStatus,
    pub index: IndexHealth,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyspaceMetrics {
    pub name: String,
    pub key_count: u64,
    pub size_bytes: u64,
    pub profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriterQueueStatus {
    pub pending_journaled: usize,
    pub pending_strict: usize,
    pub is_shutdown: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalBatchHealth {
    pub last_flush_ms: Option<i64>,
    pub flushed_count: u64,
    pub dropped_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotStatus {
    pub latest_seq: Option<SeqNo>,
    pub snapshot_count: u64,
    pub is_corrupt: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobStoreStatus {
    pub blob_count: u64,
    pub size_bytes: u64,
    pub is_accessible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexHealth {
    pub status_count: u64,
    pub workflow_count: u64,
    pub action_count: u64,
    pub is_consistent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalDoctorPanel {
    pub run_event_count: u64,
    pub snapshot_seq: u64,
    pub tail_seq: u64,
    pub corrupt_records: CorruptRecordStatus,
    pub trim_recommendation: TrimRecommendation,
    pub digest_checks: DigestCheckResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CorruptRecordStatus {
    Clean,
    Corrupt {
        count: u64,
        first_seq: Option<SeqNo>,
    },
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TrimRecommendation {
    NotNeeded,
    Recommended { tail_seq: u64, snapshot_seq: u64 },
    Critical { tail_seq: u64, snapshot_seq: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DigestCheckResult {
    pub workflow_source_ok: bool,
    pub compiled_ir_ok: bool,
    pub all_ok: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceCardPanel {
    pub last_cert_check: Option<i64>,
    pub last_replay_check: Option<i64>,
    pub last_crash_lab_fixture: Option<i64>,
    pub incomplete_warnings: Vec<IncompleteWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncompleteWarning {
    pub kind: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrashLabFixture {
    pub fixture_id: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DigestCheck {
    pub name: String,
    pub expected: [u8; 32],
    pub actual: [u8; 32],
    pub ok: bool,
}
