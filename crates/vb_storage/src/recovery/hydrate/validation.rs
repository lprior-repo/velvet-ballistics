#![forbid(unsafe_code)]
//! Validation orchestration and error mapping for hydration inputs.
//!
//! Provides:
//! - `validate_snapshot_recovery_inputs`: Top-level validation pipeline
//! - `validate_snapshot_metadata`, `validate_tail_run_metadata`, `validate_tail_seq_after_snapshot`, `validate_recovery_data_present`: Const validation functions
//! - `snapshot_input_violation_to_error`: Maps violation enums to typed `RecoveryError`

use crate::recovery::hydrate::invariants::{SnapshotRecoveryInputViolation, TailEventMetadata};
use crate::recovery::{RecoveryError, RecoveryResult};
use crate::{JournalEvent, RunSnapshot};
use vb_core::{RunId, StepIdx};

/// Top-level validation: run identity, sequence ordering, and data presence.
pub fn validate_snapshot_recovery_inputs(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
    run_id: RunId,
) -> RecoveryResult<()> {
    validate_snapshot_metadata(snapshot.run, snapshot.seq, run_id)
        .map_err(snapshot_input_violation_to_error)?;
    validate_tail_events_match_run(tail_events, run_id)?;
    validate_tail_events_after_snapshot(tail_events, snapshot)?;
    validate_recovery_data_present(
        tail_events.is_empty(),
        snapshot.slots.is_empty(),
        snapshot.taint.is_empty(),
        run_id,
    )
    .map_err(snapshot_input_violation_to_error)
}

pub fn validate_snapshot_metadata(
    snapshot_run: RunId,
    snapshot_seq: crate::EventSeq,
    run_id: RunId,
) -> Result<(), SnapshotRecoveryInputViolation> {
    if snapshot_run.get() == run_id.get() {
        Ok(())
    } else {
        Err(SnapshotRecoveryInputViolation::SnapshotRunMismatch {
            snapshot_run,
            snapshot_seq,
        })
    }
}

pub(crate) fn validate_tail_events_match_run(
    tail_events: &[JournalEvent],
    run_id: RunId,
) -> RecoveryResult<()> {
    for event in tail_events {
        validate_tail_run_metadata(TailEventMetadata::from_event(event), run_id)
            .map_err(snapshot_input_violation_to_error)?;
    }
    Ok(())
}

pub fn validate_tail_run_metadata(
    event: TailEventMetadata,
    run_id: RunId,
) -> Result<(), SnapshotRecoveryInputViolation> {
    if event.run.get() == run_id.get() {
        Ok(())
    } else {
        Err(SnapshotRecoveryInputViolation::TailRunMismatch {
            expected: run_id,
            actual: event.run,
        })
    }
}

pub fn validate_tail_events_after_snapshot(
    tail_events: &[JournalEvent],
    snapshot: &RunSnapshot,
) -> RecoveryResult<()> {
    validate_tail_first_seq_contiguous_with_snapshot(tail_events, snapshot.seq)
        .map_err(snapshot_input_violation_to_error)?;
    for event in tail_events {
        validate_tail_seq_after_snapshot(TailEventMetadata::from_event(event), snapshot.seq)
            .map_err(snapshot_input_violation_to_error)?;
    }
    Ok(())
}

/// SR-006: enforce that the first tail event seq is strictly after
/// `snapshot.seq`. A gap between snapshot and tail is permitted when the
/// journal skipped events that landed inside the snapshot itself; rejecting
/// only happens when the first tail event is at or before the snapshot seq.
pub fn validate_tail_first_seq_contiguous_with_snapshot(
    tail_events: &[JournalEvent],
    snapshot_seq: crate::EventSeq,
) -> Result<(), SnapshotRecoveryInputViolation> {
    let Some(first) = tail_events.first() else {
        return Ok(());
    };
    if first.seq() > snapshot_seq {
        Ok(())
    } else {
        Err(
            SnapshotRecoveryInputViolation::TailSeqNotContiguousWithSnapshot {
                snapshot_seq,
                actual_seq: first.seq(),
            },
        )
    }
}

pub fn validate_tail_seq_after_snapshot(
    event: TailEventMetadata,
    snapshot_seq: crate::EventSeq,
) -> Result<(), SnapshotRecoveryInputViolation> {
    if event.seq.get() > snapshot_seq.get() {
        Ok(())
    } else {
        Err(SnapshotRecoveryInputViolation::TailSeqNotAfterSnapshot {
            snapshot_seq,
            actual_seq: event.seq,
        })
    }
}

pub fn validate_recovery_data_present(
    tail_events_empty: bool,
    snapshot_slots_empty: bool,
    snapshot_taint_empty: bool,
    run_id: RunId,
) -> Result<(), SnapshotRecoveryInputViolation> {
    if tail_events_empty && snapshot_slots_empty && snapshot_taint_empty {
        Err(SnapshotRecoveryInputViolation::NoRecoveryData { run: run_id })
    } else {
        Ok(())
    }
}

/// Maps each specific input violation to its domain error variant.
pub(crate) fn snapshot_input_violation_to_error(
    violation: SnapshotRecoveryInputViolation,
) -> RecoveryError {
    match violation {
        SnapshotRecoveryInputViolation::SnapshotRunMismatch {
            snapshot_run,
            snapshot_seq,
        } => RecoveryError::CorruptSnapshot {
            run: snapshot_run,
            seq: snapshot_seq,
        },
        SnapshotRecoveryInputViolation::TailRunMismatch { expected, actual } => {
            RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: format!(
                    "tail event run_id mismatch: expected {expected:?}, found {actual:?}"
                ),
            }
        }
        SnapshotRecoveryInputViolation::TailSeqNotAfterSnapshot {
            snapshot_seq,
            actual_seq,
        } => RecoveryError::ReplayDivergence {
            step: StepIdx::ZERO,
            detail: format!(
                "tail event seq {} is not after snapshot seq {}",
                actual_seq.get(),
                snapshot_seq.get(),
            ),
        },
        SnapshotRecoveryInputViolation::TailSeqNotContiguousWithSnapshot {
            snapshot_seq,
            actual_seq,
        } => RecoveryError::ReplayDivergence {
            step: StepIdx::ZERO,
            detail: format!(
                "tail event seq {} is not contiguous with snapshot seq {} (expected {})",
                actual_seq.get(),
                snapshot_seq.get(),
                snapshot_seq
                    .get()
                    .checked_add(1)
                    .map_or(u64::MAX, |value| value),
            ),
        },
        SnapshotRecoveryInputViolation::NoRecoveryData { run } => {
            RecoveryError::NoRecoveryData { run }
        }
    }
}
