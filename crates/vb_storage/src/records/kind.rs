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
    /// Action scheduled event carrying the full replay ticket.
    ActionScheduledTicket = 34,
    /// Action completed event.
    ActionCompleted = 14,
    /// Action completed event carrying the full durable completion envelope.
    ActionCompletedEnvelope = 35,
    /// Full runtime state at a retry boundary.
    RetryCheckpoint = 37,
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
    /// Step succeeded event.
    ///
    /// Distinct from `SlotWritten = 12` because a successful step transition
    /// may be recorded without carrying the encoded slot-value payload shape.
    StepSucceeded = 33,
    /// Run cancelled event.
    RunCancelled = 21,
    /// Run killed event.
    RunKilled = 28,
    /// Ask timed out event.
    AskTimedOut = 29,
    /// Wait resolved event.
    ///
    /// Distinct from `RetryScheduled = 19` because a wait resolution is the
    /// resumption of a suspended run, not a retry attempt. See bug-hunt RE-009.
    WaitResolved = 31,
    /// Action abandoned event.
    ///
    /// Emitted when an in-flight `ActionScheduledTicket` is dropped
    /// because the run was cancelled or killed before the action
    /// boundary completed. Distinct from `ActionFailed = 15` because
    /// the action itself never returned a typed failure code: the run
    /// terminated the suspension.
    ActionAbandoned = 32,
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
    /// Retained typed run inputs (appended to preserve legacy Serde ordinals).
    RunInputs = 4,
    /// Admission receipt carrying a canonical typed input digest.
    RunAdmissionBound = 36,
    /// Policy decision record.
    PolicyDecision = 38,
    /// Admission receipt carrying actual policy identity pin (unbound input).
    RunAdmissionPinned = 39,
    /// Admission receipt carrying input digest and actual policy identity pin.
    RunAdmissionBoundPinned = 41,
    /// Complete versioned owned inputs; appended to preserve old Serde ordinals.
    RunOwnedInputs = 42,
    /// Bounded cold continuation carrier; appended to preserve every old ordinal.
    RunContinuation = 43,
    /// Manual gate outcome record; appended to preserve every old ordinal.
    GateDecision = 44,
    /// Journaled §77.17 compatibility decision; appended, no old ordinal moved.
    CompatibilityDecision = 45,
    /// Journaled §77.10 capability grant; appended, no old ordinal moved.
    CapabilityGrant = 46,
    /// Run admission receipt carrying the admitted aggregate budget (§77.18);
    /// appended, no old ordinal moved.
    RunAdmissionBudget = 47,
    /// Manual gate opened (§77.15); appended, no old ordinal moved.
    GateOpened = 48,
    /// Tenant-scoped admission receipt (§77.18); appended, no old ordinal moved.
    TenantAdmission = 49,
    /// Versioned action-failure checkpoint and retry deadline.
    RetryTimerScheduled = 51,
}

impl RecordKind {
    /// Returns the wire identifier.
    #[must_use]
    pub const fn id(self) -> u16 {
        match self {
            Self::WorkflowSource => 1,
            Self::CompiledIr => 2,
            Self::RunHeader => 3,
            Self::RunInputs => 4,
            Self::RunAdmissionBound => 36,
            Self::PolicyDecision => 38,
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
            Self::RunKilled => 28,
            Self::AskTimedOut => 29,
            Self::Snapshot => 30,
            Self::WaitResolved => 31,
            Self::ActionAbandoned => 32,
            Self::StepSucceeded => 33,
            Self::ActionScheduledTicket => 34,
            Self::ActionCompletedEnvelope => 35,
            Self::RetryCheckpoint => 37,
            Self::RunAdmissionPinned => 39,
            Self::RunAdmissionBoundPinned => 41,
            Self::RunOwnedInputs => 42,
            Self::RunContinuation => 43,
            Self::GateDecision => 44,
            Self::CompatibilityDecision => 45,
            Self::CapabilityGrant => 46,
            Self::RunAdmissionBudget => 47,
            Self::GateOpened => 48,
            Self::TenantAdmission => 49,
            Self::RetryTimerScheduled => 51,
            Self::Blob => 40,
            Self::IndexUpdate => 50,
        }
    }

    /// Returns the typed record kind for a stable wire identifier.
    #[must_use]
    pub const fn from_id(id: u16) -> Option<Self> {
        match id {
            1 => Some(Self::WorkflowSource),
            2 => Some(Self::CompiledIr),
            3 => Some(Self::RunHeader),
            4 => Some(Self::RunInputs),
            36 => Some(Self::RunAdmissionBound),
            38 => Some(Self::PolicyDecision),
            10 => Some(Self::RunAccepted),
            11 => Some(Self::StepStarted),
            12 => Some(Self::SlotWritten),
            13 => Some(Self::ActionScheduled),
            14 => Some(Self::ActionCompleted),
            15 => Some(Self::ActionFailed),
            16 => Some(Self::WaitScheduled),
            17 => Some(Self::AskScheduled),
            18 => Some(Self::AskAnswered),
            19 => Some(Self::RetryScheduled),
            20 => Some(Self::StepFailed),
            21 => Some(Self::RunCancelled),
            22 => Some(Self::RunFinished),
            23 => Some(Self::RunFailed),
            24 => Some(Self::RunAdmission),
            25 => Some(Self::RunResumed),
            26 => Some(Self::RunRetried),
            27 => Some(Self::RunAnswered),
            28 => Some(Self::RunKilled),
            29 => Some(Self::AskTimedOut),
            30 => Some(Self::Snapshot),
            31 => Some(Self::WaitResolved),
            32 => Some(Self::ActionAbandoned),
            33 => Some(Self::StepSucceeded),
            34 => Some(Self::ActionScheduledTicket),
            35 => Some(Self::ActionCompletedEnvelope),
            37 => Some(Self::RetryCheckpoint),
            39 => Some(Self::RunAdmissionPinned),
            41 => Some(Self::RunAdmissionBoundPinned),
            42 => Some(Self::RunOwnedInputs),
            43 => Some(Self::RunContinuation),
            44 => Some(Self::GateDecision),
            45 => Some(Self::CompatibilityDecision),
            46 => Some(Self::CapabilityGrant),
            47 => Some(Self::RunAdmissionBudget),
            48 => Some(Self::GateOpened),
            49 => Some(Self::TenantAdmission),
            51 => Some(Self::RetryTimerScheduled),
            40 => Some(Self::Blob),
            50 => Some(Self::IndexUpdate),
            _ => None,
        }
    }

    /// Returns true when this kind belongs to the durable journal-event family.
    #[must_use]
    pub const fn is_journal_event(self) -> bool {
        match self {
            Self::RunAccepted
            | Self::StepStarted
            | Self::SlotWritten
            | Self::ActionScheduled
            | Self::ActionCompleted
            | Self::ActionFailed
            | Self::WaitScheduled
            | Self::AskScheduled
            | Self::AskAnswered
            | Self::RetryScheduled
            | Self::StepFailed
            | Self::RunCancelled
            | Self::RunFinished
            | Self::RunFailed
            | Self::RunAdmission
            | Self::RunAdmissionBound
            | Self::RunAdmissionPinned
            | Self::RunAdmissionBoundPinned
            | Self::RunResumed
            | Self::RunRetried
            | Self::RunAnswered
            | Self::RunKilled
            | Self::AskTimedOut
            | Self::WaitResolved
            | Self::ActionAbandoned
            | Self::StepSucceeded
            | Self::ActionScheduledTicket
            | Self::ActionCompletedEnvelope
            | Self::RetryCheckpoint
            | Self::PolicyDecision
            | Self::GateDecision
            | Self::CompatibilityDecision
            | Self::CapabilityGrant
            | Self::RunAdmissionBudget
            | Self::GateOpened
            | Self::TenantAdmission
            | Self::RetryTimerScheduled
            | Self::RunContinuation => true,
            Self::WorkflowSource
            | Self::CompiledIr
            | Self::RunHeader
            | Self::RunInputs
            | Self::RunOwnedInputs
            | Self::Snapshot
            | Self::Blob
            | Self::IndexUpdate => false,
        }
    }

    /// Returns true when this kind is valid for the supplied record-family magic.
    #[must_use]
    pub const fn belongs_to_magic(self, magic: u32) -> bool {
        match magic {
            crate::constants::MAGIC_WORKFLOW_SOURCE => matches!(self, Self::WorkflowSource),
            crate::constants::MAGIC_COMPILED_ARTIFACT => matches!(self, Self::CompiledIr),
            crate::constants::MAGIC_JOURNAL_EVENT => self.is_journal_event(),
            crate::constants::MAGIC_SNAPSHOT => matches!(self, Self::Snapshot),
            crate::constants::MAGIC_BLOB => matches!(self, Self::Blob),
            crate::constants::MAGIC_INDEX_RECORD => {
                matches!(
                    self,
                    Self::RunHeader | Self::RunInputs | Self::RunOwnedInputs | Self::IndexUpdate
                )
            }
            _ => false,
        }
    }
}
