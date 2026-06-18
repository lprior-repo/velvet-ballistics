#![forbid(unsafe_code)]
//! Pure invariant proof surfaces and metadata types for hydration.
//!
//! Provides:
//! - `hydrate_snapshot_tail_*`: Production-proof surface predicates for snapshot+tail ordering
//! - `hydrate_events_preconditions`, `hydrate_dimensions_positive`: Const preconditions
//! - `TailEventMetadata`: Copy-only metadata for hydration ordering validation
//! - `SnapshotRecoveryInputViolation`: Sum type encoding specific snapshot/tail input violations

use crate::JournalEvent;
use vb_core::RunId;

// ---------------------------------------------------------------------------
// Pure boolean proof surfaces (no side effects, no allocations)
// ---------------------------------------------------------------------------

/// Production proof surface: all tail events carry the expected run identity.
#[must_use]
pub fn hydrate_snapshot_tail_run_matches(
    snapshot: &crate::RunSnapshot,
    tail_events: &[JournalEvent],
    run_id: RunId,
) -> bool {
    snapshot.run == run_id && tail_events.iter().all(|event| event.run_id() == run_id)
}

/// Production proof surface: every tail event seq is strictly after snapshot seq.
#[must_use]
pub fn hydrate_snapshot_tail_seq_after_snapshot(
    snapshot: &crate::RunSnapshot,
    tail_events: &[JournalEvent],
) -> bool {
    tail_events.iter().all(|event| event.seq() > snapshot.seq)
}

/// Production proof surface: at least one recovery datum exists.
#[must_use]
pub fn hydrate_snapshot_tail_has_evidence(
    snapshot: &crate::RunSnapshot,
    tail_events: &[JournalEvent],
) -> bool {
    !tail_events.is_empty() || !snapshot.slots.is_empty() || !snapshot.taint.is_empty()
}

/// Production proof surface: composite preconditions for snapshot+tail hydration.
#[must_use]
pub fn hydrate_snapshot_tail_preconditions(
    snapshot: &crate::RunSnapshot,
    tail_events: &[JournalEvent],
    run_id: RunId,
) -> bool {
    hydrate_snapshot_tail_run_matches(snapshot, tail_events, run_id)
        && hydrate_snapshot_tail_seq_after_snapshot(snapshot, tail_events)
        && hydrate_snapshot_tail_has_evidence(snapshot, tail_events)
}

/// Production proof surface: events-only hydration requires non-empty input.
#[must_use]
pub const fn hydrate_events_preconditions(events: &[JournalEvent]) -> bool {
    !events.is_empty()
}

/// Production proof surface: derived dimensions are positive.
#[must_use]
pub const fn hydrate_dimensions_positive(step_count: u16, slot_count: u16) -> bool {
    step_count > 0 && slot_count > 0
}

// ---------------------------------------------------------------------------
// Metadata types for validation orchestration
// ---------------------------------------------------------------------------

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
    NoRecoveryData {
        run: RunId,
    },
}
