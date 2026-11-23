#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_core::{RunId, SlotIdx, SlotValue, StepIdx, Taint};
use vb_storage::recovery::replay::core::recover_snapshot_plus_tail;
use vb_storage::recovery::types::{ActionReplayTracker, RecoveryError, RunSnapshot};
use vb_storage::{EventSeq, FjallJournal, JournalEvent, write_snapshot};

const MAX_TAIL_EVENTS: usize = 8;

fuzz_target!(|data: &[u8]| {
    let run = RunId::new(1);
    let mode = data.first().copied().unwrap_or(0);
    let scenario = mode % 5;

    match scenario {
        0 | 1 => recover_from_snapshot_and_tail(run, data),
        2 => recover_with_corrupt_snapshot(run, data),
        3 => recover_with_mismatched_run(run, data),
        _ => recover_no_snapshot(run, data),
    }
});

fn recover_from_snapshot_and_tail(run: RunId, data: &[u8]) {
    let slot = SlotIdx::new(0);
    let slot_value = SlotValue::I64(42);
    let entries = vec![(slot, slot_value, Taint::Clean)];
    let slot_data = postcard::to_allocvec(&entries).expect("encode slot entries");
    let taint_data = slot_data.clone();

    use vb_core::WorkflowDigest;
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
        slots: slot_data,
        taint: taint_data,
    };

    let tail_count = data.get(1).copied().unwrap_or(0) as usize % MAX_TAIL_EVENTS;
    let mut tail_events = Vec::new();
    for i in 0..tail_count {
        let seq = EventSeq::new(i as u64);
        let step = StepIdx::new((i as u16).wrapping_add(1));
        let event = JournalEvent::StepStarted {
            run,
            seq,
            step,
            attempt: 1,
        };
        tail_events.push(event);
    }

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

        if !tail_events.is_empty() {
            assert!(
                journal.append_strict_batch(&tail_events).is_ok(),
                "strict batch append must persist generated contiguous events"
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

        let replayed_events = journal.events_for_run(run);
        let replayed_events = match replayed_events {
            Ok(events) => events,
            Err(error) => {
                assert!(
                    matches!(error, vb_storage::JournalError::PostcardDecodeFailed(_)),
                    "unexpected journal error: {:?}",
                    error
                );
                Vec::new()
            }
        };

        let mut tracker = ActionReplayTracker::new();
        let result = recover_snapshot_plus_tail(&snapshot, &replayed_events, &mut tracker, &[]);
        assert_recovery_result(result);
        assert!(
            journal.close().is_ok(),
            "reopened journal close must succeed"
        );
    }
}

fn recover_with_corrupt_snapshot(run: RunId, _data: &[u8]) {
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

        use vb_core::WorkflowDigest;
        let corrupt_slots = vec![0xFF, 0xFE, 0xFD];
        let corrupt_taint = corrupt_slots.clone();
        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
            slots: corrupt_slots,
            taint: corrupt_taint,
        };

        assert!(
            write_snapshot(&journal, &snapshot).is_ok(),
            "write snapshot must succeed"
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

        let stored_snapshot = journal
            .snapshot(run, EventSeq::new(0))
            .expect("get snapshot");
        let stored_snapshot = stored_snapshot.expect("stored snapshot must exist");

        let mut tracker = ActionReplayTracker::new();
        let result = recover_snapshot_plus_tail(&stored_snapshot, &[], &mut tracker, &[]);
        match result {
            Ok(_) => {}
            Err(error) => {
                assert!(
                    matches!(
                        error,
                        RecoveryError::CorruptSnapshot { .. }
                            | RecoveryError::Journal(_)
                            | RecoveryError::SlotTaintReadFailed { .. }
                            | RecoveryError::CorruptSlotTaint { .. }
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

#[path = "vb_rpch_snapshot_tail_recovery_fuzz/scenarios.rs"]
mod scenarios;

use scenarios::*;

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
                        | RecoveryError::CorruptSnapshot { .. }
                        | RecoveryError::CorruptSlotTaint { .. }
                        | RecoveryError::SlotTaintReadFailed { .. }
                ),
                "unexpected recovery error: {:?}",
                error
            );
        }
    }
}
