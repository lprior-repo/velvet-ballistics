//! Public behavior tests for bead vb-ko29.5 idempotency permutations.

use tempfile::TempDir;
use vb_cli::lifecycle;
use vb_core::action::{ActionTicket, Idempotency};
use vb_core::errors::CoreError;
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_runtime::admission::{AcceptedArtifactStore, AdmissionError, ArtifactEnvelopeError};
use vb_storage::{
    ActionReplayTracker, EventSeq, FjallJournal, JournalError, JournalEvent, replay_journal,
};

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

fn valid_artifact(artifact_digest: WorkflowDigest) -> vb_storage::AcceptedArtifact {
    vb_storage::AcceptedArtifact {
        digest: artifact_digest,
        source_digest: artifact_digest,
        policy_digest: artifact_digest,
        ir: Vec::new(),
        verification: vb_storage::VerificationProof {
            digest: artifact_digest,
            gate_count: vb_runtime::admission::REQUIRED_GATE_COUNT,
            durable: true,
            bounded_claimed: true,
            taint_safe_claimed: true,
            retry_safe_claimed: true,
            idempotency_verified_claimed: true,
            replayable_claimed: true,
            idempotency_keyed: Box::new([]),
            idempotency_attested: Box::new([]),
            warnings: Vec::new(),
        },
        accepted_at_seq: EventSeq::new(1),
        required_capabilities: Box::new([]),
    }
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
fn given_divergent_artifact_digest_when_strict_admission_runs_then_digest_mismatch_is_exact()
-> Result<(), String> {
    // Given
    let requested = digest(0x21);
    let stored = digest(0x22);
    let store = FixedArtifactStore {
        artifact: valid_artifact(stored),
    };

    // When
    let result = vb_runtime::admission::admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(12),
        requested,
        vb_core::CapabilitySet::empty(),
    );

    // Then
    assert_eq!(
        result,
        Err(AdmissionError::ArtifactDigestMismatch {
            requested,
            found: stored,
        })
    );
    Ok(())
}

#[test]
fn given_conflicting_certificate_proof_digest_when_strict_admission_runs_then_digest_mismatch_is_exact()
-> Result<(), String> {
    // Given
    let requested = digest(0x31);
    let proof_digest = digest(0x32);
    let mut artifact = valid_artifact(requested);
    artifact.verification.digest = proof_digest;
    let store = FixedArtifactStore { artifact };

    // When
    let result = vb_runtime::admission::admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(31),
        requested,
        vb_core::CapabilitySet::empty(),
    );

    // Then
    assert_eq!(
        result,
        Err(AdmissionError::ArtifactDigestMismatch {
            requested,
            found: proof_digest,
        })
    );
    Ok(())
}

#[test]
fn given_stale_certificate_floor_when_strict_admission_runs_then_stale_error_and_journal_unchanged()
-> Result<(), String> {
    // Given
    let (_dir, journal) = temp_journal()?;
    let requested = digest(0x41);
    let mut artifact = valid_artifact(requested);
    artifact.accepted_at_seq = EventSeq::new(4);
    let store = FixedArtifactStore { artifact };
    let run = RunId::new(41);
    append_run_accepted(&journal, run, 0)?;
    let before = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?
        .len();

    // When
    let result = vb_runtime::admission::admit_artifact_run_with_certificate_floor(
        &store,
        RuntimePolicy::Strict,
        run,
        requested,
        vb_core::CapabilitySet::empty(),
        EventSeq::new(5),
    );

    // Then
    assert_eq!(
        result,
        Err(AdmissionError::ArtifactCertificateStale {
            digest: requested,
            accepted_at_seq: EventSeq::new(4),
            required_at_least: EventSeq::new(5),
        })
    );
    let error = result.err().ok_or("expected stale certificate error")?;
    assert_eq!(
        error.to_string(),
        format!(
            "admission rejected: artifact certificate stale for digest {requested:?}: accepted_at_seq {:?} < required_at_least {:?}",
            EventSeq::new(4),
            EventSeq::new(5)
        )
    );
    let after = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?
        .len();
    assert_eq!(after, before);
    Ok(())
}

#[test]
fn given_wrong_digest_and_stale_floor_when_strict_admission_runs_then_digest_mismatch_wins_and_journal_unchanged()
-> Result<(), String> {
    // Given
    let (_dir, journal) = temp_journal()?;
    let requested = digest(0x42);
    let stored = digest(0x43);
    let mut artifact = valid_artifact(stored);
    artifact.accepted_at_seq = EventSeq::new(4);
    let store = FixedArtifactStore { artifact };
    let run = RunId::new(42);
    append_run_accepted(&journal, run, 0)?;
    let before = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?
        .len();

    // When
    let result = vb_runtime::admission::admit_artifact_run_with_certificate_floor(
        &store,
        RuntimePolicy::Strict,
        run,
        requested,
        vb_core::CapabilitySet::empty(),
        EventSeq::new(5),
    );

    // Then
    assert_eq!(
        result,
        Err(AdmissionError::ArtifactDigestMismatch {
            requested,
            found: stored,
        })
    );
    let after = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?
        .len();
    assert_eq!(after, before);
    Ok(())
}

#[test]
fn given_completed_action_before_restart_when_replayed_then_no_redispatch_and_event_count_stable()
-> Result<(), String> {
    // Given
    let (_dir, journal) = temp_journal()?;
    let run = RunId::new(13);
    append_run_accepted(&journal, run, 0)?;
    let scheduled = JournalEvent::ActionScheduled {
        run,
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        action: ActionId::new(9),
        attempt: 1,
    };
    let completed = JournalEvent::ActionCompletedEvent {
        run,
        seq: EventSeq::new(2),
        step: StepIdx::new(0),
        action: ActionId::new(9),
        attempt: 1,
    };
    journal
        .append_strict(&scheduled)
        .map_err(|error| error.to_string())?;
    journal
        .append_strict(&completed)
        .map_err(|error| error.to_string())?;

    // When
    let mut replay_tracker = ActionReplayTracker::new();
    let replayed = replay_journal(&journal, run, &mut replay_tracker, &[], &[])
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(replayed.len(), 3);
    assert_eq!(
        replay_tracker.is_resolved(ActionId::new(9), StepIdx::new(0)),
        true
    );
    assert_eq!(count_scheduled(&replayed), 1);
    Ok(())
}

#[test]
fn given_evicted_runtime_key_when_durable_journal_replayed_then_recovery_resolves_action()
-> Result<(), String> {
    // Given
    let (_dir, journal) = temp_journal()?;
    let run = RunId::new(14);
    append_run_accepted(&journal, run, 0)?;
    let completed = JournalEvent::ActionCompletedEvent {
        run,
        seq: EventSeq::new(1),
        step: StepIdx::new(1),
        action: ActionId::new(4),
        attempt: 1,
    };
    journal
        .append_strict(&completed)
        .map_err(|error| error.to_string())?;
    let mut volatile = vb_runtime::idempotency::IdempotencyTracker::with_capacity(1);
    assert_eq!(volatile.mark_completed(&ticket(14, 1, 400)), Ok(()));
    assert_eq!(volatile.mark_completed(&ticket(14, 2, 401)), Ok(()));
    assert_eq!(volatile.is_completed(&ticket(14, 1, 400)), false);

    // When
    let mut durable_tracker = ActionReplayTracker::new();
    let events = replay_journal(&journal, run, &mut durable_tracker, &[], &[])
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(events.len(), 2);
    assert_eq!(count_completed(&events), 1);
    assert_eq!(events.contains(&completed), true);
    assert_eq!(
        durable_tracker.is_resolved(ActionId::new(4), StepIdx::new(1)),
        true
    );
    Ok(())
}

#[test]
fn given_failed_run_retried_twice_when_cli_retry_collides_then_duplicate_error_and_no_append()
-> Result<(), String> {
    // Given
    let (_dir, journal) = temp_journal()?;
    let run = RunId::new(15);
    append_run_accepted(&journal, run, 0)?;
    journal
        .append_strict(&JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
        })
        .map_err(|error| error.to_string())?;
    lifecycle::retry(run, &journal).map_err(|error| error.to_string())?;
    let before = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?
        .len();

    // When
    let result = lifecycle::retry(run, &journal);

    // Then
    match result {
        Err(CoreError::LifecycleDuplicateRequest { code, command, .. }) => {
            assert_eq!(code, CoreError::LIFECYCLE_DUPLICATE_REQUEST_CODE);
            assert_eq!(command, Some("retry"));
        }
        Err(other) => return Err(format!("expected LifecycleDuplicateRequest, got {other:?}")),
        Ok(()) => return Err("expected duplicate retry to fail".to_string()),
    }
    let after = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?
        .len();
    assert_eq!(after, before);
    Ok(())
}

#[test]
fn given_completed_run_when_cli_retry_uses_stale_key_then_stale_error_and_no_append()
-> Result<(), String> {
    // Given
    let (_dir, journal) = temp_journal()?;
    let run = RunId::new(16);
    append_run_accepted(&journal, run, 0)?;
    journal
        .append_strict(&JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(1),
            result: SlotIdx::new(0),
            attempt: 1,
        })
        .map_err(|error| error.to_string())?;
    let before = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?
        .len();

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
    let after = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?
        .len();
    assert_eq!(after, before);
    Ok(())
}

#[test]
fn given_same_sequence_in_different_runs_when_appended_then_cross_scope_journals_are_isolated()
-> Result<(), String> {
    // Given
    let (_dir, journal) = temp_journal()?;
    let run_a = RunId::new(17);
    let run_b = RunId::new(18);

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
    let run_a = RunId::new(19);
    let run_b = RunId::new(20);
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
