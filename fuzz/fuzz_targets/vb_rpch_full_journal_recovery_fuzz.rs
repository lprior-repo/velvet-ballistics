#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_core::{RunId, SlotIdx, StepIdx};
use vb_storage::recovery::replay::core::recover_full_journal;
use vb_storage::recovery::types::{ActionReplayTracker, RecoveryError};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};


fuzz_target!(|data: &[u8]| {
    let run = RunId::new(1);
    let Some(count) = event_count_from_seed(data) else {
        return;
    };
    let Some(events) = build_full_journal_events(data, run, count) else {
        return;
    };
    observe_full_journal_recovery(data, run, &events);
});

fn event_count_from_seed(data: &[u8]) -> Option<usize> {
    let byte = data.first().copied().unwrap_or(0);
    if byte == 0 {
        return Some(0);
    }
    Some(byte as usize)
}

fn build_full_journal_events(data: &[u8], run: RunId, count: usize) -> Option<Vec<JournalEvent>> {
    let mut events = Vec::with_capacity(count);
    for i in 0..count {
        let chunk_start = 1 + i * 4;
        if chunk_start + 4 > data.len() {
            break;
        }
        let mode = data[chunk_start];
        let step_byte = data[chunk_start + 1];
        let seq = EventSeq::new(i as u64);
        let step = StepIdx::new(step_byte as u16);
        let slot = SlotIdx::new(0);
        let event = match mode % 4 {
            0 => JournalEvent::StepStarted {
                run,
                seq,
                step,
                attempt: 1,
            },
            1 => JournalEvent::StepSucceeded {
                run,
                seq,
                step,
                output: slot,
            },
            2 => JournalEvent::RunFinished {
                run,
                seq,
                result: slot,
                attempt: 1,
            },
            _ => JournalEvent::RunCancelled {
                run,
                seq,
                attempt: 1,
                reason: None,
            },
        };
        events.push(event);
    }
    Some(events)
}

fn observe_full_journal_recovery(data: &[u8], run: RunId, events: &[JournalEvent]) {
    let mode = data.first().copied().unwrap_or(0);
    let scenario = mode % 4;

    match scenario {
        0 | 1 => recover_from_journal(run, events),
        2 => recover_with_corrupt_events(run, events, data),
        _ => recover_empty_journal(run),
    }
}

fn recover_from_journal(run: RunId, events: &[JournalEvent]) {
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
            journal.append_strict_batch(events).is_ok(),
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
        let mut tracker = ActionReplayTracker::new();
        let result = recover_full_journal(&journal, run, &mut tracker, &[], &[]);
        assert_recovery_result(result);
        assert!(
            journal.close().is_ok(),
            "reopened journal close must succeed"
        );
    }
}

fn recover_with_corrupt_events(run: RunId, events: &[JournalEvent], data: &[u8]) {
    let dir = tempfile::tempdir().expect("setup: tempdir");
    {
        let journal = FjallJournal::open(dir.path(), None);
        assert!(
            journal.is_ok(),
            "corrupt-event setup journal open must succeed: {:?}",
            journal.as_ref().err()
        );
        let Ok(mut journal) = journal else {
            return;
        };
        if !events.is_empty() {
            assert!(
                journal.append_strict_batch(events).is_ok(),
                "strict batch append must persist generated contiguous events"
            );
        }
        if data.len() > 10 {
            assert!(
                journal.inject_seq_gap(run, EventSeq::new(999)).is_ok(),
                "raw non-event payload injection must persist for corruption oracle"
            );
        }
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
        let mut tracker = ActionReplayTracker::new();
        let result = recover_full_journal(&journal, run, &mut tracker, &[], &[]);
        match result {
            Ok(_) => {}
            Err(error) => {
                assert!(
                    matches!(
                        error,
                        RecoveryError::Journal(_)
                            | RecoveryError::ReplayDivergence { .. }
                            | RecoveryError::NoRecoveryData { .. }
                    ),
                    "unexpected recovery error: {:?}",
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

fn recover_empty_journal(run: RunId) {
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
        let mut tracker = ActionReplayTracker::new();
        let result = recover_full_journal(&journal, run, &mut tracker, &[], &[]);
        assert!(
            matches!(result, Err(RecoveryError::NoRecoveryData { .. })),
            "empty journal must yield NoRecoveryData"
        );
        assert!(
            journal.close().is_ok(),
            "reopened journal close must succeed"
        );
    }
}

fn assert_recovery_result(result: Result<Vec<JournalEvent>, RecoveryError>) {
    match result {
        Ok(_replayed) => {
            // len() is usize, always >= 0
        }
        Err(error) => {
            assert!(
                matches!(
                    error,
                    RecoveryError::Journal(_)
                        | RecoveryError::ReplayDivergence { .. }
                        | RecoveryError::PolicyDigestMismatch { .. }
                        | RecoveryError::ActionAbiMismatch { .. }
                        | RecoveryError::NoRecoveryData { .. }
                ),
                "unexpected recovery error: {:?}",
                error
            );
        }
    }
}
