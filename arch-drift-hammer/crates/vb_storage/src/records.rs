#![forbid(unsafe_code)]
//! Durable record types for storage.

use vb_core::{RunId, WorkflowDigest, WorkflowId};

/// Typed interpretation of the persisted run-header status byte.
///
/// The byte values are owned by the runtime status model. Storage keeps this
/// type deliberately lossless so old and future records can be read without
/// changing the persisted `u8` layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct RunHeaderStatus(u8);

/// Known run-header status bytes used by the current runtime model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum KnownRunHeaderStatus {
    /// Run is pending execution.
    Pending,
    /// Run has been accepted.
    Accepted,
    /// Run is active.
    Active,
    /// Run has finished.
    Finished,
}

/// Typed error for a status byte that is not known by this build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownRunHeaderStatus {
    byte: u8,
}

/// Lossless classification of a persisted run-header status byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunHeaderStatusClass {
    /// The byte is known by this build.
    Known(KnownRunHeaderStatus),
    /// The byte is explicit but not known by this build.
    Unknown(u8),
}

impl RunHeaderStatus {
    /// Pending status byte.
    pub const PENDING: Self = Self(0);
    /// Accepted status byte.
    pub const ACCEPTED: Self = Self(1);
    /// Active status byte.
    pub const ACTIVE: Self = Self(2);
    /// Finished status byte.
    pub const FINISHED: Self = Self(3);

    /// Builds a lossless status value from its persisted byte.
    #[must_use]
    pub const fn from_byte(byte: u8) -> Self {
        Self(byte)
    }

    /// Returns the exact byte persisted on disk.
    #[must_use]
    pub const fn as_byte(self) -> u8 {
        self.0
    }

    /// Returns the known status, or a typed unknown-byte error.
    pub const fn known(self) -> Result<KnownRunHeaderStatus, UnknownRunHeaderStatus> {
        match KnownRunHeaderStatus::try_from_byte(self.0) {
            Some(status) => Ok(status),
            None => Err(UnknownRunHeaderStatus { byte: self.0 }),
        }
    }

    /// Classifies the byte without losing unknown values.
    #[must_use]
    pub const fn classify(self) -> RunHeaderStatusClass {
        match KnownRunHeaderStatus::try_from_byte(self.0) {
            Some(status) => RunHeaderStatusClass::Known(status),
            None => RunHeaderStatusClass::Unknown(self.0),
        }
    }
}

impl KnownRunHeaderStatus {
    /// Returns the persisted byte for this known status.
    #[must_use]
    pub const fn as_byte(self) -> u8 {
        match self {
            Self::Pending => 0,
            Self::Accepted => 1,
            Self::Active => 2,
            Self::Finished => 3,
        }
    }

    const fn try_from_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Pending),
            1 => Some(Self::Accepted),
            2 => Some(Self::Active),
            3 => Some(Self::Finished),
            _ => None,
        }
    }
}

impl UnknownRunHeaderStatus {
    /// Returns the unknown persisted byte.
    #[must_use]
    pub const fn byte(self) -> u8 {
        self.byte
    }
}

impl From<KnownRunHeaderStatus> for RunHeaderStatus {
    fn from(status: KnownRunHeaderStatus) -> Self {
        Self(status.as_byte())
    }
}

impl From<RunHeaderStatus> for u8 {
    fn from(status: RunHeaderStatus) -> Self {
        status.as_byte()
    }
}

impl TryFrom<u8> for KnownRunHeaderStatus {
    type Error = UnknownRunHeaderStatus;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        Self::try_from_byte(byte).ok_or(UnknownRunHeaderStatus { byte })
    }
}

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
    /// Persisted status byte owned by the runtime status model.
    ///
    /// Use [`RunHeaderRecord::run_header_status`] and
    /// [`RunHeaderRecord::set_run_header_status`] at typed boundaries. This
    /// field remains a `u8` to preserve the existing storage wire format.
    pub status: u8,
    /// Admission timestamp in milliseconds supplied by the caller.
    pub accepted_at_ms: u64,
}

impl RunHeaderRecord {
    /// Returns the status as a typed, lossless value.
    #[must_use]
    pub const fn run_header_status(&self) -> RunHeaderStatus {
        RunHeaderStatus::from_byte(self.status)
    }

    /// Replaces the persisted status byte from a typed status value.
    pub fn set_run_header_status(&mut self, status: RunHeaderStatus) {
        self.status = status.as_byte();
    }
}

/// Large payload blob record.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BlobRecord {
    /// Blob digest key.
    pub digest: [u8; crate::constants::DIGEST_BYTES],
    /// Bounded blob payload.
    pub bytes: Vec<u8>,
}

#[cfg(test)]
mod run_header_status_tests {
    use super::{
        KnownRunHeaderStatus, RunHeaderStatus, RunHeaderStatusClass, UnknownRunHeaderStatus,
    };

    #[test]
    fn run_header_status_known_bytes_classify_as_known_statuses() {
        let cases = [
            (0, KnownRunHeaderStatus::Pending),
            (1, KnownRunHeaderStatus::Accepted),
            (2, KnownRunHeaderStatus::Active),
            (3, KnownRunHeaderStatus::Finished),
        ];

        for (byte, expected) in cases {
            let status = RunHeaderStatus::from_byte(byte);

            assert_eq!(status.as_byte(), byte);
            assert_eq!(status.known(), Ok(expected));
            assert_eq!(status.classify(), RunHeaderStatusClass::Known(expected));
            assert_eq!(RunHeaderStatus::from(expected).as_byte(), byte);
        }
    }

    #[test]
    fn run_header_status_unknown_byte_returns_typed_error_and_lossless_unknown() {
        let status = RunHeaderStatus::from_byte(255);

        assert_eq!(status.known(), Err(UnknownRunHeaderStatus { byte: 255 }));
        assert_eq!(status.classify(), RunHeaderStatusClass::Unknown(255));
        assert_eq!(status.as_byte(), 255);
    }

    #[test]
    fn run_header_status_known_try_from_rejects_unknown_byte() {
        assert_eq!(
            KnownRunHeaderStatus::try_from(9),
            Err(UnknownRunHeaderStatus { byte: 9 })
        );
    }
}
