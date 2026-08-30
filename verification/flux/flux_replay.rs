// Flux-rs standalone demo refinements for replay bounds in `vb_storage`.
//
// NON-CLOSURE EVIDENCE — not bound to production code via `#[path]`.
// This file is a standalone Flux demo that exercises refinement annotations
// against hand-written shadow models. It provides model sketches for audit
// and research only; it cannot be cited as production safety evidence.
//
// Domain models: sequence contiguity during journal replay, step ordering
// invariant, tail event bounds relative to snapshot sequence, and attempt
// filtering during replay.
//
// Obligation: Demos replay-bounds and contiguity refinement models.
// Verifier: flux-rs
// Category: SCOPED-ONLY (non-closure evidence)

#![forbid(unsafe_code)]

extern crate flux_rs;
use flux_rs::attrs::*;

// ============================================================================
// Contiguous sequence validation for journal events
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
// Step ordering invariant during replay
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
// Replay divergence detection — full journal and snapshot-plus-tail
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
// Tests — runtime verification of model correctness
// ============================================================================

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
