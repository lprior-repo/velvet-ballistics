#![forbid(unsafe_code)]
//! Durable record types for storage.

use vb_core::{RunId, WorkflowDigest, WorkflowId};

/// Record kind identifiers from the storage contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u16)]
#[non_exhaustive]
pub enum RecordKind {
    /// Workflow source record.
    WorkflowSource = 1,
    /// Compiled IR record.
    CompiledIr = 2,
    /// Run header record.
    RunHeader = 3,
    /// Run accepted event.
    RunAccepted = 10,
    /// Step started event.
    StepStarted = 11,
    /// Slot written event.
    SlotWritten = 12,
    /// Action scheduled event.
    ActionScheduled = 13,
    /// Action completed event.
    ActionCompleted = 14,
    /// Action failed event.
    ActionFailed = 15,
    /// Wait scheduled event.
    WaitScheduled = 16,
    /// Ask scheduled event.
    AskScheduled = 17,
    /// Ask answered event.
    AskAnswered = 18,
    /// Retry scheduled event.
    RetryScheduled = 19,
    /// Step failed event.
    StepFailed = 20,
    /// Run cancelled event.
    RunCancelled = 21,
    /// Run finished event.
    RunFinished = 22,
    /// Run failed event.
    RunFailed = 23,
    /// Run admission metadata event.
    RunAdmission = 24,
    /// Run resumed event.
    RunResumed = 25,
    /// Run retried event.
    RunRetried = 26,
    /// Run answered event.
    RunAnswered = 27,
    /// Snapshot record.
    Snapshot = 30,
    /// Blob record.
    Blob = 40,
    /// Index update record.
    IndexUpdate = 50,
}

impl RecordKind {
    /// Returns the wire identifier.
    #[must_use]
    pub const fn id(self) -> u16 {
        match self {
            Self::WorkflowSource => 1,
            Self::CompiledIr => 2,
            Self::RunHeader => 3,
            Self::RunAccepted => 10,
            Self::StepStarted => 11,
            Self::SlotWritten => 12,
            Self::ActionScheduled => 13,
            Self::ActionCompleted => 14,
            Self::ActionFailed => 15,
            Self::WaitScheduled => 16,
            Self::AskScheduled => 17,
            Self::AskAnswered => 18,
            Self::RetryScheduled => 19,
            Self::StepFailed => 20,
            Self::RunCancelled => 21,
            Self::RunFinished => 22,
            Self::RunFailed => 23,
            Self::RunAdmission => 24,
            Self::RunResumed => 25,
            Self::RunRetried => 26,
            Self::RunAnswered => 27,
            Self::Snapshot => 30,
            Self::Blob => 40,
            Self::IndexUpdate => 50,
        }
    }
}

/// Immutable workflow source bytes bound to their digest.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WorkflowSourceRecord {
    /// Source digest key.
    pub digest: WorkflowDigest,
    /// Original strict YAML authoring bytes.
    pub source: Vec<u8>,
}

/// Compiled IR artifact bytes bound to their digest.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CompiledIrRecord {
    /// Compiled IR digest key.
    pub digest: WorkflowDigest,
    /// Postcard-compatible compiled artifact bytes.
    pub ir: Vec<u8>,
}

/// Minimal run metadata record.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RunHeaderRecord {
    /// Run identifier.
    pub run: RunId,
    /// Workflow identifier.
    pub workflow_id: WorkflowId,
    /// Compiled workflow digest bound at run acceptance.
    pub compiled_digest: WorkflowDigest,
    /// Status byte owned by the runtime status model.
    pub status: u8,
    /// Admission timestamp in milliseconds supplied by the caller.
    pub accepted_at_ms: u64,
}

/// Large payload blob record.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BlobRecord {
    /// Blob digest key.
    pub digest: [u8; crate::constants::DIGEST_BYTES],
    /// Bounded blob payload.
    pub bytes: Vec<u8>,
}
