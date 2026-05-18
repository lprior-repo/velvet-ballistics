//! Run execution view types.

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::Taint;
use vb_core::ids::{BlobId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowId};

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
#[non_exhaustive]
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
#[non_exhaustive]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum RunStatus {
    Pending = 0,
    Running = 1,
    Success = 2,
    Failure = 3,
    Cancelled = 4,
}
