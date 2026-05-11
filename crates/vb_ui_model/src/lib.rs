#![forbid(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]

//! Typed cold-path view models for Velvet Ballistics UI screens.
//!
//! This crate is intentionally decoupled from Makepad, tokio, and async runtimes.
//! All types are plain data — no fallible constructors or behavior methods.

extern crate alloc;

use alloc::{boxed::Box, string::String, vec::Vec};
use serde::{Deserialize, Serialize};

pub use vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect};
pub use vb_core::capability::Capability;
pub use vb_core::ids::{
    ActionId, BlobId, RunId, SeqNo, SlotIdx, StepIdx, SymbolId, WorkflowDigest, WorkflowId,
};
pub use vb_core::value::Taint;

pub mod emitter;
pub mod envelope;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum UiScreenKind {
    ExecutionOverview = 0,
    WorkflowGraphAuthoring = 1,
    ExecutionDetailsGraph = 2,
    VerificationCertificate = 3,
    ReplayTheater = 4,
    IncidentFailureConsole = 5,
    ActionRegistry = 6,
    StorageDoctorAiContext = 7,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiAppSnapshot {
    pub status: SystemStatusView,
    #[serde(bound = "")]
    pub active_runs: Box<[RunSummaryView]>,
    pub selected_run: Option<RunInspectionView>,
    pub selected_workflow: Option<WorkflowGraphView>,
    pub verification: Option<VerificationReportView>,
    pub replay: Option<ReplayReportView>,
    pub incident: Option<IncidentReportView>,
    #[serde(bound = "")]
    pub actions: Box<[ActionDescriptionView]>,
    pub storage: Option<StorageDoctorView>,
    pub ai_context: Option<AiContextView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSummaryView {
    pub run_id: RunId,
    pub workflow_id: WorkflowId,
    pub status: RunStatus,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub step_count: u32,
    pub event_count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowGraphView {
    pub workflow_id: WorkflowId,
    pub workflow_digest: WorkflowDigest,
    pub nodes: Vec<WorkflowNodeView>,
    pub edges: Vec<WorkflowEdgeView>,
    #[serde(skip)]
    pub node_x: Vec<f32>,
    #[serde(skip)]
    pub node_y: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowNodeView {
    pub step_idx: StepIdx,
    pub label: String,
    pub kind: WorkflowNodeKind,
    pub input_slot_count: u16,
    pub output_slot_count: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum WorkflowNodeKind {
    Sequence = 0,
    Parallel = 1,
    ForEach = 2,
    If = 3,
    Switch = 4,
    Do = 5,
    OnError = 6,
    Finish = 7,
    Start = 8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowEdgeView {
    pub from_step: StepIdx,
    pub to_step: StepIdx,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunInspectionView {
    pub run_id: RunId,
    pub workflow_id: WorkflowId,
    pub status: RunStatus,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub current_step: Option<StepIdx>,
    pub steps: Vec<StepStateView>,
    pub slot_diffs: Vec<SlotDiffView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepStateView {
    pub step_idx: StepIdx,
    pub label: String,
    pub status: StepStatus,
    pub entered_at: Option<i64>,
    pub exited_at: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum StepStatus {
    Pending = 0,
    Running = 1,
    Success = 2,
    Failed = 3,
    Skipped = 4,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotDiffView {
    pub slot_idx: SlotIdx,
    pub before: Option<String>,
    pub after: Option<String>,
    pub taint_before: Taint,
    pub taint_after: Taint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunEventsView {
    pub run_id: RunId,
    pub from_seq: SeqNo,
    pub to_seq: SeqNo,
    pub limit: u32,
    pub events: Vec<RunEventView>,
    pub has_more: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunEventView {
    pub seq: SeqNo,
    pub timestamp: i64,
    pub shard: u32,
    pub step: StepIdx,
    pub kind: RunEventKind,
    pub evidence_id: Option<BlobId>,
    pub digest: Option<[u8; 32]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum RunEventKind {
    StepEntered = 0,
    StepExited = 1,
    ActionIssued = 2,
    ActionDone = 3,
    ActionFailed = 4,
    ErrorCaught = 5,
    RetryScheduled = 6,
    JournalFlushed = 7,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReportView {
    pub workflow_id: WorkflowId,
    pub workflow_digest: WorkflowDigest,
    pub passed: bool,
    pub warnings: Vec<String>,
    pub certificate: VerificationCertificate,
    pub gate_results: Vec<GateResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationCertificate {
    pub structure: bool,
    pub boundedness: bool,
    pub resources: bool,
    pub taint: bool,
    pub action_policy: bool,
    pub durability: bool,
    pub idempotency: bool,
    pub capability: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateResult {
    pub name: String,
    pub passed: bool,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayReportView {
    pub run_id: RunId,
    pub status: RunStatus,
    pub selected_seq: Option<SeqNo>,
    pub events: Vec<RunEventView>,
    pub slot_diffs: Vec<SlotDiffView>,
    pub playback_speed: f32,
    pub is_playing: bool,
    pub recovery: Option<RecoverySuggestion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoverySuggestion {
    pub strategy: RecoveryStrategy,
    pub max_attempts: u16,
    pub idempotency_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum RecoveryStrategy {
    RetrySameKey = 0,
    ScheduleRetry = 1,
    CancelRun = 2,
    OpenReplay = 3,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IncidentReportView {
    pub run_id: RunId,
    pub failure_step: StepIdx,
    pub failure_action: ActionId,
    pub failure_code: String,
    pub attempt: u16,
    pub timestamp: i64,
    pub severity: IncidentSeverity,
    pub safe_to_retry: bool,
    pub idempotency_key_required: bool,
    pub strict_durability: bool,
    pub replay_safe: bool,
    pub repair_hints: Vec<String>,
    pub evidence_chain: EvidenceChain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum IncidentSeverity {
    Warning = 0,
    Critical = 1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceChain {
    pub scheduled_durable: bool,
    pub completion_durable: bool,
    pub side_effect_certainty: f32,
    pub journal_tail: Option<SeqNo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemStatusView {
    pub storage_health: StorageHealth,
    pub writer_queue_depth: u32,
    pub journal_batch_healthy: bool,
    pub snapshot_seq: Option<SeqNo>,
    pub blob_store_ok: bool,
    pub index_healthy: bool,
    pub uptime_seconds: i64,
    pub active_run_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum StorageHealth {
    Healthy = 0,
    Degraded = 1,
    Corrupt = 2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionDescriptionView {
    pub id: ActionId,
    pub name: String,
    pub side_effect: SideEffect,
    pub idempotency: Idempotency,
    pub retry_safety: RetrySafety,
    pub required_capabilities: Box<[Capability]>,
    pub timeout_ms: u64,
    pub input_slot_count: u16,
    pub output_slot_count: u16,
    pub max_input_bytes: u32,
    pub max_output_bytes: u32,
}

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
pub enum CorruptRecordStatus {
    Clean,
    Corrupt {
        count: u64,
        first_seq: Option<SeqNo>,
    },
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
pub struct AiContextPanel {
    pub safe_for_model: bool,
    pub secrets_redacted: bool,
    pub blobs_summarized: bool,
    pub suggested_commands: Vec<SuggestedCommand>,
    pub failure_summary: String,
    pub replay_safety: ReplaySafety,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuggestedCommand {
    pub cmd: String,
    pub desc: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplaySafety {
    Safe,
    Unsafe { reason: String },
    Unknown,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiContextView {
    pub run_id: RunId,
    pub safe_for_model: bool,
    pub secrets_redacted: bool,
    pub blobs_summarized: bool,
    pub failure_summary: Option<String>,
    pub replay_safe: bool,
    pub suggested_commands: Box<[String]>,
    pub last_cert_check: Option<i64>,
    pub last_replay_check: Option<i64>,
    pub last_crash_lab_fixture: Option<String>,
    pub incomplete_evidence_warnings: Box<[String]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum RunStatus {
    Pending = 0,
    Running = 1,
    Success = 2,
    Failure = 3,
    Cancelled = 4,
}
