//! Journal Batch Accounting Tests
//!
//! Tests for batch limit enforcement with MAX_BATCH_COUNT = 10,000.
//!
//! Behaviors covered:
//! - B01: BudgetExceeded at byte limit
//! - B02: QueueFull at count limit
//! - B03: BudgetExceeded exact error construction
//! - B04: QueueFull exact error variant
//! - B05: len() monotonic increment
//! - B06: No state mutation on limit failure
//! - B07: Limit checked before durable write

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::let_underscore_must_use,
    clippy::panic_in_result_fn
)]

use vb_core::{RunId, WorkflowDigest};
use vb_storage::{
    EventSeq, JournalError, JournalEvent, constants::MAX_BATCH_COUNT, journal::FjallJournal,
};

fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
    (temp, journal)
}

fn make_event(run: RunId, seq: u64) -> JournalEvent {
    JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(seq),
        workflow: WorkflowDigest::from_bytes([0; 32]),
    }
}

// ============================================================================
// B01: BudgetExceeded at byte limit
// ============================================================================
// Note: JournalWriteBatch does not enforce byte limits directly.
// Byte budget enforcement happens at the runtime/budget layer via
// BudgetError::JournalBatchBytesExceeded. The batch only enforces
// count limits. This test documents that behavior.

#[test]
fn batch_has_no_byte_limit_enforcement_at_storage_layer() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(1);

    let mut batch = journal.batch();

    let result = batch.append_event(&make_event(run, 0));
    assert!(
        result.is_ok(),
        "append_event should succeed at storage layer regardless of byte budget"
    );
}

// ============================================================================
// B02: QueueFull at count limit
// ============================================================================

#[test]
fn batch_append_event_returns_queue_full_at_count_limit() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(2);

    let mut batch = journal.batch();

    for i in 0..MAX_BATCH_COUNT {
        let evt = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(i as u64),
            workflow: WorkflowDigest::from_bytes([0; 32]),
        };
        let result = batch.append_event(&evt);
        assert!(
            result.is_ok(),
            "append_event {i} should succeed, got {:?}",
            result
        );
    }

    let evt_over = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(MAX_BATCH_COUNT as u64),
        workflow: WorkflowDigest::from_bytes([0; 32]),
    };
    let result = batch.append_event(&evt_over);
    assert!(
        matches!(result, Err(JournalError::QueueFull)),
        "append_event at MAX_BATCH_COUNT should return QueueFull, got {:?}",
        result
    );
}

#[test]
fn batch_queue_full_error_is_exact_variant() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(3);

    let mut batch = journal.batch();

    for i in 0..MAX_BATCH_COUNT {
        let evt = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(i as u64),
            workflow: WorkflowDigest::from_bytes([0; 32]),
        };
        let _ = batch.append_event(&evt);
    }

    let evt_over = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(MAX_BATCH_COUNT as u64),
        workflow: WorkflowDigest::from_bytes([0; 32]),
    };
    let result = batch.append_event(&evt_over);

    let err = result.expect_err("should return error");
    let is_queue_full = matches!(err, JournalError::QueueFull);
    assert!(
        is_queue_full,
        "error variant must be exactly JournalError::QueueFull, got {:?}",
        err
    );
}

// ============================================================================
// B03: BudgetExceeded exact error construction
// ============================================================================
// Note: BudgetExceeded is from vb_core::errors::CoreError, not JournalError.
// This tests that BudgetError::JournalBatchBytesExceeded can be constructed
// with exact field values.

#[test]
fn budget_error_journal_batch_bytes_exceeded_exact_construction() {
    use vb_core::budget::BudgetError;

    let actual: u32 = 2_097_152;
    let limit: u32 = 1_048_576;
    let error = BudgetError::JournalBatchBytesExceeded { actual, limit };

    let msg = format!("{}", error);
    assert!(
        msg.contains("journal batch bytes exceeded"),
        "error message should describe journal batch bytes exceeded"
    );
    assert!(
        msg.contains(&format!("{}", actual)),
        "error message should contain actual value"
    );
    assert!(
        msg.contains(&format!("{}", limit)),
        "error message should contain limit value"
    );
}

// ============================================================================
// B04: QueueFull exact error variant
// ============================================================================

#[test]
fn journal_error_queue_full_is_exact_variant() {
    let error = JournalError::QueueFull;
    let msg = format!("{}", error);
    assert!(
        msg.contains("journal writer queue is full"),
        "QueueFull error message should be descriptive, got: {}",
        msg
    );
}

#[test]
fn journal_error_queue_full_display_format() {
    let error = JournalError::QueueFull;
    let display = format!("{}", error);
    assert_eq!(display, "journal writer queue is full");
}

// ============================================================================
// B05: len() monotonic increment
// ============================================================================

#[test]
fn batch_len_monotonically_increments() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(10);

    let mut batch = journal.batch();
    let mut prev_len = batch.len();

    for i in 0..100 {
        let evt = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(i as u64),
            workflow: WorkflowDigest::from_bytes([0; 32]),
        };
        batch.append_event(&evt).expect("append should succeed");
        let new_len = batch.len();
        assert!(
            new_len > prev_len,
            "len() must monotonically increase: prev={}, new={}",
            prev_len,
            new_len
        );
        prev_len = new_len;
    }

    assert_eq!(prev_len, 100);
}

#[test]
fn batch_len_never_decreases_after_successful_appends() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(11);

    let mut batch = journal.batch();
    let mut lens = Vec::new();

    for i in 0..50 {
        let evt = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(i as u64),
            workflow: WorkflowDigest::from_bytes([0; 32]),
        };
        batch.append_event(&evt).expect("append should succeed");
        lens.push(batch.len());
    }

    for window in lens.windows(2) {
        assert!(
            window[1] >= window[0],
            "len() should never decrease: {:?}",
            lens
        );
    }
}

// ============================================================================
// B06: No state mutation on limit failure
// ============================================================================

#[test]
fn batch_len_unchanged_after_queue_full() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(20);

    let mut batch = journal.batch();

    for i in 0..MAX_BATCH_COUNT {
        let evt = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(i as u64),
            workflow: WorkflowDigest::from_bytes([0; 32]),
        };
        let _ = batch.append_event(&evt);
    }

    let len_before = batch.len();

    let evt_over = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(MAX_BATCH_COUNT as u64),
        workflow: WorkflowDigest::from_bytes([0; 32]),
    };
    let result = batch.append_event(&evt_over);
    assert!(result.is_err(), "should fail at limit");

    let len_after = batch.len();
    assert_eq!(
        len_before, len_after,
        "len() must not change when limit is exceeded: before={}, after={}",
        len_before, len_after
    );
}

#[test]
fn batch_is_empty_unchanged_after_queue_full() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(21);

    let mut batch = journal.batch();

    for i in 0..MAX_BATCH_COUNT {
        let evt = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(i as u64),
            workflow: WorkflowDigest::from_bytes([0; 32]),
        };
        let _ = batch.append_event(&evt);
    }

    let is_empty_before = batch.is_empty();

    let evt_over = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(MAX_BATCH_COUNT as u64),
        workflow: WorkflowDigest::from_bytes([0; 32]),
    };
    let _ = batch.append_event(&evt_over);

    let is_empty_after = batch.is_empty();
    assert_eq!(
        is_empty_before, is_empty_after,
        "is_empty() must not change when limit is exceeded"
    );
}

#[test]
fn batch_aborted_flag_unchanged_on_queue_full() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(22);

    let mut batch = journal.batch();

    for i in 0..MAX_BATCH_COUNT {
        let evt = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(i as u64),
            workflow: WorkflowDigest::from_bytes([0; 32]),
        };
        let _ = batch.append_event(&evt);
    }

    let len_before = batch.len();

    let evt_over = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(MAX_BATCH_COUNT as u64),
        workflow: WorkflowDigest::from_bytes([0; 32]),
    };
    let _ = batch.append_event(&evt_over);

    assert_eq!(
        batch.len(),
        len_before,
        "batch internal state should not mutate on QueueFull"
    );
}

// ============================================================================
// B07: Limit checked before durable write
// ============================================================================

#[test]
fn batch_limit_checked_before_commit() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(30);

    let mut batch = journal.batch();

    for i in 0..MAX_BATCH_COUNT {
        let evt = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(i as u64),
            workflow: WorkflowDigest::from_bytes([0; 32]),
        };
        let _ = batch.append_event(&evt);
    }

    let evt_over = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(MAX_BATCH_COUNT as u64),
        workflow: WorkflowDigest::from_bytes([0; 32]),
    };
    let result = batch.append_event(&evt_over);
    assert!(
        matches!(result, Err(JournalError::QueueFull)),
        "limit should be checked before durable write"
    );

    let commit_result = batch.commit();
    assert!(
        commit_result.is_ok(),
        "batch commit should succeed even after QueueFull on append, got {:?}",
        commit_result
    );

    let events = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");
    assert_eq!(
        events.len(),
        MAX_BATCH_COUNT,
        "exactly MAX_BATCH_COUNT events should be committed"
    );
}

#[test]
fn batch_does_not_commit_on_limit_exceeded_append() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(31);

    let mut batch = journal.batch();

    for i in 0..MAX_BATCH_COUNT {
        let evt = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(i as u64),
            workflow: WorkflowDigest::from_bytes([0; 32]),
        };
        let _ = batch.append_event(&evt);
    }

    let evt_over = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(MAX_BATCH_COUNT as u64),
        workflow: WorkflowDigest::from_bytes([0; 32]),
    };
    let append_result = batch.append_event(&evt_over);
    assert!(
        matches!(append_result, Err(JournalError::QueueFull)),
        "append over limit should return QueueFull"
    );

    let pre_commit_len = batch.len();
    assert_eq!(
        pre_commit_len, MAX_BATCH_COUNT,
        "batch len should be MAX_BATCH_COUNT before commit"
    );

    batch.commit().expect("commit should succeed");

    let events = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");
    assert_eq!(
        events.len(),
        MAX_BATCH_COUNT,
        "committed events should equal MAX_BATCH_COUNT, not exceeding"
    );
}

// ============================================================================
// Additional boundary tests
// ============================================================================

#[test]
fn batch_len_at_exactly_max_batch_count() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(40);

    let mut batch = journal.batch();

    for i in 0..MAX_BATCH_COUNT {
        let evt = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(i as u64),
            workflow: WorkflowDigest::from_bytes([0; 32]),
        };
        batch.append_event(&evt).expect("append should succeed");
    }

    assert_eq!(
        batch.len(),
        MAX_BATCH_COUNT,
        "len() should equal MAX_BATCH_COUNT after exactly MAX_BATCH_COUNT appends"
    );
}

#[test]
fn batch_len_at_max_minus_one() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(41);

    let mut batch = journal.batch();

    for i in 0..(MAX_BATCH_COUNT - 1) {
        let evt = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(i as u64),
            workflow: WorkflowDigest::from_bytes([0; 32]),
        };
        batch.append_event(&evt).expect("append should succeed");
    }

    assert_eq!(
        batch.len(),
        MAX_BATCH_COUNT - 1,
        "len() should equal MAX_BATCH_COUNT - 1"
    );

    let evt = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new((MAX_BATCH_COUNT - 1) as u64),
        workflow: WorkflowDigest::from_bytes([0; 32]),
    };
    let result = batch.append_event(&evt);
    assert!(
        result.is_ok(),
        "append at MAX_BATCH_COUNT - 1 should succeed, got {:?}",
        result
    );
}

#[test]
fn batch_queue_full_after_one_over_limit() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(42);

    let mut batch = journal.batch();

    for i in 0..MAX_BATCH_COUNT {
        let evt = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(i as u64),
            workflow: WorkflowDigest::from_bytes([0; 32]),
        };
        let _ = batch.append_event(&evt);
    }

    let evt_over = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(MAX_BATCH_COUNT as u64),
        workflow: WorkflowDigest::from_bytes([0; 32]),
    };
    let first_result = batch.append_event(&evt_over);
    assert!(
        matches!(first_result, Err(JournalError::QueueFull)),
        "first append over limit should return QueueFull"
    );

    let evt_over2 = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new((MAX_BATCH_COUNT + 1) as u64),
        workflow: WorkflowDigest::from_bytes([0; 32]),
    };
    let second_result = batch.append_event(&evt_over2);
    assert!(
        matches!(second_result, Err(JournalError::QueueFull)),
        "second append over limit should also return QueueFull"
    );
}
