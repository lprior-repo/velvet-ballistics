#![forbid(unsafe_code)]
//! Deterministic boundary-decision transcript.
//!
//! Records every [`crate::scheduler::BoundaryDecision`] the scheduler
//! emits, plus the index and choice that produced it, so callers can
//! replay the boundary exploration deterministically without
//! re-executing the runtime.
//!
//! TODO(vb-awxlm): when vb-wy33p.6 boundary transcript lands, swap
//! this local transcript for the production `BoundaryTranscript` at
//! scheduler construction time. Until then, this module is the
//! canonical replay artifact for the scheduler facade.

use vb_core::ids::StepIdx;

use crate::scheduler::types::{BoundaryChoice, BoundaryDecision};

/// One entry in the scheduler's boundary transcript.
///
/// `Clone` only (not `Copy`) because both [`BoundaryChoice`] and
/// [`BoundaryDecision`] carry a [`RuntimeError`] which is itself
/// `Clone`-only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryTranscriptEntry {
    /// Monotonically increasing step index assigned by the scheduler.
    pub step_index: u32,
    /// Boundary choice that was submitted for this step.
    pub choice: BoundaryChoice,
    /// Boundary decision emitted by the scheduler.
    pub decision: BoundaryDecision,
    /// PRNG state **after** this decision was made. Useful for
    /// reproducing divergent runs without re-executing the scheduler.
    pub post_rng_state: u64,
    /// Snapshot of the active step at the time of the decision, if
    /// the caller supplied a step context via [`BoundaryChoice`].
    pub observed_step: Option<StepIdx>,
}

/// Append-only transcript of scheduler boundary decisions.
///
/// The transcript is the canonical replay artifact: two schedulers
/// initialized with the same seed, same `SchedulerConfig`, and same
/// input choice stream produce identical transcripts (modulo
/// per-entry timestamps, which the scheduler does not record).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BoundaryTranscript {
    entries: Vec<BoundaryTranscriptEntry>,
}

impl BoundaryTranscript {
    /// Creates an empty transcript.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Appends a single decision to the transcript.
    ///
    /// This is the only mutating method on the transcript, so the
    /// transcript is append-only by construction.
    pub fn record(
        &mut self,
        step_index: u32,
        choice: BoundaryChoice,
        decision: BoundaryDecision,
        post_rng_state: u64,
        observed_step: Option<StepIdx>,
    ) {
        self.entries.push(BoundaryTranscriptEntry {
            step_index,
            choice,
            decision,
            post_rng_state,
            observed_step,
        });
    }

    /// Returns the number of entries currently recorded.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether the transcript has no entries yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the entries as a slice, in insertion order.
    #[must_use]
    pub fn as_slice(&self) -> &[BoundaryTranscriptEntry] {
        &self.entries
    }

    /// Returns the entry at `index`, if any.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&BoundaryTranscriptEntry> {
        self.entries.get(index)
    }

    /// Returns the last entry, if any.
    #[must_use]
    pub fn last(&self) -> Option<&BoundaryTranscriptEntry> {
        self.entries.last()
    }

    /// Returns the variant tag sequence of all decisions, in order.
    ///
    /// Each entry is the discriminant index of [`BoundaryDecision`]:
    /// 0 = Advance, 1 = Yield, 2 = Fail, 3 = Retry. This is the
    /// compact form used by tests to compare transcripts.
    #[must_use]
    pub fn decision_variants(&self) -> Vec<u8> {
        self.entries
            .iter()
            .map(|entry| match entry.decision {
                BoundaryDecision::Advance => 0u8,
                BoundaryDecision::Yield { .. } => 1u8,
                BoundaryDecision::Fail { .. } => 2u8,
                BoundaryDecision::Retry { .. } => 3u8,
            })
            .collect()
    }

    /// Resets the transcript to empty.
    ///
    /// **Append-only in production**: this method exists for test
    /// fixtures that want to reuse a scheduler across runs and is
    /// not part of the production transcript contract. Production
    /// callers should construct a fresh `BoundaryTranscript` via
    /// [`Self::new`] rather than mutating an existing one.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
