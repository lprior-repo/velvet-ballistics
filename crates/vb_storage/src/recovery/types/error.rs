#![forbid(unsafe_code)]
//! Recovery error types with typed diagnostics.

use crate::{EventSeq, JournalError};
use serde::{Deserialize, Serialize};
use vb_core::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};

/// Recovery failures with typed diagnostics.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RecoveryError {
    /// Journal operation failed during recovery.
    #[error("journal error during recovery: {0}")]
    Journal(#[from] JournalError),
    /// Workflow source digest does not match the stored record.
    #[error("workflow source digest mismatch: expected {expected:?}, found {found:?}")]
    WorkflowSourceDigestMismatch {
        /// Expected digest.
        expected: WorkflowDigest,
        /// Found digest.
        found: WorkflowDigest,
    },
    /// Compiled IR digest does not match the stored record.
    #[error("compiled IR digest mismatch: expected {expected:?}, found {found:?}")]
    CompiledIrDigestMismatch {
        /// Expected digest.
        expected: WorkflowDigest,
        /// Found digest.
        found: WorkflowDigest,
    },
    /// Action ABI digest mismatch during recovery.
    #[error(
        "action ABI digest mismatch for action {action_id:?}: expected {expected:?}, found {found:?}"
    )]
    ActionAbiMismatch {
        /// Action with mismatched ABI.
        action_id: ActionId,
        /// Expected digest.
        expected: WorkflowDigest,
        /// Found digest.
        found: WorkflowDigest,
    },
    /// Policy digest mismatch during recovery.
    #[error("policy digest mismatch for step {step:?}: expected {expected:?}, found {found:?}")]
    PolicyDigestMismatch {
        /// Step where policy diverged.
        step: StepIdx,
        /// Expected digest.
        expected: WorkflowDigest,
        /// Found digest.
        found: WorkflowDigest,
    },
    /// Durable admission evidence is absent, so policy digest evidence cannot be read.
    #[error("policy digest unavailable for run {run:?} step {step:?}: expected {expected:?}")]
    PolicyDigestUnavailable {
        /// Run identifier missing durable admission evidence.
        run: RunId,
        /// Step whose policy digest was required.
        step: StepIdx,
        /// Expected digest from recovery caller.
        expected: WorkflowDigest,
    },
    /// Recovery caller did not provide policy expectations for a run missing admission evidence.
    #[error("policy digest expectation missing for run {run:?}")]
    PolicyDigestExpectationMissing {
        /// Run identifier missing both durable admission evidence and caller expectations.
        run: RunId,
    },
    /// Full digest verification was requested without the required digest config.
    #[error("full digest check config missing")]
    FullDigestCheckConfigMissing,
    /// Durable admission evidence names a different artifact than the accepted run.
    #[error(
        "run admission artifact digest mismatch for run {run:?}: expected {expected:?}, found {found:?}"
    )]
    RunAdmissionArtifactDigestMismatch {
        /// Run identifier with divergent admission evidence.
        run: RunId,
        /// Digest from the accepted run evidence.
        expected: WorkflowDigest,
        /// Digest found in the admission event.
        found: WorkflowDigest,
    },
    /// A non-idempotent action was encountered during recovery and cannot be re-executed.
    #[error(
        "non-idempotent action {action:?} at step {step:?} cannot be re-executed during recovery"
    )]
    NonIdempotentActionBlocked {
        /// Action identifier.
        action: ActionId,
        /// Step where the action was scheduled.
        step: StepIdx,
    },
    /// Replay diverged from expected state machine trajectory.
    #[error("replay divergence at step {step:?}: {detail}")]
    ReplayDivergence {
        /// Step where divergence was detected.
        step: StepIdx,
        /// Divergence description.
        detail: String,
    },
    /// Recovery could not read existing slot taint and must fail closed.
    #[error("slot taint read_taint failed for slot {slot:?}")]
    SlotTaintReadFailed {
        /// Slot whose taint could not be read.
        slot: SlotIdx,
    },
    /// Durable slot taint metadata was present but could not be decoded.
    #[error("slot taint metadata corrupt for slot {slot:?}")]
    CorruptSlotTaint {
        /// Slot whose persisted taint metadata was corrupt.
        slot: SlotIdx,
    },
    /// No snapshot or journal events found for run.
    #[error("no recovery data found for run {run:?}")]
    NoRecoveryData {
        /// Run identifier.
        run: RunId,
    },
    /// Snapshot is present but corrupt or unreadable.
    #[error("snapshot corrupt for run {run:?} at seq {seq:?}")]
    CorruptSnapshot {
        /// Run identifier.
        run: RunId,
        /// Snapshot sequence.
        seq: EventSeq,
    },
    /// Recovery produced a terminal state that does not match expectations.
    #[error("recovery terminal state mismatch: expected {expected:?}, found {found:?}")]
    TerminalStateMismatch {
        /// Expected terminal event kind.
        expected: String,
        /// Found terminal event kind.
        found: String,
    },
    /// Durable event indexes exceed the runtime frame dimensions that can be represented.
    #[error("recovery frame dimension overflow for run {run:?}")]
    FrameDimensionOverflow {
        /// Run identifier.
        run: RunId,
    },
}

/// Result alias for recovery operations.
pub type RecoveryResult<T> = Result<T, RecoveryError>;
