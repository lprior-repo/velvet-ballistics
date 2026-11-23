use super::*;
pub(super) fn recover_with_mismatched_run(run: RunId, _data: &[u8]) {
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
        let other_run = RunId::new(999);
        let slot = SlotIdx::new(0);
        let entries = vec![(slot, SlotValue::I64(42), Taint::Clean)];
        let slot_data = postcard::to_allocvec(&entries).expect("encode slot entries");
        let taint_data = slot_data.clone();

        let snapshot = RunSnapshot {
            run: other_run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
            slots: slot_data,
            taint: taint_data,
        };

        assert!(
            write_snapshot(&journal, &snapshot).is_ok(),
            "write snapshot must succeed"
        );

        let tail_event = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
            attempt: 1,
        };
        assert!(
            journal.append_strict_batch(&[tail_event]).is_ok(),
            "append tail event must succeed"
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
            .snapshot(RunId::new(999), EventSeq::new(0))
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
                        RecoveryError::Journal(_) | RecoveryError::CorruptSnapshot { .. }
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

pub(super) fn recover_no_snapshot(run: RunId, data: &[u8]) {
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

        let tail_count = data.get(1).copied().unwrap_or(0) as usize % 3;
        for i in 0..tail_count {
            let seq = EventSeq::new(i as u64);
            let step = StepIdx::new((i as u16).wrapping_add(1));
            let event = JournalEvent::StepStarted {
                run,
                seq,
                step,
                attempt: 1,
            };
            assert!(
                journal.append_strict_batch(&[event]).is_ok(),
                "append tail event must succeed"
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

        let stored_snapshot = journal
            .snapshot(run, EventSeq::new(0))
            .expect("get snapshot");
        assert!(stored_snapshot.is_none(), "no snapshot should be stored");

        let tail_events = journal.events_for_run(run);
        let tail_events = match tail_events {
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

        use vb_core::WorkflowDigest;
        let empty_snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
            slots: Vec::new(),
            taint: Vec::new(),
        };

        let mut tracker = ActionReplayTracker::new();
        let result = recover_snapshot_plus_tail(&empty_snapshot, &tail_events, &mut tracker, &[]);
        assert_recovery_result(result);
        assert!(
            journal.close().is_ok(),
            "reopened journal close must succeed"
        );
    }
}
