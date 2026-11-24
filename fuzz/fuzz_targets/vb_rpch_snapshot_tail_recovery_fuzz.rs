#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_core::{RunId, SlotIdx, SlotValue, StepIdx, Taint};
use vb_storage::recovery::replay::core::recover_snapshot_plus_tail;
use vb_storage::recovery::types::{ActionReplayTracker, RecoveryError, RunSnapshot};
use vb_storage::{EventSeq, FjallJournal, JournalError, JournalEvent, write_snapshot};

const MAX_TAIL_EVENTS: u8 = 8;
type ScenarioResult = Result<(), Box<dyn std::error::Error>>;

fuzz_target!(|data: &[u8]| {
    let run = RunId::new(1);
    let scenario = data.first().map_or(0, |&byte| byte % 5);

    let result = match scenario {
        0 | 1 => recover_from_snapshot_and_tail(run, data),
        2 => recover_with_corrupt_snapshot(run, data),
        3 => recover_with_mismatched_run(run, data),
        _ => recover_no_snapshot(run, data),
    };
    assert!(result.is_ok(), "recovery scenario {scenario}: {result:?}");
});

fn recover_from_snapshot_and_tail(run: RunId, data: &[u8]) -> ScenarioResult {
    let slot = SlotIdx::new(0);
    let slot_value = SlotValue::I64(42);
    let entries = [(slot, slot_value, Taint::Clean)];
    let slot_data = postcard::to_allocvec(entries.as_slice())?;
    let taint_data = slot_data.clone();

    use vb_core::WorkflowDigest;
    let snapshot = RunSnapshot {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
        slots: slot_data,
        taint: taint_data,
    };

    let tail_count = data.get(1).map_or(0, |&byte| byte % MAX_TAIL_EVENTS);
    let tail_events: Vec<_> = (0..tail_count)
        .map(|index| tail_event(run, index))
        .collect();

    let dir = tempfile::tempdir()?;
    {
        let mut journal = FjallJournal::open(dir.path(), None)?;

        if !tail_events.is_empty() {
            journal.append_strict_batch(&tail_events)?;
        }

        journal.close()?;
    }

    {
        let mut journal = FjallJournal::open(dir.path(), None)?;

        let replayed_events = read_tail(&journal, run)?;

        let mut tracker = ActionReplayTracker::new();
        let result = recover_snapshot_plus_tail(&snapshot, &replayed_events, &mut tracker, &[]);
        check_recovery_result(result)?;
        journal.close()?;
    }
    Ok(())
}

fn recover_with_corrupt_snapshot(run: RunId, _data: &[u8]) -> ScenarioResult {
    let dir = tempfile::tempdir()?;
    {
        let mut journal = FjallJournal::open(dir.path(), None)?;

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

        write_snapshot(&journal, &snapshot)?;
        journal.close()?;
    }

    {
        let mut journal = FjallJournal::open(dir.path(), None)?;
        let stored_snapshot = journal
            .snapshot(run, EventSeq::new(0))?
            .ok_or("stored snapshot must exist")?;

        let mut tracker = ActionReplayTracker::new();
        let result = recover_snapshot_plus_tail(&stored_snapshot, &[], &mut tracker, &[]);
        match result {
            Ok(_)
            | Err(
                RecoveryError::CorruptSnapshot { .. }
                | RecoveryError::Journal(_)
                | RecoveryError::SlotTaintReadFailed { .. }
                | RecoveryError::CorruptSlotTaint { .. },
            ) => {}
            Err(error) => return Err(error.into()),
        }
        journal.close()?;
    }
    Ok(())
}

fn tail_event(run: RunId, index: u8) -> JournalEvent {
    JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(u64::from(index)),
        step: StepIdx::new(u16::from(index).wrapping_add(1)),
        attempt: 1,
    }
}

#[path = "vb_rpch_snapshot_tail_recovery_fuzz/scenarios.rs"]
mod scenarios;

use scenarios::*;

fn read_tail(journal: &FjallJournal, run: RunId) -> Result<Vec<JournalEvent>, JournalError> {
    match journal.events_for_run(run) {
        Err(JournalError::PostcardDecodeFailed(_)) => Ok(Vec::new()),
        result => result,
    }
}

fn check_recovery_result(
    result: Result<Vec<JournalEvent>, RecoveryError>,
) -> Result<(), RecoveryError> {
    match result {
        Ok(_)
        | Err(
            RecoveryError::Journal(_)
            | RecoveryError::ReplayDivergence { .. }
            | RecoveryError::PolicyDigestMismatch { .. }
            | RecoveryError::ActionAbiMismatch { .. }
            | RecoveryError::NoRecoveryData { .. }
            | RecoveryError::CorruptSnapshot { .. }
            | RecoveryError::CorruptSlotTaint { .. }
            | RecoveryError::SlotTaintReadFailed { .. },
        ) => Ok(()),
        Err(error) => Err(error),
    }
}
