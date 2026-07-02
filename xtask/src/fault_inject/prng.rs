#![forbid(unsafe_code)]

//! SplitMix64 PRNG and the deterministic schedule hash used by the fault
//! injection engine.

use crate::fault_inject::report::{FaultOutcome, JournalOutcome};

/// SplitMix64 — tiny, deterministic, well-known 64-bit PRNG used to
/// disambiguate unspecified transient/retry decisions.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SplitMix64(u64);

impl SplitMix64 {
    pub(crate) const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// Advance and return the next `u64`.
    #[inline]
    pub(crate) fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Pull a `u8` value in `0..=max` (inclusive). `max` is interpreted as
    /// `u8::MAX` when larger.
    #[inline]
    pub(crate) fn next_u8_in_range(&mut self, max: u8) -> u8 {
        if max == 0 {
            return 0;
        }
        let raw = self.next_u64();
        // Map raw u64 -> [0, max] inclusive. Bias is at most
        // `u64::MAX % (u8::MAX + 1)` which is at most 255 vs. 2^64-1 —
        // acceptable for fault injection where the PRNG only chooses
        // retry counts or transient/persistent decisions.
        let span = u64::from(max).saturating_add(1);
        let reduced = raw.checked_rem(span).unwrap_or(0);
        u8::try_from(reduced).unwrap_or(0)
    }
}

/// Deterministic splitmix64 fingerprint of `(seed, outcomes, journal)`.
///
/// Pure function with no IO. Used by callers to verify that two
/// `FaultReport`s describe byte-identical runs.
#[must_use]
pub fn compute_schedule_hash(
    seed: u64,
    outcomes: &[FaultOutcome],
    journal: &[JournalOutcome],
) -> u64 {
    // Mix the seed with a deterministic constant so the fingerprint is
    // uncorrelated with the seed itself, then fold the outcomes and
    // journal entries into the hash via SplitMix64.
    let mut prng = SplitMix64::new(seed ^ 0xA5A5_5A5A_DEAD_BEEFu64);
    let mut hash = prng.next_u64();
    for outcome in outcomes {
        for byte in outcome_tag_bytes(outcome) {
            hash = hash.rotate_left(7).wrapping_add(u64::from(*byte));
        }
        hash = hash.wrapping_add(prng.next_u64());
    }
    for entry in journal {
        let label = entry_boundary_label(entry);
        let label_len = u64::try_from(label.len()).unwrap_or(0);
        hash ^= label_len.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        hash = hash.rotate_left(13).wrapping_add(prng.next_u64());
    }
    hash
}

fn outcome_tag_bytes(outcome: &FaultOutcome) -> &'static [u8] {
    match outcome {
        FaultOutcome::Crashed { .. } => b"crashed",
        FaultOutcome::AppendFailed { .. } => b"append_failed",
        FaultOutcome::LockResolved { .. } => b"lock_resolved",
        FaultOutcome::LockExhausted { .. } => b"lock_exhausted",
        FaultOutcome::ActionFailed { .. } => b"action_failed",
        FaultOutcome::TimedOut { .. } => b"timed_out",
        FaultOutcome::Restarted { .. } => b"restarted",
    }
}

fn entry_boundary_label(entry: &JournalOutcome) -> String {
    match entry {
        JournalOutcome::Appended { boundary, .. }
        | JournalOutcome::Missing { boundary, .. }
        | JournalOutcome::Pending { boundary }
        | JournalOutcome::Corrupt { boundary } => boundary.label(),
    }
}
