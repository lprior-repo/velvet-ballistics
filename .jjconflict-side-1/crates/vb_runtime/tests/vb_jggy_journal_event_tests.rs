#![forbid(unsafe_code)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::expect_used,
    clippy::get_first,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]
//! Unit tests for vb-jggy: RuntimeJournalEvent attempt field.
//!
//! POST-003 requires:
//! - `RuntimeJournalEvent::StepSucceeded { run, step, attempt: u16, output }`
//! - `RuntimeJournalEvent::ActionFailed { run, step, attempt: u16, action }`
//!
//! These tests verify the attempt field is present, serializes correctly,
//! and round-trip decode produces the correct value.
//!
//! These tests are expected to FAIL until vb-jggy implementation is complete.

use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx};
use vb_runtime::journal::{RuntimeJournal, RuntimeJournalEvent, VolatileRuntimeJournal};

/// B30: StepSucceeded carries attempt field and round-trips correctly
#[test]
fn step_succeeded_carries_attempt_and_round_trips() {
    let run = RunId::new(1);
    let step = StepIdx::new(2);
    let output = SlotIdx::new(3);
    let attempt: u16 = 7;

    let event = RuntimeJournalEvent::StepSucceeded {
        run,
        step,
        output,
        attempt, // POST-003: attempt field should exist
    };

    // Verify the attempt field is accessible
    match &event {
        RuntimeJournalEvent::StepSucceeded {
            run: r,
            step: s,
            output: o,
            attempt: a,
        } => {
            assert_eq!(*r, run);
            assert_eq!(*s, step);
            assert_eq!(*o, output);
            assert_eq!(*a, 7);
        }
        other => panic!("Expected StepSucceeded, got {:?}", other),
    }

    // Round-trip through postcard serialization
    let encoded = postcard::to_allocvec(&event).expect("encoding should succeed");
    let decoded: RuntimeJournalEvent =
        postcard::from_bytes(&encoded).expect("decoding should succeed");

    match decoded {
        RuntimeJournalEvent::StepSucceeded {
            run: r,
            step: s,
            output: o,
            attempt: a,
        } => {
            assert_eq!(r, run);
            assert_eq!(s, step);
            assert_eq!(o, output);
            assert_eq!(a, 7, "attempt should round-trip correctly");
        }
        other => panic!("Expected StepSucceeded after round-trip, got {:?}", other),
    }
}

/// B31: ActionFailed carries attempt field and round-trips correctly
#[test]
fn action_failed_carries_attempt_and_round_trips() {
    let run = RunId::new(42);
    let step = StepIdx::new(5);
    let action = ActionId::new(99);
    let attempt: u16 = 13;

    let event = RuntimeJournalEvent::ActionFailed {
        run,
        step,
        action,
        attempt, // POST-003: attempt field should exist
    };

    // Verify the attempt field is accessible
    match &event {
        RuntimeJournalEvent::ActionFailed {
            run: r,
            step: s,
            action: a,
            attempt: att,
        } => {
            assert_eq!(*r, run);
            assert_eq!(*s, step);
            assert_eq!(*a, action);
            assert_eq!(*att, 13);
        }
        other => panic!("Expected ActionFailed, got {:?}", other),
    }

    // Round-trip through postcard serialization
    let encoded = postcard::to_allocvec(&event).expect("encoding should succeed");
    let decoded: RuntimeJournalEvent =
        postcard::from_bytes(&encoded).expect("decoding should succeed");

    match decoded {
        RuntimeJournalEvent::ActionFailed {
            run: r,
            step: s,
            action: a,
            attempt: att,
        } => {
            assert_eq!(r, run);
            assert_eq!(s, step);
            assert_eq!(a, action);
            assert_eq!(att, 13, "attempt should round-trip correctly");
        }
        other => panic!("Expected ActionFailed after round-trip, got {:?}", other),
    }
}

/// StepSucceeded with attempt=1 (first attempt)
#[test]
fn step_succeeded_attempt_one_round_trips() {
    let event = RuntimeJournalEvent::StepSucceeded {
        run: RunId::new(1),
        step: StepIdx::ZERO,
        output: SlotIdx::ZERO,
        attempt: 1,
    };

    let encoded = postcard::to_allocvec(&event).expect("encoding should succeed");
    let decoded: RuntimeJournalEvent =
        postcard::from_bytes(&encoded).expect("decoding should succeed");

    match decoded {
        RuntimeJournalEvent::StepSucceeded { attempt: a, .. } => {
            assert_eq!(a, 1);
        }
        other => panic!("Expected StepSucceeded, got {:?}", other),
    }
}

/// ActionFailed with max u16 attempt value
#[test]
fn action_failed_max_attempt_round_trips() {
    let event = RuntimeJournalEvent::ActionFailed {
        run: RunId::new(1),
        step: StepIdx::ZERO,
        action: ActionId::new(0),
        attempt: u16::MAX,
    };

    let encoded = postcard::to_allocvec(&event).expect("encoding should succeed");
    let decoded: RuntimeJournalEvent =
        postcard::from_bytes(&encoded).expect("decoding should succeed");

    match decoded {
        RuntimeJournalEvent::ActionFailed { attempt: a, .. } => {
            assert_eq!(a, u16::MAX);
        }
        other => panic!("Expected ActionFailed, got {:?}", other),
    }
}

/// StepSucceeded with attempt=0 should NOT happen in practice (attempt starts at 1)
/// but we verify the field exists and serializes correctly
#[test]
fn step_succeeded_attempt_zero_serializes() {
    let event = RuntimeJournalEvent::StepSucceeded {
        run: RunId::new(1),
        step: StepIdx::ZERO,
        output: SlotIdx::ZERO,
        attempt: 0,
    };

    let encoded = postcard::to_allocvec(&event).expect("encoding should succeed");
    let decoded: RuntimeJournalEvent =
        postcard::from_bytes(&encoded).expect("decoding should succeed");

    match decoded {
        RuntimeJournalEvent::StepSucceeded { attempt: a, .. } => {
            assert_eq!(a, 0);
        }
        other => panic!("Expected StepSucceeded, got {:?}", other),
    }
}

/// Journal append preserves attempt field through VolatileRuntimeJournal
#[test]
fn volatile_journal_preserves_step_succeeded_attempt() {
    let journal = VolatileRuntimeJournal::new();
    let run = RunId::new(99);
    let step = StepIdx::new(7);
    let output = SlotIdx::new(4);
    let attempt: u16 = 12;

    let event = RuntimeJournalEvent::StepSucceeded {
        run,
        step,
        output,
        attempt,
    };

    journal.append(event).expect("append should succeed");

    let events = journal.snapshot().expect("snapshot should succeed");
    assert!(!events.is_empty(), "journal should have events");

    match &events[0] {
        RuntimeJournalEvent::StepSucceeded {
            run: r,
            step: s,
            output: o,
            attempt: a,
        } => {
            assert_eq!(*r, run);
            assert_eq!(*s, step);
            assert_eq!(*o, output);
            assert_eq!(*a, attempt);
        }
        other => panic!("Expected StepSucceeded, got {:?}", other),
    }
}

/// Journal append preserves attempt field through ActionFailed
#[test]
fn volatile_journal_preserves_action_failed_attempt() {
    let journal = VolatileRuntimeJournal::new();
    let run = RunId::new(77);
    let step = StepIdx::new(3);
    let action = ActionId::new(55);
    let attempt: u16 = 9;

    let event = RuntimeJournalEvent::ActionFailed {
        run,
        step,
        action,
        attempt,
    };

    journal.append(event).expect("append should succeed");

    let events = journal.snapshot().expect("snapshot should succeed");
    assert!(!events.is_empty(), "journal should have events");

    match &events[0] {
        RuntimeJournalEvent::ActionFailed {
            run: r,
            step: s,
            action: a,
            attempt: att,
        } => {
            assert_eq!(*r, run);
            assert_eq!(*s, step);
            assert_eq!(*a, action);
            assert_eq!(*att, attempt);
        }
        other => panic!("Expected ActionFailed, got {:?}", other),
    }
}

/// Verify other RuntimeJournalEvent variants still work (regression test)
#[test]
fn other_journal_variants_still_work() {
    let journal = VolatileRuntimeJournal::new();

    // RunSubmitted variant (no attempt needed)
    let submitted = RuntimeJournalEvent::RunSubmitted {
        run: RunId::new(1),
        workflow: vb_core::ids::WorkflowDigest::from_bytes([1; 32]),
    };
    assert_eq!(journal.append(submitted), Ok(()));

    // RunFinished variant
    let finished = RuntimeJournalEvent::RunFinished {
        run: RunId::new(1),
        result: SlotIdx::ZERO,
    };
    assert_eq!(journal.append(finished), Ok(()));

    // ActionScheduled variant
    let scheduled = RuntimeJournalEvent::ActionScheduled {
        run: RunId::new(1),
        step: StepIdx::ZERO,
        action: ActionId::new(0),
    };
    assert_eq!(journal.append(scheduled), Ok(()));

    let events = journal.snapshot().expect("snapshot should succeed");
    assert_eq!(events.len(), 3);
}

/// Verify run_id() method works for StepSucceeded and ActionFailed with attempt
#[test]
fn run_id_method_works_for_variants_with_attempt() {
    let run = RunId::new(123);

    let step_succeeded = RuntimeJournalEvent::StepSucceeded {
        run,
        step: StepIdx::ZERO,
        output: SlotIdx::ZERO,
        attempt: 5,
    };
    assert_eq!(step_succeeded.run_id(), run);

    let action_failed = RuntimeJournalEvent::ActionFailed {
        run,
        step: StepIdx::ZERO,
        action: ActionId::new(0),
        attempt: 5,
    };
    assert_eq!(action_failed.run_id(), run);
}
