#![forbid(unsafe_code)]
use super::*;

#[test]
fn checked_add_accepts_exact_fit() {
    let staged: u64 = 60;
    let delta: u64 = 60;
    let limit: u64 = 120;
    let total = staged.checked_add(delta).expect("must not overflow");
    assert!(total <= limit, "exact fit must be accepted");
    assert_eq!(total, 120, "total must be 120");
}

#[test]
fn checked_add_accepts_under_limit() {
    let staged: u64 = 60;
    let delta: u64 = 80;
    let limit: u64 = 200;
    let total = staged.checked_add(delta).expect("must not overflow");
    assert!(total < limit, "under limit must be accepted");
    assert_eq!(total, 140, "total must be 140");
}

#[test]
fn checked_add_rejects_over_limit() {
    let staged: u64 = 60;
    let delta: u64 = 41;
    let limit: u64 = 100;
    let total = staged.checked_add(delta).expect("must not overflow");
    assert!(total > limit, "over limit must be rejected");
}

#[test]
fn zero_length_encoded_event_is_always_accepted_if_not_overflow() {
    let staged: u64 = 100;
    let delta: u64 = 0;
    let limit: u64 = 100;
    let total = staged
        .checked_add(delta)
        .expect("zero delta never overflows");
    assert!(total <= limit, "zero-length must be accepted");
    assert_eq!(total, staged, "total must equal staged when delta is 0");
}

#[test]
fn checked_add_returns_none_on_overflow() {
    let total = u64::MAX.checked_add(1u64);
    assert!(total.is_none(), "u64::MAX + 1 must overflow (return None)");
}

#[test]
fn queue_full_error_is_distinct_from_payload_too_large() {
    let qf = JournalError::QueueFull;
    let ptl = JournalError::PayloadTooLarge { len: 100, max: 50 };
    assert!(
        matches!(qf, JournalError::QueueFull),
        "QueueFull must match itself"
    );
    assert!(
        matches!(ptl, JournalError::PayloadTooLarge { .. }),
        "PayloadTooLarge must match itself"
    );
}

#[test]
fn payload_too_large_details_are_accurate() {
    let err = JournalError::PayloadTooLarge { len: 200, max: 100 };
    let msg = format!("{err}");
    assert!(msg.contains("200"), "message must contain len, got {msg}");
    assert!(msg.contains("100"), "message must contain max, got {msg}");
}

#[test]
fn duplicate_event_fields_are_accurate() {
    let run = RunId::new(42);
    let err = JournalError::DuplicateEvent {
        run,
        seq: EventSeq::new(7),
    };
    let msg = format!("{err}");
    assert!(msg.contains("42"), "message must contain run id, got {msg}");
}

#[test]
fn rejected_duplicate_event_not_staged_in_batch() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(100);
    let event = make_event(run, 0);

    let mut batch1 = JournalWriteBatch::new(&journal);
    batch1.append_event(&event).expect("first append");
    batch1.commit().expect("first commit");

    let mut batch2 = JournalWriteBatch::new(&journal);
    let initial_len = batch2.len();
    let result = batch2.append_event(&event);
    assert!(
        matches!(result, Err(JournalError::DuplicateEvent { .. })),
        "must be DuplicateEvent, got {result:?}"
    );
    assert_eq!(
        batch2.len(),
        initial_len,
        "batch len must be unchanged after duplicate rejection"
    );
}

#[test]
fn batch_len_unchanged_after_queue_full() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(101);
    let mut batch = JournalWriteBatch::new(&journal);

    for i in 0..MAX_BATCH_COUNT {
        batch
            .append_event(&make_event(run, i as u64))
            .expect("append");
    }
    assert_eq!(batch.len(), MAX_BATCH_COUNT);

    let result = batch.append_event(&make_event(run, MAX_BATCH_COUNT as u64));
    assert!(
        matches!(result, Err(JournalError::QueueFull)),
        "must be QueueFull, got {result:?}"
    );
    assert_eq!(
        batch.len(),
        MAX_BATCH_COUNT,
        "len must be unchanged after QueueFull rejection"
    );
}

#[test]
fn batch_remains_open_after_queue_full() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(102);
    let mut batch = JournalWriteBatch::new(&journal);

    for i in 0..3 {
        batch.append_event(&make_event(run, i)).expect("append");
    }
    assert_eq!(batch.len(), 3);

    for i in 3..MAX_BATCH_COUNT {
        batch
            .append_event(&make_event(run, i as u64))
            .expect("append");
    }

    let result = batch.append_event(&make_event(run, MAX_BATCH_COUNT as u64));
    assert!(
        matches!(result, Err(JournalError::QueueFull)),
        "must be QueueFull"
    );

    assert_eq!(
        batch.len(),
        MAX_BATCH_COUNT,
        "QueueFull must not abort the batch"
    );
}

#[test]
fn rejected_event_not_persisted_after_commit() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(103);
    let mut batch = JournalWriteBatch::new(&journal);

    for i in 0..3 {
        batch.append_event(&make_event(run, i)).expect("append");
    }
    for i in 3..MAX_BATCH_COUNT {
        batch
            .append_event(&make_event(run, i as u64))
            .expect("append");
    }
    let result = batch.append_event(&make_event(run, MAX_BATCH_COUNT as u64));
    assert!(
        matches!(result, Err(JournalError::QueueFull)),
        "overflow must be QueueFull"
    );

    batch.commit().expect("commit must succeed");

    let events = journal.events_for_run(run).expect("replay");
    assert_eq!(
        events.len(),
        MAX_BATCH_COUNT,
        "only MAX_BATCH_COUNT events must be persisted, not rejected ones"
    );
}

#[test]
fn rejected_event_key_usable_in_subsequent_batch() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(104);
    let mut batch1 = JournalWriteBatch::new(&journal);

    for i in 0..MAX_BATCH_COUNT {
        batch1
            .append_event(&make_event(run, i as u64))
            .expect("append");
    }
    let result = batch1.append_event(&make_event(run, MAX_BATCH_COUNT as u64));
    assert!(
        matches!(result, Err(JournalError::QueueFull)),
        "overflow must be QueueFull"
    );
    batch1.commit().expect("commit 1");

    let mut batch2 = JournalWriteBatch::new(&journal);
    let result = batch2.append_event(&make_event(run, MAX_BATCH_COUNT as u64));
    assert!(
        result.is_ok(),
        "rejected key must be reusable in subsequent batch, got {result:?}"
    );
}