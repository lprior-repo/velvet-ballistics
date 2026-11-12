#![forbid(unsafe_code)]

use crate::JournalEvent;
use crate::recovery::types::RunSnapshot;
use vb_core::RunId;

/// Production proof surface for snapshot-plus-tail run identity.
#[must_use]
pub fn hydrate_snapshot_tail_run_matches(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
    run_id: RunId,
) -> bool {
    snapshot.run == run_id && tail_events.iter().all(|event| event.run_id() == run_id)
}

/// Production proof surface for snapshot-plus-tail sequence ordering.
#[must_use]
pub fn hydrate_snapshot_tail_seq_after_snapshot(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
) -> bool {
    tail_events.iter().all(|event| event.seq() > snapshot.seq)
}

/// Production proof surface for non-empty recovery evidence.
#[must_use]
pub fn hydrate_snapshot_tail_has_evidence(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
) -> bool {
    !tail_events.is_empty() || !snapshot.slots.is_empty() || !snapshot.taint.is_empty()
}

/// Production proof surface for hydrate_run_frame preconditions that do not decode bytes.
#[must_use]
pub fn hydrate_snapshot_tail_preconditions(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
    run_id: RunId,
) -> bool {
    hydrate_snapshot_tail_run_matches(snapshot, tail_events, run_id)
        && hydrate_snapshot_tail_seq_after_snapshot(snapshot, tail_events)
        && hydrate_snapshot_tail_has_evidence(snapshot, tail_events)
}

/// Production proof surface for events-only hydrate preconditions.
#[must_use]
pub const fn hydrate_events_preconditions(events: &[JournalEvent]) -> bool {
    !events.is_empty()
}

/// Production proof surface for positive frame dimensions.
#[must_use]
pub const fn hydrate_dimensions_positive(step_count: u16, slot_count: u16) -> bool {
    step_count > 0 && slot_count > 0
}

/// Copy-only metadata needed to validate snapshot/tail hydration ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TailEventMetadata {
    pub(crate) run: RunId,
    pub(crate) seq: crate::EventSeq,
}

impl TailEventMetadata {
    #[must_use]
    pub(crate) const fn new(run: RunId, seq: crate::EventSeq) -> Self {
        Self { run, seq }
    }

    #[must_use]
    pub(crate) const fn from_event(event: &JournalEvent) -> Self {
        Self::new(event.run_id(), event.seq())
    }
}

/// Allocation-free classification for snapshot/tail hydration preconditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SnapshotRecoveryInputViolation {
    SnapshotRunMismatch {
        expected_run: RunId,
        snapshot_run: RunId,
        snapshot_seq: crate::EventSeq,
    },
    TailRunMismatch {
        expected: RunId,
        actual: RunId,
    },
    TailSeqNotAfterSnapshot {
        snapshot_seq: crate::EventSeq,
        actual_seq: crate::EventSeq,
    },
    /// First tail event seq is not contiguous with snapshot seq.
    TailSeqGap {
        snapshot_seq: crate::EventSeq,
        actual_seq: crate::EventSeq,
    },
    NoRecoveryData {
        run: RunId,
    },
}

pub(crate) const fn validate_snapshot_metadata(
    snapshot_run: RunId,
    snapshot_seq: crate::EventSeq,
    run_id: RunId,
) -> Result<(), SnapshotRecoveryInputViolation> {
    if snapshot_run.get() == run_id.get() {
        Ok(())
    } else {
        Err(SnapshotRecoveryInputViolation::SnapshotRunMismatch {
            expected_run: run_id,
            snapshot_run,
            snapshot_seq,
        })
    }
}

pub(crate) const fn validate_tail_run_metadata(
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

pub(crate) const fn validate_tail_seq_after_snapshot(
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

pub(crate) const fn validate_recovery_data_present(
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
