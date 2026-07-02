#![forbid(unsafe_code)]

//! Drive-loop recovery state: tracks evidence gaps surfaced by
//! `read_slot` failures so the drive loop can observe them instead of
//! silently swallowing the underlying read errors (RE-004).

use vb_core::ids::SlotIdx;

/// Typed report returned by [`DriveState::take_evidence_gaps_report`]
/// at the end of `drive_deterministic_full` so operators can observe
/// `read_slot` failures (RE-004 observability follow-up; previously
/// the counter was only visible in tests).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct EvidenceGapReport {
    /// Number of evidence gaps recorded since the drive started.
    pub count: u64,
    /// Slot index of the most recent gap, or `None` when no gaps were
    /// recorded.
    pub last_slot: Option<SlotIdx>,
}

/// Drive-loop state threaded through `finish_drive_step` and
/// `emit_slot_evidence` so evidence gaps (read_slot errors) can be
/// surfaced to the drive loop instead of silently swallowed.
///
/// RE-004: the original `if let Ok(value) = run.read_slot(slot)`
/// pattern dropped both the slot evidence AND the underlying read
/// error. `record_evidence_gap` is the typed channel that the
/// drive loop owns; callers can later expose or log the counter
/// without changing the public signature of `drive_deterministic_full`.
#[derive(Debug, Clone, Default)]
pub(crate) struct DriveState {
    evidence_gaps: u64,
    last_gap_slot: Option<SlotIdx>,
}

impl DriveState {
    /// Creates a fresh drive state with no recorded gaps.
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            evidence_gaps: 0,
            last_gap_slot: None,
        }
    }

    /// Records one evidence gap for `slot`. RE-004 calls this when
    /// `emit_slot_evidence` cannot read the slot it would emit
    /// evidence for. Saturates on overflow so this method is
    /// always safe to call.
    pub(crate) fn record_evidence_gap(&mut self, slot: SlotIdx) {
        self.evidence_gaps = self.evidence_gaps.saturating_add(1);
        self.last_gap_slot = Some(slot);
    }

    /// Returns the total number of recorded evidence gaps.
    #[must_use]
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) const fn evidence_gaps(&self) -> u64 {
        self.evidence_gaps
    }

    /// Consumes the gap counter and returns a typed report.
    ///
    /// This is the operator-visible surface for RE-004 evidence gaps
    /// (black-hat FINDING-002). Callers should invoke this at the end
    /// of the drive loop and route the report through their existing
    /// error channel; the counter itself is reset to zero so the
    /// same `DriveState` can be reused for the next drive.
    #[must_use]
    pub(crate) const fn take_evidence_gaps_report(&mut self) -> EvidenceGapReport {
        let report = EvidenceGapReport {
            count: self.evidence_gaps,
            last_slot: self.last_gap_slot,
        };
        self.evidence_gaps = 0;
        self.last_gap_slot = None;
        report
    }
}
