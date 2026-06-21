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
    /// Wait cancelled event.
    WaitCancelled = 31,
    /// Ask cancelled event.
    AskCancelled = 32,
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
    /// A storage marker written during the recovery process to checkpoint
    /// replay progress. A subsequent recovery invocation can detect a prior
    /// recovery attempt's progress by reading the stamp and resume from
    /// that point instead of re-replaying the full journal.
    ///
    /// Wire ID 7 is a non-journal record kind. It uses its own magic
    /// `MAGIC_RECOVERY_STAMP` (`"VRST"`, `0x5652_5354`) — distinct from
    /// `MAGIC_WORKFLOW_SOURCE` (`"VBSR"`, which is bound to kind 1) and
    /// the journal-event magic (`"VBJE"`) — so that recovery writes can
    /// be admitted and rejected independently of the source/journal paths.
    RecoveryStamp = 7,
}

impl RecordKind {
    /// Returns the wire identifier.
    ///
    /// The wire ID is the `#[repr(u16)]` discriminant declared above; this
    /// method is the single source of truth for serializing a variant to its
    /// on-disk `u16`. Adding a new variant requires picking the correct
    /// discriminant literal — `id()` will follow automatically.
    #[must_use]
    pub const fn id(self) -> u16 {
        self as u16
    }
}
