#![forbid(unsafe_code)]

//! Drive-loop recovery state: tracks evidence gaps surfaced by
//! `read_slot` failures so the drive loop can observe them instead of
//! silently swallowing the underlying read errors (RE-004).

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
}

impl DriveState {
    /// Creates a fresh drive state with no recorded gaps.
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self { evidence_gaps: 0 }
    }

    /// Records one evidence gap. RE-004 calls this when
    /// `emit_slot_evidence` cannot read the slot it would emit
    /// evidence for. Saturates on overflow so this method is
    /// always safe to call.
    pub(crate) fn record_evidence_gap(&mut self) {
        self.evidence_gaps = self.evidence_gaps.saturating_add(1);
    }

    /// Returns the total number of recorded evidence gaps.
    #[must_use]
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) const fn evidence_gaps(&self) -> u64 {
        self.evidence_gaps
    }
}
