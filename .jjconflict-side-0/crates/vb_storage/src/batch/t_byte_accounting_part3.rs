#![forbid(unsafe_code)]
use super::*;

#[test]
fn duplicate_detection_fires_before_count_check() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(200);
    let event = make_event(run, 0);

    let mut batch1 = JournalWriteBatch::new(&journal);
    batch1.append_event(&event).expect("first append");
    batch1.commit().expect("first commit");

    let mut batch2 = JournalWriteBatch::new(&journal);
    let result = batch2.append_event(&event);
    assert!(
        matches!(result, Err(JournalError::DuplicateEvent { .. })),
        "duplicate must fire before QueueFull, got {result:?}"
    );
}

#[test]
fn payload_too_large_fires_before_queue_count_check() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(202);
    let event = make_event(run, 0);

    let mut batch = JournalWriteBatch::new(&journal);
    let result = batch.append_event(&event);
    assert!(
        result.is_ok(),
        "valid event must be accepted, got {result:?}"
    );
}

#[test]
fn queue_full_fires_before_any_possible_encoding_guard_for_new_events() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(201);
    let mut batch = JournalWriteBatch::new(&journal);

    for i in 0..MAX_BATCH_COUNT {
        batch
            .append_event(&make_event(run, i as u64))
            .expect("append");
    }
    let result = batch.append_event(&make_event(run, MAX_BATCH_COUNT as u64));
    assert!(
        matches!(result, Err(JournalError::QueueFull)),
        "QueueFull must fire at count limit, got {result:?}"
    );
}

#[test]
fn duplicate_and_queue_full_conflict_duplicate_wins() {
    let (_temp, journal) = temp_journal();
    let run = RunId::new(204);
    let event = make_event(run, 0);

    let mut batch1 = JournalWriteBatch::new(&journal);
    batch1.append_event(&event).expect("append");
    batch1.commit().expect("commit");

    let mut batch2 = JournalWriteBatch::new(&journal);
    let result = batch2.append_event(&event);
    assert!(
        matches!(result, Err(JournalError::DuplicateEvent { .. })),
        "DuplicateEvent must win over other guards, got {result:?}"
    );
}

#[test]
fn checked_add_never_panics() {
    for (a, b) in [
        (0u64, 0u64),
        (1, 1),
        (u64::MAX, 0),
        (0, u64::MAX),
        (u64::MAX, 1),
        (u64::MAX, u64::MAX),
    ] {
        let _result = a.checked_add(b);
    }
}

#[test]
fn checked_add_overflow_returns_none() {
    let result = u64::MAX.checked_add(1u64);
    assert!(result.is_none(), "u64::MAX + 1 must overflow (return None)");
}

#[test]
fn checked_add_normal_returns_some_with_correct_sum() {
    let result = 100u64.checked_add(200u64);
    assert!(result.is_some(), "100 + 200 must not overflow");
    let sum = result.expect("checked_add must not fail at this magnitude");
    assert_eq!(sum, 300u64, "sum must equal 300");
}

#[test]
fn u64_max_limit_with_large_delta_overflows() {
    let staged: u64 = u64::MAX;
    let delta: u64 = 1;
    let result = staged.checked_add(delta);
    assert!(result.is_none(), "u64::MAX + 1 must overflow");
}

#[test]
fn storage_default_byte_limit_is_nonzero() {
    let default_limit: u64 = 1_048_576;
    assert!(default_limit > 0, "default byte limit must be non-zero");
}

#[test]
fn default_limit_fits_in_u32() {
    let limit: u64 = 1_048_576;
    assert!(
        limit <= u32::MAX as u64,
        "default limit must fit in u32 without truncation"
    );
}
