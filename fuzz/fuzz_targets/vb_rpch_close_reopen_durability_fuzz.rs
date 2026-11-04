#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_core::{RunId, StepIdx};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

const MAX_BATCH_SIZE: usize = 12;

fuzz_target!(|data: &[u8]| {
    let run = RunId::new(1);
    let mode = data.first().copied().unwrap_or(0);
    let scenario = mode % 4;

    match scenario {
        0 => durability_close_reopen(run, data),
        1 => durability_partial_write(run, data),
        2 => durability_multiple_closes(run, data),
        _ => durability_concurrent_style(run, data),
    }
});

fn durability_close_reopen(run: RunId, data: &[u8]) {
    let event_count = bounded_count(data, 1, MAX_BATCH_SIZE);
    let Some(events) = build_events(run, event_count) else {
        return;
    };

    let dir = tempfile::tempdir().expect("setup: tempdir");
    {
        let journal = FjallJournal::open(dir.path(), None);
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
        let journal = FjallJournal::open(dir.path(), None);
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

fn durability_partial_write(run: RunId, data: &[u8]) {
    let batch1_count = bounded_count(data, 1, 4);
    let batch2_count = bounded_count(data, 1, 4);
    let Some(batch1) = build_events(run, batch1_count) else {
        return;
    };
    let Some(batch2) = build_events_from(run, batch1_count as u64, batch2_count) else {
        return;
    };

    let dir = tempfile::tempdir().expect("setup: tempdir");
    {
        let journal = FjallJournal::open(dir.path(), None);
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
        let journal = FjallJournal::open(dir.path(), None);
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
        let journal = FjallJournal::open(dir.path(), None);
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
                    batch1_count + batch2_count,
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

fn durability_multiple_closes(run: RunId, data: &[u8]) {
    let event_count = bounded_count(data, 1, 4);
    let Some(events) = build_events(run, event_count) else {
        return;
    };

    let dir = tempfile::tempdir().expect("setup: tempdir");
    {
        let journal = FjallJournal::open(dir.path(), None);
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
            let journal = FjallJournal::open(dir.path(), None);
            assert!(
                journal.is_ok(),
                "reopened journal must open: {:?}",
                journal.as_ref().err()
            );
            let Ok(mut journal) = journal else {
                return;
            };
            assert!(
                journal.close().is_ok(),
                "repeated close must succeed"
            );
        }
    }

    {
        let journal = FjallJournal::open(dir.path(), None);
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

fn durability_concurrent_style(run: RunId, data: &[u8]) {
    let run2 = RunId::new(2);
    let count1 = bounded_count(data, 1, 4);
    let count2 = bounded_count(data, 1, 4);
    let Some(events1) = build_events(run, count1) else {
        return;
    };
    let Some(events2) = build_events(run2, count2) else {
        return;
    };

    let dir = tempfile::tempdir().expect("setup: tempdir");
    {
        let journal = FjallJournal::open(dir.path(), None);
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
        let journal = FjallJournal::open(dir.path(), None);
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
                    count1,
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
                    count2,
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

fn bounded_count(data: &[u8], min: usize, max: usize) -> usize {
    if data.is_empty() {
        return min;
    }
    let byte = data[0];
    if max <= min {
        return min;
    }
    let span = max - min;
    min + (byte as usize) % span
}

fn build_events(run: RunId, count: usize) -> Option<Vec<JournalEvent>> {
    let mut events = Vec::with_capacity(count);
    for i in 0..count {
        let seq = EventSeq::new(i as u64);
        let step = StepIdx::new(i as u16);
        events.push(JournalEvent::StepStarted {
            run,
            seq,
            step,
            attempt: 1,
        });
    }
    Some(events)
}

fn build_events_from(run: RunId, start_seq: u64, count: usize) -> Option<Vec<JournalEvent>> {
    let mut events = Vec::with_capacity(count);
    for i in 0..count {
        let seq = EventSeq::new(start_seq + i as u64);
        let step = StepIdx::new(i as u16);
        events.push(JournalEvent::StepStarted {
            run,
            seq,
            step,
            attempt: 1,
        });
    }
    Some(events)
}
