#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_core::{RunId, SlotIdx, StepIdx};
use vb_storage::recovery::replay::core::recover_full_journal;
use vb_storage::recovery::types::{ActionReplayTracker, RecoveryError};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

type ScenarioResult = Result<(), Box<dyn std::error::Error>>;

fuzz_target!(|data: &[u8]| {
    let run = RunId::new(1);
    let (seed, records): (u8, &[u8]) = match data.split_first() {
        Some((&seed, records)) => (seed, records),
        None => (0, &[]),
    };
    let scenario = seed % 4;
    let result = match scenario {
        0 | 1 => recover_from_journal(run, &build_full_journal_events(records, run, seed)),
        2 => recover_with_corrupt_events(run, &build_full_journal_events(records, run, seed), data),
        _ => recover_empty_journal(run),
    };
    assert!(
        result.is_ok(),
        "full-journal scenario {scenario}: {result:?}"
    );
});

fn build_full_journal_events(data: &[u8], run: RunId, count: u8) -> Vec<JournalEvent> {
    let (records, _) = data.as_chunks::<4>();
    let slot = SlotIdx::new(0);
    (0..u64::from(count))
        .zip(records)
        .map(|(index, &[mode, step_byte, _, _])| {
            let seq = EventSeq::new(index);
            let step = StepIdx::new(u16::from(step_byte));
            match mode % 4 {
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
            }
        })
        .collect()
}

fn recover_from_journal(run: RunId, events: &[JournalEvent]) -> ScenarioResult {
    let dir = tempfile::tempdir()?;
    {
        let mut journal = FjallJournal::open(dir.path(), None)?;
        journal.append_strict_batch(events)?;
        journal.close()?;
    }
    {
        let mut journal = FjallJournal::open(dir.path(), None)?;
        let mut tracker = ActionReplayTracker::new();
        let result = recover_full_journal(&journal, run, &mut tracker, &[], &[]);
        check_recovery_result(result)?;
        journal.close()?;
    }
    Ok(())
}

fn recover_with_corrupt_events(run: RunId, events: &[JournalEvent], data: &[u8]) -> ScenarioResult {
    let dir = tempfile::tempdir()?;
    {
        let mut journal = FjallJournal::open(dir.path(), None)?;
        if !events.is_empty() {
            journal.append_strict_batch(events)?;
        }
        if data.len() > 10 {
            journal.inject_seq_gap(run, EventSeq::new(999))?;
        }
        journal.close()?;
    }
    {
        let mut journal = FjallJournal::open(dir.path(), None)?;
        let mut tracker = ActionReplayTracker::new();
        let result = recover_full_journal(&journal, run, &mut tracker, &[], &[]);
        match result {
            Ok(_)
            | Err(
                RecoveryError::Journal(_)
                | RecoveryError::ReplayDivergence { .. }
                | RecoveryError::NoRecoveryData { .. },
            ) => {}
            Err(error) => return Err(error.into()),
        }
        journal.close()?;
    }
    Ok(())
}

fn recover_empty_journal(run: RunId) -> ScenarioResult {
    let dir = tempfile::tempdir()?;
    {
        let mut journal = FjallJournal::open(dir.path(), None)?;
        journal.close()?;
    }
    {
        let mut journal = FjallJournal::open(dir.path(), None)?;
        let mut tracker = ActionReplayTracker::new();
        let result = recover_full_journal(&journal, run, &mut tracker, &[], &[]);
        match result {
            Err(RecoveryError::NoRecoveryData { .. }) => {}
            Err(error) => return Err(error.into()),
            Ok(_) => return Err("empty journal must yield NoRecoveryData".into()),
        }
        journal.close()?;
    }
    Ok(())
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
            | RecoveryError::NoRecoveryData { .. },
        ) => Ok(()),
        Err(error) => Err(error),
    }
}
