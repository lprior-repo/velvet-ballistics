#![forbid(unsafe_code)]

use tempfile::TempDir;
use vb_core::{CapabilitySet, RunId, RuntimePolicy, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::recovery::{
    ActionReplayTracker, RecoveryHydration, RecoveryTerminalState, recover_full_journal,
    recover_runtime_summary,
};
use vb_storage::{EventSeq, FjallConfig, FjallJournal, JournalError, JournalEvent};

fn test_digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

fn open_journal(dir: &TempDir) -> Result<FjallJournal, String> {
    FjallJournal::open(dir.path(), Some(FjallConfig::default())).map_err(|error| error.to_string())
}

fn test_admission_event(run: RunId, seq: EventSeq, digest: WorkflowDigest) -> JournalEvent {
    JournalEvent::RunAdmission {
        run,
        seq,
        artifact_digest: digest,
        granted_capabilities: CapabilitySet::empty(),
        policy: RuntimePolicy::Relaxed,
    }
}

fn write_events_strict(journal: &FjallJournal, events: &[JournalEvent]) -> Result<(), String> {
    for event in events {
        journal
            .append_strict(event)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn resumed_run_events(run: RunId, digest: WorkflowDigest) -> Vec<JournalEvent> {
    vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        test_admission_event(run, EventSeq::new(1), digest),
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::RetryScheduledEvent {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(5),
            step: StepIdx::new(1),
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(6),
            slot: SlotIdx::new(2),
            value: None,
            extra: None,
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(7),
            step: StepIdx::new(1),
            output: SlotIdx::new(2),
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(8),
            result: SlotIdx::new(2),
            attempt: 1,
        },
    ]
}

#[test]
fn resume_tail_replays_exactly_when_journal_is_reopened() -> Result<(), String> {
    let dir = TempDir::new().map_err(|error| error.to_string())?;
    let run = RunId::new(16_200);
    let digest = test_digest(0x16);
    let expected = resumed_run_events(run, digest);

    {
        let journal = open_journal(&dir)?;
        write_events_strict(&journal, &expected)?;
    }

    let journal = open_journal(&dir)?;
    let recovered = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        recovered, expected,
        "reopened journal replay must preserve every pre-resume, resume-marker, and post-resume event"
    );

    let mut tracker = ActionReplayTracker::new();
    let full_replay = recover_full_journal(&journal, run, &mut tracker, &[], &[])
        .map_err(|error| error.to_string())?;
    assert_eq!(
        full_replay, expected,
        "full recovery replay must match the exact durable resume journal"
    );
    assert_eq!(
        tracker.is_resolved(vb_core::ActionId::new(1), StepIdx::ZERO),
        false,
        "timer/step resume replay must not invent resolved external actions"
    );

    let hydration = recover_runtime_summary(&journal, run).map_err(|error| error.to_string())?;
    let RecoveryHydration::Summary(summary) = hydration else {
        return Err(format!("expected summary hydration, got {hydration:?}"));
    };
    assert_eq!(summary.run, run);
    assert_eq!(summary.first_seq, EventSeq::new(0));
    assert_eq!(summary.last_seq, EventSeq::new(8));
    assert_eq!(summary.workflow, Some(digest));
    assert_eq!(summary.steps_started, 2);
    assert_eq!(summary.steps_succeeded, 1);
    assert_eq!(summary.suspensions, 2);
    assert_eq!(summary.slots_written, 1);
    assert_eq!(
        summary.terminal,
        Some(RecoveryTerminalState::Finished {
            result: SlotIdx::new(2)
        })
    );
    Ok(())
}

#[test]
fn resume_tail_replay_is_deterministic_when_read_twice() -> Result<(), String> {
    let dir = TempDir::new().map_err(|error| error.to_string())?;
    let run = RunId::new(16_201);
    let expected = resumed_run_events(run, test_digest(0x17));

    {
        let journal = open_journal(&dir)?;
        write_events_strict(&journal, &expected)?;
    }

    let (replay_a, full_a, action_resolved_a) = {
        let journal = open_journal(&dir)?;
        let replay = journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?;
        let mut tracker = ActionReplayTracker::new();
        let full = recover_full_journal(&journal, run, &mut tracker, &[], &[])
            .map_err(|error| error.to_string())?;
        let action_resolved = tracker.is_resolved(vb_core::ActionId::new(99), StepIdx::new(1));
        (replay, full, action_resolved)
    };

    let (replay_b, full_b, action_resolved_b) = {
        let journal = open_journal(&dir)?;
        let replay = journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?;
        let mut tracker = ActionReplayTracker::new();
        let full = recover_full_journal(&journal, run, &mut tracker, &[], &[])
            .map_err(|error| error.to_string())?;
        let action_resolved = tracker.is_resolved(vb_core::ActionId::new(99), StepIdx::new(1));
        (replay, full, action_resolved)
    };

    assert_eq!(replay_a, expected);
    assert_eq!(replay_b, expected);
    assert_eq!(replay_a, replay_b);
    assert_eq!(full_a, full_b);
    assert_eq!(action_resolved_a, action_resolved_b);
    Ok(())
}

#[test]
fn resume_tail_replay_rejects_sequence_gap_before_resume_continuation() -> Result<(), String> {
    let dir = TempDir::new().map_err(|error| error.to_string())?;
    let run = RunId::new(16_202);
    let digest = test_digest(0x18);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(1),
            attempt: 1,
        },
    ];

    {
        let journal = open_journal(&dir)?;
        write_events_strict(&journal, &events)?;
    }

    let journal = open_journal(&dir)?;
    let result = journal.events_for_run(run);
    let Err(JournalError::SequenceGap { expected, actual }) = result else {
        return Err(format!("expected SequenceGap, got {result:?}"));
    };
    assert_eq!(expected, EventSeq::new(2));
    assert_eq!(actual, EventSeq::new(3));

    let mut tracker = ActionReplayTracker::new();
    let full_result = recover_full_journal(&journal, run, &mut tracker, &[], &[]);
    let Err(vb_storage::recovery::RecoveryError::Journal(JournalError::SequenceGap {
        expected,
        actual,
    })) = full_result
    else {
        return Err(format!(
            "expected recovery SequenceGap, got {full_result:?}"
        ));
    };
    assert_eq!(expected, EventSeq::new(2));
    assert_eq!(actual, EventSeq::new(3));
    Ok(())
}
