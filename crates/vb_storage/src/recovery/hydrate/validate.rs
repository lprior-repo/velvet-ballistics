#![forbid(unsafe_code)]

use super::preconditions::{
    SnapshotRecoveryInputViolation, TailEventMetadata, validate_recovery_data_present,
    validate_snapshot_metadata, validate_tail_run_metadata, validate_tail_seq_after_snapshot,
};

use crate::JournalEvent;
use crate::recovery::types::{RecoveryError, RecoveryResult, RunSnapshot};
use vb_core::RunId;

pub(crate) fn validate_snapshot_recovery_inputs(
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

fn validate_tail_events_match_run(
    tail_events: &[JournalEvent],
    run_id: RunId,
) -> RecoveryResult<()> {
    for event in tail_events {
        validate_tail_run_metadata(TailEventMetadata::from_event(event), run_id)
            .map_err(snapshot_input_violation_to_error)?;
    }
    Ok(())
}

fn validate_tail_events_after_snapshot(
    tail_events: &[JournalEvent],
    snapshot: &RunSnapshot,
) -> RecoveryResult<()> {
    // Contiguity gate: first tail event must be snapshot.seq + 1.
    // This catches gaps between snapshot and tail that the
    // per-event ordering check misses.
    if let Some(first) = tail_events.first() {
        let first_seq = first.seq();
        let expected = snapshot
            .seq
            .get()
            .checked_add(1)
            .ok_or(SnapshotRecoveryInputViolation::TailSeqGap {
                snapshot_seq: snapshot.seq,
                actual_seq: snapshot.seq,
            })
            .map_err(snapshot_input_violation_to_error)?;
        if first_seq.get() != expected {
            return Err(snapshot_input_violation_to_error(
                SnapshotRecoveryInputViolation::TailSeqGap {
                    snapshot_seq: snapshot.seq,
                    actual_seq: first_seq,
                },
            ));
        }
    }
    for event in tail_events {
        validate_tail_seq_after_snapshot(TailEventMetadata::from_event(event), snapshot.seq)
            .map_err(snapshot_input_violation_to_error)?;
    }
    Ok(())
}

fn snapshot_input_violation_to_error(violation: SnapshotRecoveryInputViolation) -> RecoveryError {
    match violation {
        SnapshotRecoveryInputViolation::SnapshotRunMismatch {
            expected_run,
            snapshot_seq,
            ..
        } => RecoveryError::CorruptSnapshot {
            run: expected_run,
            seq: snapshot_seq,
        },
        SnapshotRecoveryInputViolation::TailRunMismatch { expected, actual } => {
            RecoveryError::ReplayDivergence {
                step: vb_core::StepIdx::ZERO,
                detail: format!(
                    "tail event run_id mismatch: expected {expected:?}, found {actual:?}"
                ),
            }
        }
        SnapshotRecoveryInputViolation::TailSeqNotAfterSnapshot {
            snapshot_seq,
            actual_seq,
        } => RecoveryError::ReplayDivergence {
            step: vb_core::StepIdx::ZERO,
            detail: format!(
                "tail event seq {} is not after snapshot seq {}",
                actual_seq.get(),
                snapshot_seq.get()
            ),
        },
        SnapshotRecoveryInputViolation::TailSeqGap {
            snapshot_seq,
            actual_seq,
        } => RecoveryError::ReplayDivergence {
            step: vb_core::StepIdx::ZERO,
            detail: format!(
                "tail event seq {} is not contiguous with snapshot seq {} (gap detected)",
                actual_seq.get(),
                snapshot_seq.get()
            ),
        },
        SnapshotRecoveryInputViolation::NoRecoveryData { run } => {
            RecoveryError::NoRecoveryData { run }
        }
    }
}
