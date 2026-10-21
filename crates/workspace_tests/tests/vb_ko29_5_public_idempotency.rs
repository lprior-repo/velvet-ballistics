//! Public behavior tests for bead vb-ko29.5 idempotency permutations.

use tempfile::TempDir;
use vb_core::action::{ActionTicket, Idempotency};
use vb_core::errors::CoreError;
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_runtime::admission::{AcceptedArtifactStore, AdmissionError, ArtifactEnvelopeError};
use vb_storage::{
    ActionReplayTracker, EventSeq, FjallJournal, JournalError, JournalEvent, replay_journal,
};
use velvet_ballistics::lifecycle;

struct FixedArtifactStore {
    artifact: vb_storage::AcceptedArtifact,
}

impl AcceptedArtifactStore for FixedArtifactStore {
    fn load_accepted_artifact(
        &self,
        _artifact_digest: WorkflowDigest,
    ) -> Result<vb_storage::AcceptedArtifact, ArtifactEnvelopeError> {
        Ok(self.artifact.clone())
    }
}

fn temp_journal() -> Result<(TempDir, FjallJournal), String> {
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let journal = FjallJournal::open(dir.path(), None).map_err(|error| error.to_string())?;
    Ok((dir, journal))
}

fn digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

fn ticket(run: u64, step: u16, key: u128) -> ActionTicket {
    ActionTicket {
        run: RunId::new(run),
        step: StepIdx::new(step),
        seq: SeqNo::new(0),
        action: ActionId::new(7),
        attempt: 1,
        idempotency_key: key,
        capacity: 3,
    }
}

fn valid_artifact(_artifact_digest: WorkflowDigest) -> vb_storage::AcceptedArtifact {
    let source = b"version: velvet-ballistics/v1\nname: idempotency_fixture\nwhen: { manual: {} }\nsteps:\n  - id: make\n    set: { output: answer, value: \"42\" }\n  - id: done\n    finish: { result: answer }\n";
    let workflow = vb_compile::compile_workflow(source).expect("compile fixture workflow");
    vb_storage::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])
        .expect("prepare fixture artifact")
}

fn append_run_accepted(
    journal: &FjallJournal,
    run: RunId,
    seq: u64,
) -> Result<JournalEvent, String> {
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(seq),
        workflow: digest(0xAA),
    };
    journal
        .append_strict(&event)
        .map_err(|error| error.to_string())?;
    Ok(event)
}

#[test]
fn given_duplicate_success_event_when_appended_then_duplicate_event_variant_and_count_unchanged()
-> Result<(), String> {
    // Given
    let (_dir, journal) = temp_journal()?;
    let run = RunId::new(10);
    let event = append_run_accepted(&journal, run, 0)?;
    let before = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?
        .len();

    // When
    let result = journal.append_strict(&event);

    // Then
    match result {
        Err(JournalError::DuplicateEvent { run: got, seq }) => {
            assert_eq!(got, run);
            assert_eq!(seq, EventSeq::new(0));
        }
        Err(other) => return Err(format!("expected DuplicateEvent, got {other:?}")),
        Ok(()) => return Err("expected duplicate success append to be rejected".to_string()),
    }
    let after = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?
        .len();
    assert_eq!(after, before);
    Ok(())
}

#[test]
fn given_duplicate_failure_event_when_appended_then_duplicate_event_variant_and_count_unchanged()
-> Result<(), String> {
    // Given
    let (_dir, journal) = temp_journal()?;
    let run = RunId::new(11);
    append_run_accepted(&journal, run, 0)?;
    let failed = JournalEvent::ActionFailedEvent {
        run,
        seq: EventSeq::new(1),
        step: StepIdx::new(2),
        action: ActionId::new(3),
        attempt: 1,
    };
    journal
        .append_strict(&failed)
        .map_err(|error| error.to_string())?;
    let before = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?
        .len();

    // When
    let result = journal.append_strict(&failed);

    // Then
    match result {
        Err(JournalError::DuplicateEvent { run: got, seq }) => {
            assert_eq!(got, run);
            assert_eq!(seq, EventSeq::new(1));
        }
        Err(other) => return Err(format!("expected DuplicateEvent, got {other:?}")),
        Ok(()) => return Err("expected duplicate failure append to be rejected".to_string()),
    }
    let after = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?
        .len();
    assert_eq!(after, before);
    Ok(())
}

#[test]
fn given_diverged_attempt_count_when_duplicate_appended_then_duplicate_event_variant()
-> Result<(), String> {
    // Given
    let (_dir, journal) = temp_journal()?;
    let run = RunId::new(12);
    append_run_accepted(&journal, run, 0)?;
    let failed = JournalEvent::ActionFailedEvent {
        run,
        seq: EventSeq::new(1),
        step: StepIdx::new(2),
        action: ActionId::new(3),
        attempt: 1,
    };
    journal
        .append_strict(&failed)
        .map_err(|error| error.to_string())?;
    let before = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?
        .len();

    // When
    let failed_again = JournalEvent::ActionFailedEvent {
        run,
        seq: EventSeq::new(1),
        step: StepIdx::new(2),
        action: ActionId::new(3),
        attempt: 2,
    };
    let result = journal.append_strict(&failed_again);

    // Then
    match result {
        Err(JournalError::DuplicateEvent { run: got, seq }) => {
            assert_eq!(got, run);
            assert_eq!(seq, EventSeq::new(1));
        }
        Err(other) => return Err(format!("expected DuplicateEvent, got {other:?}")),
        Ok(()) => return Err("expected duplicate with different attempt to be rejected".to_string()),
    }
    let after = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?
        .len();
    assert_eq!(after, before);
    Ok(())
}

#[test]
fn given_same_sequence_different_run_when_appended_then_cross_run_sequences_are_independent()
-> Result<(), String> {
    // Given
    let (_dir, journal) = temp_journal()?;
    let run_a = RunId::new(13);
    let run_b = RunId::new(14);

    // When
    let event_a = append_run_accepted(&journal, run_a, 0)?;
    let event_b = append_run_accepted(&journal, run_b, 0)?;

    // Then
    let events_a = journal
        .events_for_run(run_a)
        .map_err(|error| error.to_string())?;
    let events_b = journal
        .events_for_run(run_b)
        .map_err(|error| error.to_string())?;
    assert_eq!(events_a, vec![event_a]);
    assert_eq!(events_b, vec![event_b]);
    Ok(())
}

#[test]
fn given_same_run_different_sequence_when_appended_then_order_is_preserved()
-> Result<(), String> {
    // Given
    let (_dir, journal) = temp_journal()?;
    let run = RunId::new(15);

    // When
    let event_0 = append_run_accepted(&journal, run, 0)?;
    let event_1 = append_run_accepted(&journal, run, 1)?;

    // Then
    let events = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(events, vec![event_0, event_1]);
    Ok(())
}

#[test]
fn given_duplicate_lifecycle_command_when_retried_then_lifecycle_duplicate_request_and_count_unchanged()
-> Result<(), String> {
    // Given
    let (_dir, journal) = temp_journal()?;
    let run = RunId::new(16);
    append_run_accepted(&journal, run, 0)?;
    journal
        .append_strict(&JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
        })
        .map_err(|error| error.to_string())?;

    // When
    let first = lifecycle::retry(run, &journal);
    let second = lifecycle::retry(run, &journal);

    // Then
    assert!(matches!(first, Ok(())));
    match second {
        Err(CoreError::LifecycleDuplicateRequest { code, command, .. }) => {
            assert_eq!(code, CoreError::LIFECYCLE_DUPLICATE_REQUEST_CODE);
            assert_eq!(command, Some("retry"));
        }
        Err(other) => return Err(format!("expected LifecycleDuplicateRequest, got {other:?}")),
        Ok(()) => return Err("expected duplicate retry to fail".to_string()),
    }
    Ok(())
}

#[test]
fn given_completed_run_when_lifecycle_retry_uses_stale_key_then_lifecycle_stale_request()
-> Result<(), String> {
    // Given
    let (_dir, journal) = temp_journal()?;
    let run = RunId::new(17);
    append_run_accepted(&journal, run, 0)?;
    journal
        .append_strict(&JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(1),
            result: SlotIdx::new(0),
            attempt: 1,
        })
        .map_err(|error| error.to_string())?;

    // When
    let result = lifecycle::retry(run, &journal);

    // Then
    match result {
        Err(CoreError::LifecycleStaleRequest { code, command, .. }) => {
            assert_eq!(code, CoreError::LIFECYCLE_STALE_REQUEST_CODE);
            assert_eq!(command, Some("retry"));
        }
        Err(other) => return Err(format!("expected LifecycleStaleRequest, got {other:?}")),
        Ok(()) => return Err("expected stale retry to fail".to_string()),
    }
    Ok(())
}

#[test]
fn given_same_sequence_in_different_runs_when_appended_then_cross_scope_journals_are_isolated()
-> Result<(), String> {
    // Given
    let (_dir, journal) = temp_journal()?;
    let run_a = RunId::new(18);
    let run_b = RunId::new(19);

    // When
    let event_a = append_run_accepted(&journal, run_a, 0)?;
    let event_b = append_run_accepted(&journal, run_b, 0)?;

    // Then
    let events_a = journal
        .events_for_run(run_a)
        .map_err(|error| error.to_string())?;
    let events_b = journal
        .events_for_run(run_b)
        .map_err(|error| error.to_string())?;
    assert_eq!(events_a, vec![event_a]);
    assert_eq!(events_b, vec![event_b]);
    Ok(())
}

#[test]
fn given_distinct_run_scopes_when_lifecycle_retry_runs_then_each_scope_appends_its_own_retry()
-> Result<(), String> {
    // Given
    let (_dir, journal) = temp_journal()?;
    let run_a = RunId::new(20);
    let run_b = RunId::new(21);
    append_run_accepted(&journal, run_a, 0)?;
    append_run_accepted(&journal, run_b, 0)?;
    journal
        .append_strict(&JournalEvent::RunFailedEvent {
            run: run_a,
            seq: EventSeq::new(1),
            attempt: 1,
        })
        .map_err(|error| error.to_string())?;
    journal
        .append_strict(&JournalEvent::RunFailedEvent {
            run: run_b,
            seq: EventSeq::new(1),
            attempt: 1,
        })
        .map_err(|error| error.to_string())?;

    // When
    lifecycle::retry(run_a, &journal).map_err(|error| error.to_string())?;
    lifecycle::retry(run_b, &journal).map_err(|error| error.to_string())?;

    // Then
    let events_a = journal
        .events_for_run(run_a)
        .map_err(|error| error.to_string())?;
    let events_b = journal
        .events_for_run(run_b)
        .map_err(|error| error.to_string())?;
    assert_eq!(events_a.len(), 3);
    assert_eq!(events_b.len(), 3);
    assert_eq!(count_retried(&events_a), 1);
    assert_eq!(count_retried(&events_b), 1);
    Ok(())
}

#[test]
fn given_retry_required_policy_when_same_key_dispatched_twice_then_second_dispatch_is_denied()
-> Result<(), String> {
    // Given
    let mut tracker = vb_runtime::idempotency::IdempotencyTracker::with_default_capacity();

    // When
    let first = tracker.track_for_policy(Idempotency::AtLeastOnceExternal, 9010);
    let second = tracker.track_for_policy(Idempotency::AtLeastOnceExternal, 9010);

    // Then
    assert_eq!(first, true);
    assert_eq!(second, false);
    assert_eq!(
        tracker.is_completed_for_policy(Idempotency::AtLeastOnceExternal, 9010),
        true
    );
    Ok(())
}

fn count_scheduled(events: &[JournalEvent]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, JournalEvent::ActionScheduled { .. }))
        .count()
}

fn count_completed(events: &[JournalEvent]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, JournalEvent::ActionCompletedEvent { .. }))
        .count()
}

fn count_retried(events: &[JournalEvent]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, JournalEvent::RunRetried { .. }))
        .count()
}
