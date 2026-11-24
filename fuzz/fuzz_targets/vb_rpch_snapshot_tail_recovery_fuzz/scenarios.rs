use super::*;
pub(super) fn recover_with_mismatched_run(run: RunId, _data: &[u8]) -> ScenarioResult {
    let dir = tempfile::tempdir()?;
    let other_run = RunId::new(999);
    {
        let mut journal = FjallJournal::open(dir.path(), None)?;

        use vb_core::WorkflowDigest;
        let slot = SlotIdx::new(0);
        let entries = [(slot, SlotValue::I64(42), Taint::Clean)];
        let slot_data = postcard::to_allocvec(entries.as_slice())?;
        let taint_data = slot_data.clone();

        let snapshot = RunSnapshot {
            run: other_run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
            slots: slot_data,
            taint: taint_data,
        };

        write_snapshot(&journal, &snapshot)?;

        let tail_event = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
            attempt: 1,
        };
        journal.append_strict_batch(&[tail_event])?;
        journal.close()?;
    }

    {
        let mut journal = FjallJournal::open(dir.path(), None)?;
        let stored_snapshot = journal
            .snapshot(other_run, EventSeq::new(0))?
            .ok_or("stored snapshot must exist")?;

        let mut tracker = ActionReplayTracker::new();
        let result = recover_snapshot_plus_tail(&stored_snapshot, &[], &mut tracker, &[]);
        match result {
            Ok(_) | Err(RecoveryError::Journal(_) | RecoveryError::CorruptSnapshot { .. }) => {}
            Err(error) => return Err(error.into()),
        }
        journal.close()?;
    }
    Ok(())
}

pub(super) fn recover_no_snapshot(run: RunId, data: &[u8]) -> ScenarioResult {
    let dir = tempfile::tempdir()?;
    {
        let mut journal = FjallJournal::open(dir.path(), None)?;
        let tail_count = data.get(1).map_or(0, |&byte| byte % 3);
        (0..tail_count)
            .try_for_each(|index| journal.append_strict_batch(&[tail_event(run, index)]))?;
        journal.close()?;
    }

    {
        let mut journal = FjallJournal::open(dir.path(), None)?;
        if journal.snapshot(run, EventSeq::new(0))?.is_some() {
            return Err("no snapshot should be stored".into());
        }
        let tail_events = read_tail(&journal, run)?;

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
        check_recovery_result(result)?;
        journal.close()?;
    }
    Ok(())
}
