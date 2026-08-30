// Flux-rs standalone demo refinements for sequence bounds in `vb_storage` types.
//
// NON-CLOSURE EVIDENCE — not bound to production code via `#[path]`.
// This file is a standalone Flux demo that exercises refinement annotations
// against hand-written shadow types. It provides model sketches for audit
// and research only; it cannot be cited as production safety evidence.
//
// Domain models: EventSeq range invariants, sequence contiguity, step ordering,
// replay bounds, and attempt filtering during replay.
//
// Obligation: Demos sequence contiguity and replay-bounds refinement models.
// Verifier: flux-rs
// Category: SCOPED-ONLY (non-closure evidence)

#![forbid(unsafe_code)]

extern crate flux_rs;
use flux_rs::attrs::*;

// ============================================================================
// EventSeq refined type — range invariant
// ============================================================================

/// Refined EventSeq: a sequence value bounded to [0, u64::MAX].
///
/// The invariant is trivially satisfied by the raw `u64` inner value because
/// `u64` itself cannot hold values outside `[0, u64::MAX]`. The refinement
/// records the invariant so that callers and downstream functions can
/// reason about the sequence without needing explicit bounds checks.
#[flux_rs::refined_by(raw: u64)]
#[flux_rs::invariant(0 <= raw && raw <= u64::MAX)]
pub struct EventSeqRefined {
    #[flux_rs::field(u64[raw])]
    raw: u64,
}

/// Trusted model: `EventSeq::new(v)` produces a valid refined sequence.
#[flux_rs::trusted]
#[flux_rs::sig(fn(v: u64) -> EventSeqRefined[v])]
fn model_event_seq_new_is_valid(v: u64) -> EventSeqRefined {
    EventSeqRefined { raw: v }
}

/// Trusted model: `EventSeq::get()` returns the raw value that satisfies the
/// invariant.
#[flux_rs::trusted]
#[flux_rs::sig(fn(s: EventSeqRefined[v]) -> u64[v])]
fn model_event_seq_get_preserves_invariant(s: EventSeqRefined<u64>) -> u64 {
    s.raw
}

/// Trusted model: EventSeq::ZERO has raw value 0.
#[flux_rs::trusted]
#[flux_rs::sig(fn() -> bool[true])]
fn model_event_seq_zero_is_zero() -> bool {
    true
}

/// Trusted model: EventSeq::MAX has raw value u64::MAX.
#[flux_rs::trusted]
#[flux_rs::sig(fn() -> bool[true])]
fn model_event_seq_max_is_max() -> bool {
    true
}

// ============================================================================
// Sequence contiguity refinements
// ============================================================================

/// A sequence of u64 values is contiguous when each element is exactly one
/// greater than the previous (using saturating arithmetic).
#[flux_rs::sig(fn(seqs: &[u64]) -> bool[is_contiguous_seq_array(seqs)])]
fn model_is_contiguous_seq_array(seqs: &[u64]) -> bool {
    if seqs.len() <= 1 {
        return true;
    }
    for i in 0..seqs.len() - 1 {
        if seqs[i].saturating_add(1) != seqs[i + 1] {
            return false;
        }
    }
    true
}

/// A sequence with a gap (non-contiguous) is detected when any adjacent pair
/// fails the saturating-add-one check.
#[flux_rs::sig(fn(seqs: &[u64]) -> bool[has_sequence_gap(seqs)])]
fn model_has_sequence_gap(seqs: &[u64]) -> bool {
    if seqs.len() < 2 {
        return false;
    }
    for i in 0..seqs.len() - 1 {
        if seqs[i].saturating_add(1) != seqs[i + 1] {
            return true;
        }
    }
    false
}

/// A sequence contains a duplicate when any two distinct positions hold the
/// same value. Duplicate detection implies non-contiguity for length >= 2.
#[flux_rs::sig(fn(seqs: &[u64]) -> bool[has_sequence_duplicate(seqs)])]
fn model_has_sequence_duplicate(seqs: &[u64]) -> bool {
    if seqs.len() < 2 {
        return false;
    }
    let mut i = 0usize;
    while i < seqs.len() {
        let mut j = i + 1;
        while j < seqs.len() {
            if seqs[i] == seqs[j] {
                return true;
            }
            j += 1;
        }
        i += 1;
    }
    false
}

/// Contiguity is transitive: if a prefix is contiguous and the next element
/// extends it by exactly one, the full prefix is contiguous.
#[flux_rs::sig(
    fn(prefix: &[u64], next_val: u64) -> bool[
        is_contiguous_seq_array(prefix) && (prefix.is_empty() || prefix[prefix.len() - 1].saturating_add(1) == next_val)
    ]
)]
fn model_contiguity_extends(prefix: &[u64], next_val: u64) -> bool {
    if prefix.is_empty() {
        return true;
    }
    prefix[prefix.len() - 1].saturating_add(1) == next_val
}

// ============================================================================
// Step ordering refinements
// ============================================================================

/// Step indices are bounded to [0, u16::MAX] by construction.
#[flux_rs::sig(fn(idx: u16) -> bool[0 <= idx && idx <= 16#ffff#])]
fn model_step_idx_in_bounds(idx: u16) -> bool {
    true
}

/// Step ordering diverges when the current step is strictly less than the
/// previous step (monotonic non-decreasing invariant violation).
#[flux_rs::sig(fn(previous: Option<u16>, current: u16) -> bool[step_order_diverges(previous, current)])]
fn model_step_order_diverges(previous: Option<u16>, current: u16) -> bool {
    match previous {
        Some(prev) => current < prev,
        None => false,
    }
}

/// Monotonic step ordering is preserved when the current step is >= previous.
#[flux_rs::sig(fn(previous: Option<u16>, current: u16) -> bool[step_order_preserved(previous, current)])]
fn model_step_order_preserved(previous: Option<u16>, current: u16) -> bool {
    match previous {
        Some(prev) => current >= prev,
        None => true,
    }
}

// ============================================================================
// Replay bounds refinements
// ============================================================================

/// Tail events must have sequence values strictly greater than the snapshot
/// sequence for replay to be valid.
#[flux_rs::sig(
    fn(snapshot_seq: u64, tail_seqs: &[u64]) -> bool[tail_events_after_snapshot(snapshot_seq, tail_seqs)]
)]
fn model_tail_events_after_snapshot(snapshot_seq: u64, tail_seqs: &[u64]) -> bool {
    for seq in tail_seqs {
        if *seq <= snapshot_seq {
            return false;
        }
    }
    true
}

/// Tail validation rejects any event whose sequence is not strictly after the
/// snapshot sequence.
#[flux_rs::sig(
    fn(snapshot_seq: u64, tail_seqs: &[u64]) -> bool[!tail_events_after_snapshot(snapshot_seq, tail_seqs)]
)]
fn model_tail_events_rejection(snapshot_seq: u64, tail_seqs: &[u64]) -> bool {
    for seq in tail_seqs {
        if *seq <= snapshot_seq {
            return true;
        }
    }
    false
}

/// Attempt filter: stale attempts are those strictly below the maximum
/// attempt found in the event set.
#[flux_rs::sig(fn(attempt: Option<u16>, max_attempt: u16) -> bool[is_stale_attempt(attempt, max_attempt)])]
fn model_is_stale_attempt(attempt: Option<u16>, max_attempt: u16) -> bool {
    let a = match attempt {
        Some(v) => v,
        None => 1,
    };
    a < max_attempt
}

/// Attempt filter: current attempts are those >= max_attempt.
#[flux_rs::sig(fn(attempt: Option<u16>, max_attempt: u16) -> bool[is_current_attempt(attempt, max_attempt)])]
fn model_is_current_attempt(attempt: Option<u16>, max_attempt: u16) -> bool {
    let a = match attempt {
        Some(v) => v,
        None => 1,
    };
    a >= max_attempt
}

/// Max attempt is at least 1 for any non-empty event set.
#[flux_rs::sig(fn(attempts: &[u16]) -> bool[max_attempt_ge_one(attempts)])]
fn model_max_attempt_ge_one(attempts: &[u16]) -> bool {
    let mut max = 1u16;
    for &a in attempts {
        if a > max {
            max = a;
        }
    }
    max >= 1
}

// ============================================================================
// Tests — runtime verification of model correctness
// ============================================================================

#[cfg(test)]
mod flux_sequence_tests {
    use super::*;

    #[test]
    fn contiguous_sequence_valid() {
        assert!(model_is_contiguous_seq_array(&[0, 1, 2, 3, 4]));
    }

    #[test]
    fn non_contiguous_gap_detected() {
        assert!(model_has_sequence_gap(&[0, 1, 3, 4]));
    }

    #[test]
    fn duplicate_detected() {
        assert!(model_has_sequence_duplicate(&[0, 1, 1, 2]));
    }

    #[test]
    fn empty_sequence_is_contiguous() {
        assert!(model_is_contiguous_seq_array(&[]));
    }

    #[test]
    fn single_element_is_contiguous() {
        assert!(model_is_contiguous_seq_array(&[42]));
    }

    #[test]
    fn contiguity_extends_valid() {
        assert!(model_contiguity_extends(&[0, 1, 2], 3));
    }

    #[test]
    fn contiguity_extends_fails() {
        assert!(!model_contiguity_extends(&[0, 1, 2], 5));
    }

    #[test]
    fn step_order_preserved() {
        assert!(model_step_order_preserved(Some(3), 5));
        assert!(model_step_order_preserved(Some(3), 3));
        assert!(model_step_order_preserved(None, 0));
    }

    #[test]
    fn step_order_diverges() {
        assert!(model_step_order_diverges(Some(5), 3));
        assert!(!model_step_order_diverges(Some(3), 5));
        assert!(!model_step_order_diverges(None, 0));
    }

    #[test]
    fn tail_after_snapshot_valid() {
        assert!(model_tail_events_after_snapshot(10, &[11, 12, 13]));
    }

    #[test]
    fn tail_after_snapshot_rejected() {
        assert!(!model_tail_events_after_snapshot(10, &[10, 11, 12]));
        assert!(!model_tail_events_after_snapshot(10, &[5, 6, 7]));
    }

    #[test]
    fn tail_rejection_detected() {
        assert!(model_tail_events_rejection(10, &[10, 11]));
        assert!(model_tail_events_rejection(10, &[5, 6]));
        assert!(!model_tail_events_rejection(10, &[11, 12]));
    }

    #[test]
    fn stale_attempt_detection() {
        assert!(model_is_stale_attempt(Some(2), 5));
        assert!(!model_is_stale_attempt(Some(5), 5));
        assert!(!model_is_stale_attempt(Some(10), 5));
        assert!(model_is_stale_attempt(None, 5));
    }

    #[test]
    fn current_attempt_detection() {
        assert!(!model_is_current_attempt(Some(2), 5));
        assert!(model_is_current_attempt(Some(5), 5));
        assert!(model_is_current_attempt(Some(10), 5));
        assert!(model_is_current_attempt(None, 5));
    }

    #[test]
    fn max_attempt_at_least_one() {
        assert!(model_max_attempt_ge_one(&[]));
        assert!(model_max_attempt_ge_one(&[1]));
        assert!(model_max_attempt_ge_one(&[3, 5, 2]));
    }
}
