#![forbid(unsafe_code)]
//! Record kind identifiers from the storage contract.

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
    /// Step succeeded event.
    StepSucceeded = 29,
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
    /// Run killed event.
    RunKilled = 28,
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
    /// Recovery stamp record (master contract §18).
    ///
    /// A journal marker written during the recovery process to checkpoint
    /// replay progress. A subsequent recovery invocation can detect a prior
    /// recovery attempt's progress by reading the stamp and resume from
    /// that point instead of re-replaying the full journal. Carries the
    /// `VBSR` magic (`0x56425352`) per the storage envelope contract.
    RecoveryStamp = 7,
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
            Self::StepSucceeded => 29,
            Self::ActionScheduled => 13,
            Self::ActionCompleted => 14,
            Self::ActionFailed => 15,
            Self::WaitScheduled => 16,
            Self::AskScheduled => 17,
            Self::AskAnswered => 18,
            Self::RetryScheduled => 19,
            Self::StepFailed => 20,
            Self::RunCancelled => 21,
            Self::RunKilled => 28,
            Self::RunFinished => 22,
            Self::RunFailed => 23,
            Self::RunAdmission => 24,
            Self::RunResumed => 25,
            Self::RunRetried => 26,
            Self::RunAnswered => 27,
            Self::Snapshot => 30,
            Self::Blob => 40,
            Self::IndexUpdate => 50,
            Self::RecoveryStamp => 7,
        }
    }
}
