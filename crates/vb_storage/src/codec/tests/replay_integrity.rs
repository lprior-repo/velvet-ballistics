#![allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used
)]
//! Unit tests for replay integrity with RecordKind::RunKilled.
//!
//! Covers behaviors B54-B61 from the cancel/kill lattice test plan.
//! These tests verify that RunKilled events are properly validated during
//! replay, sequence numbers are contiguous, and kind 28 admission does not
//! weaken existing rejection boundaries.

use super::*;
use crate::{
    JournalEvent, constants::MAGIC_JOURNAL_EVENT, error::JournalError, records::RecordKind,
    types::EventSeq,
};
use vb_core::RunId;

use crate::codec::validation::{is_known_record_kind, validate_kind_family};

// =============================================================================
// B56: validate_replayed_event match returns Ok
// =============================================================================

#[test]
fn validate_replayed_event_match_returns_ok() {
    let run = RunId::new(10);
    let seq = EventSeq::new(5);
    let event = JournalEvent::RunKilled {
        run,
        seq,
        attempt: 1,
    };

    let result = validate_replayed_event(run, seq, &event);
    assert!(
        result.is_ok(),
        "matching run and seq for RunKilled must pass validation"
    );
}

// =============================================================================
// B57: validate_replayed_event seq mismatch returns SequenceGap
// =============================================================================

#[test]
fn validate_replayed_event_seq_mismatch_returns_gap() {
    let run = RunId::new(10);
    let event = JournalEvent::RunKilled {
        run,
        seq: EventSeq::new(5),
        attempt: 1,
    };

    let result = validate_replayed_event(run, EventSeq::new(3), &event);
    assert!(
        matches!(
            result,
            Err(JournalError::SequenceGap { expected, actual })
            if expected == EventSeq::new(3) && actual == EventSeq::new(5)
        ),
        "seq mismatch must yield SequenceGap with correct expected/actual, got {:?}",
        result
    );
}

// =============================================================================
// B58: validate_replayed_event run mismatch returns WrongRun
// =============================================================================

#[test]
fn validate_replayed_event_run_mismatch_returns_wrong_run() {
    let actual_run = RunId::new(10);
    let expected_run = RunId::new(20);
    let event = JournalEvent::RunKilled {
        run: actual_run,
        seq: EventSeq::new(0),
        attempt: 1,
    };

    let result = validate_replayed_event(expected_run, EventSeq::new(0), &event);
    assert!(
        matches!(
            result,
            Err(JournalError::WrongRun { expected, actual })
            if expected == expected_run && actual == actual_run
        ),
        "run mismatch must yield WrongRun with correct expected/actual, got {:?}",
        result
    );
}

// =============================================================================
// B55: RunKilled events preserve their EventSeq through replay validation
// =============================================================================

#[test]
fn validate_replayed_event_preserves_runkilled_seq() {
    let run = RunId::new(42);
    let seq = EventSeq::new(99);
    let event = JournalEvent::RunKilled {
        run,
        seq,
        attempt: 1,
    };

    // Verify the event correctly reports its seq
    assert_eq!(event.seq(), seq);
    // Verify seq is preserved in validation
    let result = validate_replayed_event(run, EventSeq::new(99), &event);
    assert!(
        result.is_ok(),
        "RunKilled with matching seq must pass replay"
    );
}

#[test]
fn validate_replayed_event_with_runkilled_at_boundary_seq_zero() {
    let run = RunId::new(1);
    let event = JournalEvent::RunKilled {
        run,
        seq: EventSeq::new(0),
        attempt: 1,
    };
    let result = validate_replayed_event(run, EventSeq::new(0), &event);
    assert!(result.is_ok(), "seq 0 must be valid for RunKilled replay");
}

#[test]
fn validate_replayed_event_with_runkilled_at_boundary_run_max() {
    let run = RunId::new(u64::MAX);
    let event = JournalEvent::RunKilled {
        run,
        seq: EventSeq::new(5),
        attempt: 1,
    };
    let result = validate_replayed_event(run, EventSeq::new(5), &event);
    assert!(
        result.is_ok(),
        "max RunId must be valid for RunKilled replay"
    );
}

// =============================================================================
// B61: next_seq overflow returns SequenceOverflow
// =============================================================================

#[test]
fn next_seq_max_returns_overflow() {
    let seq = EventSeq::new(u64::MAX);
    let result = next_seq(seq);
    assert!(
        matches!(result, Err(JournalError::SequenceOverflow)),
        "next_seq(u64::MAX) must return SequenceOverflow, got {:?}",
        result
    );
}

#[test]
fn next_seq_zero_returns_one() {
    let seq = EventSeq::new(0);
    let result = next_seq(seq);
    assert!(result.is_ok(), "next_seq(0) must succeed");
    assert_eq!(result.unwrap(), EventSeq::new(1));
}

#[test]
fn next_seq_normal_increments() {
    let seq = EventSeq::new(42);
    let result = next_seq(seq);
    assert!(result.is_ok(), "next_seq(42) must succeed");
    assert_eq!(result.unwrap(), EventSeq::new(43));
}

#[test]
fn next_seq_max_minus_one_returns_max() {
    let seq = EventSeq::new(u64::MAX - 1);
    let result = next_seq(seq);
    assert!(result.is_ok(), "next_seq(u64::MAX-1) must succeed");
    assert_eq!(result.unwrap(), EventSeq::new(u64::MAX));
}

// =============================================================================
// B60: journal-specific kind admission does not open unknown kind 31
// =============================================================================

/// Confirms that WaitResolved (kind 31) is admitted for the journal magic
/// while genuinely unknown kinds (e.g. 32, 9) remain rejected.
#[test]
fn kind_28_29_31_admission_for_journal_magic() {
    assert!(is_known_record_kind(28), "kind 28 must be known");
    assert!(
        validate_kind_family(MAGIC_JOURNAL_EVENT, 28).is_ok(),
        "kind 28 must be admitted for journal magic"
    );
    assert!(is_known_record_kind(29), "kind 29 must be known");
    assert!(
        validate_kind_family(MAGIC_JOURNAL_EVENT, 29).is_ok(),
        "kind 29 must be admitted for journal magic"
    );
    // WaitResolved (31) is the dedicated journal kind for bug-hunt RE-009.
    assert!(
        is_known_record_kind(31),
        "kind 31 (WaitResolved) must be known"
    );
    assert!(
        validate_kind_family(MAGIC_JOURNAL_EVENT, 31).is_ok(),
        "kind 31 (WaitResolved) must be admitted for journal magic"
    );
    // Kinds outside the journal range remain rejected.
    assert!(!is_known_record_kind(32), "kind 32 must remain unknown");
    assert!(
        matches!(
            validate_kind_family(MAGIC_JOURNAL_EVENT, 32),
            Err(JournalError::RecordKindFamilyMismatch { .. })
        ),
        "kind 32 must be rejected for journal magic"
    );
}

// =============================================================================
// RunKilled event validation: is_valid() checks
// =============================================================================

#[test]
fn runkilled_with_valid_fields_is_valid() {
    let event = JournalEvent::RunKilled {
        run: RunId::new(42),
        seq: EventSeq::new(0),
        attempt: 1,
    };
    assert!(event.is_valid());
}

#[test]
fn runkilled_with_zero_run_is_invalid() {
    let event = JournalEvent::RunKilled {
        run: RunId::new(0),
        seq: EventSeq::new(0),
        attempt: 1,
    };
    assert!(!event.is_valid(), "RunKilled with RunId(0) must be invalid");
}

#[test]
fn runkilled_with_zero_attempt_is_invalid() {
    let event = JournalEvent::RunKilled {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        attempt: 0,
    };
    assert!(
        !event.is_valid(),
        "RunKilled with attempt=0 must be invalid"
    );
}

#[test]
fn runkilled_with_overflow_seq_is_invalid() {
    let event = JournalEvent::RunKilled {
        run: RunId::new(1),
        seq: EventSeq::new(u64::MAX),
        attempt: 1,
    };
    assert!(!event.is_valid(), "RunKilled with seq=MAX must be invalid");
}

// =============================================================================
// RunKilled record_kind() and field accessors
// =============================================================================

#[test]
fn runkilled_record_kind_returns_run_killed() {
    let event = JournalEvent::RunKilled {
        run: RunId::new(5),
        seq: EventSeq::new(3),
        attempt: 2,
    };
    assert_eq!(event.record_kind(), RecordKind::RunKilled);
}

#[test]
fn runkilled_run_id_returns_correct_value() {
    let event = JournalEvent::RunKilled {
        run: RunId::new(77),
        seq: EventSeq::new(1),
        attempt: 1,
    };
    assert_eq!(event.run_id(), RunId::new(77));
}

#[test]
fn runkilled_attempt_returns_some_attempt() {
    let event = JournalEvent::RunKilled {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        attempt: 7,
    };
    assert_eq!(event.attempt(), Some(7));
}

// =============================================================================
// Cross-kind validation: RunKilled is distinct from RunCancelled
// =============================================================================

#[test]
fn runkilled_distinct_from_runcancelled_in_replay_validation() {
    let run = RunId::new(50);
    let seq = EventSeq::new(2);

    let killed = JournalEvent::RunKilled {
        run,
        seq,
        attempt: 1,
    };
    let cancelled = JournalEvent::RunCancelled {
        run,
        seq,
        attempt: 1,
        reason: None,
    };

    assert_ne!(
        killed.record_kind(),
        cancelled.record_kind(),
        "RunKilled and RunCancelled must have distinct RecordKind values"
    );

    // Both should pass replay validation with matching run+seq
    assert!(validate_replayed_event(run, seq, &killed).is_ok());
    assert!(validate_replayed_event(run, seq, &cancelled).is_ok());
}
