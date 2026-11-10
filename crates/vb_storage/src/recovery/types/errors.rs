#![forbid(unsafe_code)]

use crate::{EventSeq, JournalError};
use vb_core::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};

/// Recovery failures with typed diagnostics.
#[derive(Debug)]
#[non_exhaustive]
pub enum RecoveryError {
    /// Journal operation failed during recovery.
    Journal(JournalError),
    /// Workflow source digest does not match the stored record.
    WorkflowSourceDigestMismatch {
        /// Expected digest.
        expected: WorkflowDigest,
        /// Found digest.
        found: WorkflowDigest,
    },
    /// Compiled IR digest does not match the stored record.
    CompiledIrDigestMismatch {
        /// Expected digest.
        expected: WorkflowDigest,
        /// Found digest.
        found: WorkflowDigest,
    },
    /// Action ABI digest mismatch during recovery.
    ActionAbiMismatch {
        /// Action with mismatched ABI.
        action_id: ActionId,
    },
    /// Policy digest mismatch during recovery.
    PolicyDigestMismatch {
        /// Step where policy diverged.
        step: StepIdx,
    },
    /// A non-idempotent action was encountered during recovery and cannot be re-executed.
    NonIdempotentActionBlocked {
        /// Action identifier.
        action: ActionId,
        /// Step where the action was scheduled.
        step: StepIdx,
    },
    /// Replay diverged from expected state machine trajectory.
    ReplayDivergence {
        /// Step where divergence was detected.
        step: StepIdx,
        /// Divergence description.
        detail: String,
    },
    /// Recovery could not read existing slot taint and must fail closed.
    SlotTaintReadFailed {
        /// Slot whose taint could not be read.
        slot: SlotIdx,
    },
    /// Durable slot taint metadata was present but could not be decoded.
    CorruptSlotTaint {
        /// Slot whose persisted taint metadata was corrupt.
        slot: SlotIdx,
    },
    /// No snapshot or journal events found for run.
    NoRecoveryData {
        /// Run identifier.
        run: RunId,
    },
    /// Snapshot is present but corrupt or unreadable.
    CorruptSnapshot {
        /// Run identifier.
        run: RunId,
        /// Snapshot sequence.
        seq: EventSeq,
    },
    /// No snapshot has been written for the requested (run, seq).
    ///
    /// This is distinct from `CorruptSnapshot`: a missing snapshot has no
    /// record in the keyspace (`Ok(None)` from `journal.snapshot`), while a
    /// corrupt snapshot exists but its envelope / postcard payload cannot
    /// be decoded (`Err(JournalError::PostcardDecodeFailed(_))`). Master §18
    /// line 873 requires a typed storage-error surface so the recovery
    /// boundary can pick snapshot-plus-tail recovery (missing) versus
    /// fail-closed on real corruption (corrupt) without conflating them.
    MissingSnapshot {
        /// Run identifier.
        run: RunId,
        /// Snapshot sequence that was requested.
        seq: EventSeq,
    },
    /// Recovery produced a terminal state that does not match expectations.
    TerminalStateMismatch {
        /// Expected terminal event kind.
        expected: String,
        /// Found terminal event kind.
        found: String,
    },
    /// Durable event indexes exceed the runtime frame dimensions that can be represented.
    FrameDimensionOverflow {
        /// Run identifier.
        run: RunId,
    },
    /// Recovery frame seed cannot support a live `RunState` (workflow,
    /// store, action attempts, admission, collect states, action
    /// contracts, or action ABI digests are not present in durable
    /// events). The runtime cannot resume execution from a frame-only
    /// seed; callers must reconcile via summary-only observation or
    /// restart from scratch.
    UnsupportedFrameSeed {
        /// Run identifier.
        run: RunId,
        /// Canonical reason string for the cannot-resume classification.
        reason: String,
    },
    /// Artifact was not found in the store during recovery.
    ArtifactNotFound {
        /// Digest that was looked up.
        digest: WorkflowDigest,
    },
    /// Artifact IR bytes could not be decoded to WorkflowParts.
    ArtifactDecodeFailed,
}

impl From<JournalError> for RecoveryError {
    fn from(inner: JournalError) -> Self {
        Self::Journal(inner)
    }
}

impl Eq for RecoveryError {}

/// Result alias for recovery operations.
pub type RecoveryResult<T> = Result<T, RecoveryError>;
