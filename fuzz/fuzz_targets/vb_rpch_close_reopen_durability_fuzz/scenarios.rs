use std::path::Path;

use super::*;

pub(super) fn durability_close_reopen(run: RunId, data: &[u8], dir: &Path) {
    let event_count = bounded_count(data, 1, MAX_BATCH_SIZE);
    let events = build_events(run, event_count);

    {
        let journal = FjallJournal::open(dir, None);
        assert!(
            journal.is_ok(),
            "initial journal open must succeed: {:?}",
            journal.as_ref().err()
        );
        let Ok(mut journal) = journal else {
            return;
        };
        assert!(
            journal.append_strict_batch(&events).is_ok(),
            "strict batch append must persist generated contiguous events"
        );
        assert!(
            journal.close().is_ok(),
            "journal close must report strict durability"
        );
    }
    {
        let journal = FjallJournal::open(dir, None);
        assert!(
            journal.is_ok(),
            "reopened journal must open: {:?}",
            journal.as_ref().err()
        );
        let Ok(mut journal) = journal else {
            return;
        };
        let replayed = journal.events_for_run_full(run);
        match replayed {
            Ok(replayed_events) => {
                assert_eq!(
                    replayed_events.len(),
                    events.len(),
                    "replayed events must equal written count after close/reopen"
                );
            }
            Err(error) => {
                assert!(
                    matches!(error, vb_storage::JournalError::PostcardDecodeFailed(_)),
                    "unexpected journal error after durability test: {:?}",
                    error
                );
            }
        }
        assert!(
            journal.close().is_ok(),
            "reopened journal close must succeed"
        );
    }
}

pub(super) fn durability_partial_write(run: RunId, data: &[u8], dir: &Path) {
    let batch1_count = bounded_count(data, 1, 4);
    let batch2_count = bounded_count(data, 1, 4);
    let batch1 = build_events(run, batch1_count);
    let batch2 = build_events_from(run, u64::from(batch1_count), batch2_count);

    {
        let journal = FjallJournal::open(dir, None);
        assert!(
            journal.is_ok(),
            "initial journal open must succeed: {:?}",
            journal.as_ref().err()
        );
        let Ok(mut journal) = journal else {
            return;
        };
        assert!(
            journal.append_strict_batch(&batch1).is_ok(),
            "first strict batch must persist"
        );
        assert!(
            journal.close().is_ok(),
            "intermediate close must report strict durability"
        );
    }

    {
        let journal = FjallJournal::open(dir, None);
        assert!(
            journal.is_ok(),
            "reopened journal must open: {:?}",
            journal.as_ref().err()
        );
        let Ok(mut journal) = journal else {
            return;
        };
        assert!(
            journal.append_strict_batch(&batch2).is_ok(),
            "second strict batch must persist"
        );
        assert!(
            journal.close().is_ok(),
            "final close must report strict durability"
        );
    }

    {
        let journal = FjallJournal::open(dir, None);
        assert!(
            journal.is_ok(),
            "verify: journal open must succeed: {:?}",
            journal.as_ref().err()
        );
        let Ok(mut journal) = journal else {
            return;
        };
        let replayed = journal.events_for_run_full(run);
        match replayed {
            Ok(replayed_events) => {
                assert_eq!(
                    replayed_events.len(),
                    batch1.len().saturating_add(batch2.len()),
                    "all batches must survive close/reopen/append"
                );
            }
            Err(error) => {
                assert!(
                    matches!(error, vb_storage::JournalError::PostcardDecodeFailed(_)),
                    "unexpected journal error in partial write test: {:?}",
                    error
                );
            }
        }
        assert!(
            journal.close().is_ok(),
            "verify: journal close must succeed"
        );
    }
}

pub(super) fn durability_multiple_closes(run: RunId, data: &[u8], dir: &Path) {
    let event_count = bounded_count(data, 1, 4);
    let events = build_events(run, event_count);

    {
        let journal = FjallJournal::open(dir, None);
        assert!(
            journal.is_ok(),
            "initial journal open must succeed: {:?}",
            journal.as_ref().err()
        );
        let Ok(mut journal) = journal else {
            return;
        };
        assert!(
            journal.append_strict_batch(&events).is_ok(),
            "strict batch append must persist generated contiguous events"
        );
        assert!(
            journal.close().is_ok(),
            "first close must report strict durability"
        );
    }

    for _i in 0..3 {
        {
            let journal = FjallJournal::open(dir, None);
            assert!(
                journal.is_ok(),
                "reopened journal must open: {:?}",
                journal.as_ref().err()
            );
            let Ok(mut journal) = journal else {
                return;
            };
            assert!(journal.close().is_ok(), "repeated close must succeed");
        }
    }

    {
        let journal = FjallJournal::open(dir, None);
        assert!(
            journal.is_ok(),
            "final verify: journal open must succeed: {:?}",
            journal.as_ref().err()
        );
        let Ok(mut journal) = journal else {
            return;
        };
        let replayed = journal.events_for_run_full(run);
        match replayed {
            Ok(replayed_events) => {
                assert_eq!(
                    replayed_events.len(),
                    events.len(),
                    "events must survive multiple close/reopen cycles"
                );
            }
            Err(error) => {
                assert!(
                    matches!(error, vb_storage::JournalError::PostcardDecodeFailed(_)),
                    "unexpected journal error in multiple close test: {:?}",
                    error
                );
            }
        }
        assert!(
            journal.close().is_ok(),
            "final verify: journal close must succeed"
        );
    }
}

pub(super) fn durability_concurrent_style(run: RunId, data: &[u8], dir: &Path) {
    let run2 = RunId::new(2);
    let count1 = bounded_count(data, 1, 4);
    let count2 = bounded_count(data, 1, 4);
    let events1 = build_events(run, count1);
    let events2 = build_events(run2, count2);

    {
        let journal = FjallJournal::open(dir, None);
        assert!(
            journal.is_ok(),
            "initial journal open must succeed: {:?}",
            journal.as_ref().err()
        );
        let Ok(mut journal) = journal else {
            return;
        };
        assert!(
            journal.append_strict_batch(&events1).is_ok(),
            "run1 strict batch must persist"
        );
        assert!(
            journal.append_strict_batch(&events2).is_ok(),
            "run2 strict batch must persist"
        );
        assert!(
            journal.close().is_ok(),
            "close must report strict durability"
        );
    }

    {
        let journal = FjallJournal::open(dir, None);
        assert!(
            journal.is_ok(),
            "verify: journal open must succeed: {:?}",
            journal.as_ref().err()
        );
        let Ok(mut journal) = journal else {
            return;
        };
        let replayed1 = journal.events_for_run_full(run);
        match replayed1 {
            Ok(replayed) => {
                assert_eq!(
                    replayed.len(),
                    events1.len(),
                    "run1 must see only its own events"
                );
            }
            Err(error) => {
                assert!(
                    matches!(error, vb_storage::JournalError::PostcardDecodeFailed(_)),
                    "unexpected journal error for run1: {:?}",
                    error
                );
            }
        }
        let replayed2 = journal.events_for_run_full(run2);
        match replayed2 {
            Ok(replayed) => {
                assert_eq!(
                    replayed.len(),
                    events2.len(),
                    "run2 must see only its own events"
                );
            }
            Err(error) => {
                assert!(
                    matches!(error, vb_storage::JournalError::PostcardDecodeFailed(_)),
                    "unexpected journal error for run2: {:?}",
                    error
                );
            }
        }
        assert!(
            journal.close().is_ok(),
            "verify: journal close must succeed"
        );
    }
}
