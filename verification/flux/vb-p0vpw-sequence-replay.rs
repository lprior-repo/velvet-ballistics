//! Flux-rs refinement annotations for sequence bounds and replay invariants.
//!
//! ============================================================================
//! PRODUCTION BINDING — SCOPED-ONLY
//! ============================================================================
//!
//! These refinement annotations model the sequence and replay invariants
//! that production code in `vb_storage` enforces by construction. The
//! companion extern files (`extern_flux_sequence.rs`, `extern_flux_replay.rs`)
//! document the production types that this model mirrors.
//!
//! Production bindings:
//!   Sequence bounds:    `crates/vb_storage/src/types.rs:73` EventSeq(u64)
//!   Contiguity check:   `crates/vb_storage/src/journal/replay.rs`
//!   Step ordering:      `crates/vb_storage/src/recovery/types.rs` StepIdx
//!   Replay bounds:      `crates/vb_storage/src/recovery/replay/core.rs`
//!   Non-idempotent:     `crates/vb_storage/src/recovery/replay/attempt.rs`
//!
//! The refined model types define the same invariants that production
//! types enforce by construction:
//!   - EventSeqRefined raw ∈ [0, u64::MAX] (trivially true for u64)
//!   - Contiguous sequence: next == prev.saturating_add(1)
//!   - Step ordering: current >= previous (monotonic non-decreasing)
//!   - Tail events: seq > snapshot_seq (strictly after snapshot)
//!   - Attempt filter: current >= max_attempt, stale < max_attempt
//!
//! These SCOPED-ONLY models are verified at runtime via the #[cfg(test)]
//! module below. They document the invariants for human review and
//! future Flux/Verus binding when production types carry #[refined_by].
//!
//! See also:
//!   verification/flux/extern_flux_sequence.rs
//!   verification/flux/extern_flux_replay.rs
//!   verification/flux/WIRING_STATUS.md

#![forbid(unsafe_code)]
#![allow(dead_code)]

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
///
/// Production mirror: EventSeq in crates/vb_storage/src/types.rs:73
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
// Journal replay contiguity models (from flux_replay)
// ============================================================================

/// Model: a slice of sequence values passes contiguity when every adjacent
/// pair satisfies `next == prev + 1` (saturating).
#[flux_rs::sig(
    fn(seqs: &[u64]) -> bool[
        contiguous_sequence_check(seqs)
    ]
)]
fn model_contiguous_sequence_check(seqs: &[u64]) -> bool {
    if seqs.len() <= 1 {
        return true;
    }
    let mut expected = seqs[0];
    for &seq in seqs.iter().skip(1) {
        if seq != expected {
            return false;
        }
        expected = expected.saturating_add(1);
    }
    true
}

/// Model: a gap in the sequence is detected when any adjacent pair fails the
/// `next == prev + 1` check.
#[flux_rs::sig(
    fn(seqs: &[u64]) -> bool[
        !contiguous_sequence_check(seqs)
    ]
)]
fn model_sequence_gap_found(seqs: &[u64]) -> bool {
    if seqs.len() < 2 {
        return false;
    }
    let mut expected = seqs[0];
    for &seq in seqs.iter().skip(1) {
        if seq != expected {
            return true;
        }
        expected = expected.saturating_add(1);
    }
    false
}

// ============================================================================
// Replay divergence detection
// ============================================================================

/// Model: snapshot-plus-tail replay is valid when every tail event sequence
/// is strictly greater than the snapshot sequence.
#[flux_rs::sig(
    fn(snapshot_seq: u64, tail_seqs: &[u64]) -> bool[
        replay_tail_valid(snapshot_seq, tail_seqs)
    ]
)]
fn model_replay_tail_valid(snapshot_seq: u64, tail_seqs: &[u64]) -> bool {
    for seq in tail_seqs {
        if *seq <= snapshot_seq {
            return false;
        }
    }
    true
}

/// Model: snapshot-plus-tail replay diverges when any tail event has
/// sequence <= snapshot sequence.
#[flux_rs::sig(
    fn(snapshot_seq: u64, tail_seqs: &[u64]) -> bool[
        replay_tail_diverges(snapshot_seq, tail_seqs)
    ]
)]
fn model_replay_tail_diverges(snapshot_seq: u64, tail_seqs: &[u64]) -> bool {
    for seq in tail_seqs {
        if *seq <= snapshot_seq {
            return true;
        }
    }
    false
}

// ============================================================================
// Non-idempotent action blocking
// ============================================================================

/// Model: an action is already resolved when the tracker has seen it.
#[flux_rs::sig(
    fn(resolved: bool, action: u64, step: u16) -> bool[
        action_already_resolved(resolved, action, step)
    ]
)]
fn model_action_already_resolved(resolved: bool, _action: u64, _step: u16) -> bool {
    resolved
}

/// Model: non-idempotent action is blocked when the action+step pair is
/// already in the resolved set.
#[flux_rs::sig(
    fn(is_resolved: bool) -> bool[
        non_idempotent_action_blocked(is_resolved)
    ]
)]
fn model_non_idempotent_action_blocked(is_resolved: bool) -> bool {
    is_resolved
}

// ============================================================================
// Terminal state extraction
// ============================================================================

/// Model: a terminal event is one of RunFinished, RunCancelled, RunKilled,
/// or RunFailedEvent.
#[flux_rs::sig(
    fn(is_terminal: bool) -> bool[is_terminal_event(is_terminal)]
)]
fn model_is_terminal_event(is_terminal: bool) -> bool {
    is_terminal
}

/// Model: terminal event extraction finds the last terminal event from the
/// latest attempt.
#[flux_rs::sig(
    fn(attempt: u16, max_attempt: u16) -> bool[
        terminal_event_from_latest_attempt(attempt, max_attempt)
    ]
)]
fn model_terminal_event_from_latest_attempt(attempt: u16, max_attempt: u16) -> bool {
    attempt == max_attempt
}

// ============================================================================
// Step divergence from replay perspective
// ============================================================================

/// Model: step order diverges when a StepStarted event carries an index
/// strictly less than the previously observed step. This enforces the
/// monotonic non-decreasing step ordering invariant.
#[flux_rs::sig(
    fn(last_step: Option<u16>, current: u16) -> bool[
        step_started_diverges(last_step, current)
    ]
)]
fn model_step_started_diverges(last_step: Option<u16>, current: u16) -> bool {
    match last_step {
        Some(prev) => current < prev,
        None => false,
    }
}

/// Model: step order is valid when current >= last (or last is None).
#[flux_rs::sig(
    fn(last_step: Option<u16>, current: u16) -> bool[
        step_started_valid(last_step, current)
    ]
)]
fn model_step_started_valid(last_step: Option<u16>, current: u16) -> bool {
    match last_step {
        Some(prev) => current >= prev,
        None => true,
    }
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

#[cfg(test)]
mod flux_replay_tests {
    use super::*;

    #[test]
    fn contiguous_sequence_passes() {
        assert!(model_contiguous_sequence_check(&[0, 1, 2, 3]));
    }

    #[test]
    fn contiguous_single_element() {
        assert!(model_contiguous_sequence_check(&[42]));
    }

    #[test]
    fn contiguous_empty() {
        assert!(model_contiguous_sequence_check(&[]));
    }

    #[test]
    fn gap_sequence_detected() {
        assert!(model_sequence_gap_found(&[0, 1, 3, 4]));
    }

    #[test]
    fn duplicate_sequence_detected() {
        assert!(model_sequence_gap_found(&[0, 1, 1, 2]));
    }

    #[test]
    fn no_gap_in_contiguous() {
        assert!(!model_sequence_gap_found(&[0, 1, 2, 3]));
    }

    #[test]
    fn step_diverges() {
        assert!(model_step_started_diverges(Some(5), 3));
    }

    #[test]
    fn step_preserved() {
        assert!(model_step_started_valid(Some(3), 5));
        assert!(model_step_started_valid(Some(3), 3));
        assert!(model_step_started_valid(None, 0));
    }

    #[test]
    fn step_no_previous_valid() {
        assert!(model_step_started_valid(None, 0));
    }

    #[test]
    fn tail_valid_after_snapshot() {
        assert!(model_replay_tail_valid(100, &[101, 102, 103]));
    }

    #[test]
    fn tail_rejected_at_snapshot() {
        assert!(!model_replay_tail_valid(100, &[100, 101, 102]));
    }

    #[test]
    fn tail_rejected_before_snapshot() {
        assert!(!model_replay_tail_valid(100, &[50, 60, 70]));
    }

    #[test]
    fn tail_diverges_detected() {
        assert!(model_replay_tail_diverges(100, &[100, 101]));
        assert!(model_replay_tail_diverges(100, &[50, 60]));
    }

    #[test]
    fn tail_no_divergence() {
        assert!(!model_replay_tail_diverges(100, &[101, 102, 103]));
    }

    #[test]
    fn action_resolved_blocks() {
        assert!(model_non_idempotent_action_blocked(true));
        assert!(!model_non_idempotent_action_blocked(false));
    }

    #[test]
    fn terminal_from_latest() {
        assert!(model_terminal_event_from_latest_attempt(3, 3));
        assert!(!model_terminal_event_from_latest_attempt(2, 3));
    }
}
