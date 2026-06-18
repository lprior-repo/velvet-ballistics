//! Kani harnesses for vb_storage recovery hydration functions.
//!
//! Targets the allocation-free production seams:
//! - `hydrate_events_preconditions`
//! - `hydrate_dimensions_positive`
//! - `hydrate_snapshot_tail_has_evidence`
//! - `hydrate_snapshot_tail_run_matches`
//! - `hydrate_snapshot_tail_seq_after_snapshot`
//! - `hydrate_snapshot_tail_preconditions`

#![forbid(unsafe_code)]

use vb_core::{RunId, StepIdx, WorkflowDigest};

// Note: EventSeq from vb_storage (crate::EventSeq), not vb_core
use crate::EventSeq;

use crate::JournalEvent;
use crate::recovery::RunSnapshot;
use crate::recovery::hydrate::{
    hydrate_dimensions_positive, hydrate_events_preconditions, hydrate_snapshot_tail_has_evidence,
    hydrate_snapshot_tail_preconditions, hydrate_snapshot_tail_run_matches,
    hydrate_snapshot_tail_seq_after_snapshot,
};

// ============================================================================
// RunSnapshot helpers
// ============================================================================

/// Creates a RunSnapshot with empty slots/taint for precondition testing.
fn empty_snapshot(run: RunId, seq: EventSeq) -> RunSnapshot {
    RunSnapshot {
        run,
        seq,
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
        slots: Vec::new(),
        taint: Vec::new(),
    }
}

/// Creates a RunSnapshot with non-empty slots for evidence testing.
fn snapshot_with_slots(run: RunId, seq: EventSeq) -> RunSnapshot {
    RunSnapshot {
        run,
        seq,
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
        slots: vec![0u8], // non-empty to satisfy has_evidence
        taint: vec![0u8],
    }
}

// ============================================================================
// Event construction helpers
// ============================================================================

/// Creates a StepStarted event at given run/seq.
fn step_started(run: RunId, seq: EventSeq, step: StepIdx, attempt: u16) -> JournalEvent {
    JournalEvent::StepStarted {
        run,
        seq,
        step,
        attempt,
    }
}

// ============================================================================
// Kani proofs for hydration preconditions
// ============================================================================

/// PO-VB-STORAGE-001: hydrate_events_preconditions returns true for non-empty events.
#[kani::proof]
fn kani_events_preconditions_non_empty() {
    let run = RunId::new(42);
    let step_idx = StepIdx::new(0);

    let event1 = JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(0),
        step: step_idx,
        attempt: 1,
    };
    let event2 = JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(1),
        step: step_idx,
        attempt: 1,
    };

    // Use slice to avoid complex Vec drop paths
    let events = &[event1, event2];

    let result = hydrate_events_preconditions(events);
    kani::assert(result, "non-empty events must satisfy preconditions");
}

/// PO-VB-STORAGE-002: hydrate_events_preconditions returns false for empty events.
#[kani::proof]
fn kani_events_preconditions_empty() {
    let events: &[JournalEvent] = &[];

    let result = hydrate_events_preconditions(events);
    kani::assert(!result, "empty events must not satisfy preconditions");
}

/// PO-VB-STORAGE-003: hydrate_dimensions_positive accepts positive dimensions.
#[kani::proof]
fn kani_dimensions_positive_accepts_positive() {
    // Use small bounded values
    let step_count = u16::from(kani::any::<u8>().saturating_add(1)); // 1..256
    let slot_count = u16::from(kani::any::<u8>().saturating_add(1)); // 1..256

    let result = hydrate_dimensions_positive(step_count, slot_count);
    kani::assert(result, "positive dimensions must be accepted");
}

/// PO-VB-STORAGE-004: hydrate_dimensions_positive rejects zero step_count.
#[kani::proof]
fn kani_dimensions_positive_rejects_zero_step() {
    let step_count = 0u16;
    let slot_count = u16::from(kani::any::<u8>().saturating_add(1));

    let result = hydrate_dimensions_positive(step_count, slot_count);
    kani::assert(!result, "zero step_count must be rejected");
}

/// PO-VB-STORAGE-005: hydrate_dimensions_positive rejects zero slot_count.
#[kani::proof]
fn kani_dimensions_positive_rejects_zero_slot() {
    let step_count = u16::from(kani::any::<u8>().saturating_add(1));
    let slot_count = 0u16;

    let result = hydrate_dimensions_positive(step_count, slot_count);
    kani::assert(!result, "zero slot_count must be rejected");
}

/// PO-VB-STORAGE-006: hydrate_dimensions_positive rejects both zero.
#[kani::proof]
fn kani_dimensions_positive_rejects_both_zero() {
    let step_count = 0u16;
    let slot_count = 0u16;

    let result = hydrate_dimensions_positive(step_count, slot_count);
    kani::assert(!result, "both zero must be rejected");
}

/// PO-VB-STORAGE-007: hydrate_snapshot_tail_has_evidence true when tail non-empty.
#[kani::proof]
fn kani_has_evidence_tail_non_empty() {
    let run = RunId::new(42);
    let step_idx = StepIdx::new(0);

    let event1 = step_started(run, EventSeq::new(1), step_idx, 1);
    let event2 = step_started(run, EventSeq::new(2), step_idx, 1);

    let snapshot = empty_snapshot(run, EventSeq::new(0));
    let events = &[event1, event2];

    let result = hydrate_snapshot_tail_has_evidence(&snapshot, events);
    kani::assert(result, "non-empty tail satisfies has_evidence");
}

/// PO-VB-STORAGE-008: hydrate_snapshot_tail_has_evidence true when slots non-empty.
#[kani::proof]
fn kani_has_evidence_slots_non_empty() {
    let run = RunId::new(42);
    let snapshot = snapshot_with_slots(run, EventSeq::new(0));
    let events: &[JournalEvent] = &[];

    let result = hydrate_snapshot_tail_has_evidence(&snapshot, events);
    kani::assert(result, "non-empty slots satisfies has_evidence");
}

/// PO-VB-STORAGE-009: hydrate_snapshot_tail_has_evidence false when all empty.
#[kani::proof]
fn kani_has_evidence_all_empty() {
    let run = RunId::new(42);
    let snapshot = empty_snapshot(run, EventSeq::new(0));
    let events: &[JournalEvent] = &[];

    let result = hydrate_snapshot_tail_has_evidence(&snapshot, events);
    kani::assert(!result, "all empty must not satisfy has_evidence");
}

/// PO-VB-STORAGE-010: hydrate_snapshot_tail_run_matches true when ids match.
#[kani::proof]
fn kani_run_matches_true() {
    let run = RunId::new(42);
    let step_idx = StepIdx::new(0);

    let event1 = step_started(run, EventSeq::new(1), step_idx, 1);
    let event2 = step_started(run, EventSeq::new(2), step_idx, 1);

    let snapshot = empty_snapshot(run, EventSeq::new(5));
    let events = &[event1, event2];

    let result = hydrate_snapshot_tail_run_matches(&snapshot, events, run);
    kani::assert(result, "matching run_id must return true");
}

/// PO-VB-STORAGE-011: hydrate_snapshot_tail_run_matches false when snapshot run differs.
#[kani::proof]
fn kani_run_matches_snapshot_differs() {
    let run_a = RunId::new(42);
    let run_b = RunId::new(43);
    let step_idx = StepIdx::new(0);

    let event1 = step_started(run_a, EventSeq::new(1), step_idx, 1);
    let event2 = step_started(run_a, EventSeq::new(2), step_idx, 1);

    let snapshot = empty_snapshot(run_b, EventSeq::new(0));
    let events = &[event1, event2];

    let result = hydrate_snapshot_tail_run_matches(&snapshot, events, run_a);
    kani::assert(!result, "snapshot run mismatch must return false");
}

/// PO-VB-STORAGE-012: hydrate_snapshot_tail_run_matches false when event run differs.
#[kani::proof]
fn kani_run_matches_event_differs() {
    let run_a = RunId::new(42);
    let run_b = RunId::new(43);
    let step_idx = StepIdx::new(0);

    let event1 = step_started(run_b, EventSeq::new(1), step_idx, 1);
    let event2 = step_started(run_b, EventSeq::new(2), step_idx, 1);

    let snapshot = empty_snapshot(run_a, EventSeq::new(0));
    let events = &[event1, event2];

    let result = hydrate_snapshot_tail_run_matches(&snapshot, events, run_a);
    kani::assert(!result, "event run mismatch must return false");
}

/// PO-VB-STORAGE-013: hydrate_snapshot_tail_seq_after_snapshot true when seq after.
#[kani::proof]
fn kani_seq_after_true() {
    let run = RunId::new(42);
    let step_idx = StepIdx::new(0);
    let snapshot_seq = EventSeq::new(5);

    let event1 = step_started(run, EventSeq::new(6), step_idx, 1);
    let event2 = step_started(run, EventSeq::new(7), step_idx, 1);

    let snapshot = empty_snapshot(run, snapshot_seq);
    let events = &[event1, event2];

    let result = hydrate_snapshot_tail_seq_after_snapshot(&snapshot, events);
    kani::assert(result, "seq after snapshot must return true");
}

/// PO-VB-STORAGE-014: hydrate_snapshot_tail_seq_after_snapshot false when seq before.
#[kani::proof]
fn kani_seq_after_false_before() {
    let run = RunId::new(42);
    let step_idx = StepIdx::new(0);
    let snapshot_seq = EventSeq::new(10);

    let event1 = step_started(run, EventSeq::new(5), step_idx, 1);
    let event2 = step_started(run, EventSeq::new(7), step_idx, 1);

    let snapshot = empty_snapshot(run, snapshot_seq);
    let events = &[event1, event2];

    let result = hydrate_snapshot_tail_seq_after_snapshot(&snapshot, events);
    kani::assert(!result, "seq before snapshot must return false");
}

/// PO-VB-STORAGE-015: hydrate_snapshot_tail_seq_after_snapshot false when seq equal.
#[kani::proof]
fn kani_seq_after_false_equal() {
    let run = RunId::new(42);
    let step_idx = StepIdx::new(0);
    let snapshot_seq = EventSeq::new(5);

    let event1 = step_started(run, EventSeq::new(5), step_idx, 1);
    let event2 = step_started(run, EventSeq::new(6), step_idx, 1);

    let snapshot = empty_snapshot(run, snapshot_seq);
    let events = &[event1, event2];

    let result = hydrate_snapshot_tail_seq_after_snapshot(&snapshot, events);
    kani::assert(!result, "seq equal to snapshot must return false");
}

/// PO-VB-STORAGE-016: hydrate_snapshot_tail_preconditions true when all conditions met.
#[kani::proof]
fn kani_preconditions_all_met() {
    let run = RunId::new(42);
    let step_idx = StepIdx::new(0);
    let snapshot_seq = EventSeq::new(5);

    let event1 = step_started(run, EventSeq::new(6), step_idx, 1);
    let event2 = step_started(run, EventSeq::new(7), step_idx, 1);

    let snapshot = snapshot_with_slots(run, snapshot_seq);
    let events = &[event1, event2];

    let result = hydrate_snapshot_tail_preconditions(&snapshot, events, run);
    kani::assert(result, "all conditions met must return true");
}

/// PO-VB-STORAGE-017: hydrate_snapshot_tail_preconditions false when run mismatch.
#[kani::proof]
fn kani_preconditions_false_run_mismatch() {
    let run_a = RunId::new(42);
    let run_b = RunId::new(43);
    let step_idx = StepIdx::new(0);
    let snapshot_seq = EventSeq::new(5);

    let event1 = step_started(run_a, EventSeq::new(6), step_idx, 1);

    let snapshot = snapshot_with_slots(run_a, snapshot_seq);
    let events = &[event1];

    let result = hydrate_snapshot_tail_preconditions(&snapshot, events, run_b);
    kani::assert(!result, "run mismatch must cause preconditions false");
}
