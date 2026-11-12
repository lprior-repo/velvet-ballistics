//! Public behavior tests for bead vb-ko29.5 idempotency permutations.

mod vb_ko29_5_fixture;

use tempfile::TempDir;
use vb_core::action::Idempotency;
use vb_core::errors::CoreError;
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_runtime::admission::{
    AcceptedArtifactStore, AdmissionError, ArtifactEnvelopeError, admit_artifact_run,
    admit_artifact_run_with_certificate_floor,
};
use vb_storage::admission::accepted_artifact_digest;
use vb_storage::admission::prepare_compiled_artifact;
use vb_storage::{EventSeq, FjallJournal, JournalError, JournalEvent};
use velvet_ballistics::lifecycle;

fn compile_fixture() -> Result<vb_core::CompiledWorkflow, String> {
    let source = "version: velvet-ballistics/v1\nname: idempotency_fixture\nwhen: { manual: {} }\nsteps:\n  - id: make\n    set: { output: answer, value: \"42\" }\n  - id: done\n    finish: { result: answer }\n";
    vb_compile::compile_workflow(source.as_bytes()).map_err(|error| error.to_string())
}

fn accepted_artifact_fixture() -> Result<vb_storage::AcceptedArtifact, String> {
    let workflow = compile_fixture()?;
    prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])
        .map_err(|error| error.to_string())
}

struct FixedAcceptedStore {
    artifact: vb_storage::AcceptedArtifact,
}

impl AcceptedArtifactStore for FixedAcceptedStore {
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
        Ok(()) => {
            return Err("expected duplicate with different attempt to be rejected".to_string());
        }
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
    let artifact = accepted_artifact_fixture()?;
    let stored = artifact.digest;
    let requested = digest(0x21);
    let store = FixedAcceptedStore { artifact };

    // When
    let result = admit_artifact_run(
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
    let artifact = accepted_artifact_fixture()?;
    let requested = artifact.digest;
    let proof_digest = digest(0x32);
    let mut tampered = artifact.clone();
    tampered.verification.digest = proof_digest;
    let store = FixedAcceptedStore { artifact: tampered };

    // When
    let result = admit_artifact_run(
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
    let artifact = accepted_artifact_fixture()?;
    let mut tampered = artifact.clone();
    tampered.accepted_at_seq = EventSeq::new(4);
    let tampered_digest = accepted_artifact_digest(&tampered).map_err(|error| error.to_string())?;
    tampered.digest = tampered_digest;
    tampered.verification.digest = tampered_digest;
    let store = FixedAcceptedStore { artifact: tampered };
    let run = RunId::new(41);
    append_run_accepted(&journal, run, 0)?;
    let before = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;

    // When
    let result = admit_artifact_run_with_certificate_floor(
        &store,
        RuntimePolicy::Strict,
        run,
        tampered_digest,
        vb_core::CapabilitySet::empty(),
        EventSeq::new(5),
    );

    // Then
    assert_eq!(
        result,
        Err(AdmissionError::ArtifactCertificateStale {
            digest: tampered_digest,
            accepted_at_seq: EventSeq::new(4),
            required_at_least: EventSeq::new(5),
        })
    );
    let error = result.err().ok_or("expected stale certificate error")?;
    let display = error.to_string();
    let expected_display = format!(
        "admission rejected: artifact certificate stale for digest {:?}: accepted_at_seq {:?} < required_at_least {:?}",
        tampered_digest,
        EventSeq::new(4),
        EventSeq::new(5)
    );
    assert_eq!(
        display, expected_display,
        "stale certificate Display must match exact production format"
    );
    let after = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(after, before);
    Ok(())
}

#[test]
fn given_wrong_digest_and_stale_floor_when_strict_admission_runs_then_digest_mismatch_wins_and_journal_unchanged()
-> Result<(), String> {
    // Given
    let (_dir, journal) = temp_journal()?;
    let artifact = accepted_artifact_fixture()?;
    let mut tampered = artifact.clone();
    tampered.accepted_at_seq = EventSeq::new(4);
    let stored = accepted_artifact_digest(&tampered).map_err(|error| error.to_string())?;
    tampered.digest = stored;
    tampered.verification.digest = stored;
    let requested = digest(0x42);
    let store = FixedAcceptedStore { artifact: tampered };
    let run = RunId::new(42);
    append_run_accepted(&journal, run, 0)?;
    let before = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;

    // When
    let result = admit_artifact_run_with_certificate_floor(
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
        .map_err(|error| error.to_string())?;
    assert_eq!(after, before);
    Ok(())
}

#[test]
fn given_completed_action_before_restart_when_replayed_then_no_redispatch_and_event_count_stable()
-> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let run = RunId::new(13);
    let (ticket, abi_digest) = vb_ko29_5_fixture::produce_completed_run_with_abi(dir.path(), run)?;
    let journal = FjallJournal::open(dir.path(), None).map_err(|e| e.to_string())?;

    let before = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;

    let mut tracker = vb_storage::ActionReplayTracker::default();
    let expected = vec![(ticket.action, abi_digest)];
    let replayed = vb_storage::replay_journal(&journal, run, &mut tracker, &expected, &[])
        .map_err(|error| error.to_string())?;

    assert!(tracker.is_resolved(ticket.action, ticket.step));
    assert_eq!(replayed, before);

    // Verify scheduled ABI equals completed ABI equals expected digest and is nonzero.
    let scheduled_abi = before
        .iter()
        .find_map(|e| {
            if let JournalEvent::ActionScheduledTicket {
                action_abi_digest, ..
            } = e
            {
                Some(*action_abi_digest)
            } else {
                None
            }
        })
        .expect("scheduled ticket event must exist");
    let completed_abi = before
        .iter()
        .find_map(|e| {
            if let JournalEvent::ActionCompletedEnvelope {
                action_abi_digest, ..
            } = e
            {
                Some(*action_abi_digest)
            } else {
                None
            }
        })
        .expect("completion envelope event must exist");
    assert_eq!(
        scheduled_abi, abi_digest,
        "scheduled ABI must equal expected"
    );
    assert_eq!(
        completed_abi, abi_digest,
        "completed ABI must equal expected"
    );
    assert_eq!(
        scheduled_abi, completed_abi,
        "scheduled and completed ABI must match"
    );
    assert_ne!(
        scheduled_abi.as_bytes(),
        [0u8; 32],
        "scheduled ABI must be nonzero"
    );
    assert_ne!(
        completed_abi.as_bytes(),
        [0u8; 32],
        "completed ABI must be nonzero"
    );
    assert_eq!(
        before
            .iter()
            .filter(|e| matches!(e, JournalEvent::ActionScheduledTicket { .. }))
            .count(),
        1,
        "exactly one scheduled ticket event"
    );
    assert_eq!(
        before
            .iter()
            .filter(|e| matches!(e, JournalEvent::ActionCompletedEnvelope { .. }))
            .count(),
        1,
        "exactly one completion envelope event"
    );
    assert!(
        before
            .iter()
            .any(|e| matches!(e, JournalEvent::RunFinished { .. })),
        "run must be marked as finished"
    );

    let after = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(after, before, "journal unchanged by replay");
    Ok(())
}

#[test]
fn given_evicted_runtime_key_when_durable_journal_replayed_then_recovery_resolves_action()
-> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let run1 = RunId::new(14);
    let run2 = RunId::new(15);
    let (ticket1, abi_digest1) =
        vb_ko29_5_fixture::produce_completed_run_with_abi(dir.path(), run1)?;
    let (ticket2, _abi_digest2) =
        vb_ko29_5_fixture::produce_completed_run_with_abi(dir.path(), run2)?;

    let mut volatile = vb_runtime::idempotency::IdempotencyTracker::with_capacity(1);
    volatile
        .mark_completed(&ticket1)
        .map_err(|error| error.to_string())?;
    volatile
        .mark_completed(&ticket2)
        .map_err(|error| error.to_string())?;
    assert!(
        !volatile.is_completed(&ticket1),
        "volatile tracker must have evicted the first key"
    );

    let journal = FjallJournal::open(dir.path(), None).map_err(|e| e.to_string())?;
    let before = journal
        .events_for_run(run1)
        .map_err(|error| error.to_string())?;

    let mut tracker = vb_storage::ActionReplayTracker::default();
    let expected = vec![(ticket1.action, abi_digest1)];
    let replayed = vb_storage::replay_journal(&journal, run1, &mut tracker, &expected, &[])
        .map_err(|error| error.to_string())?;

    assert!(tracker.is_resolved(ticket1.action, ticket1.step));
    assert_eq!(replayed, before);
    assert!(
        replayed
            .iter()
            .any(|e| matches!(e, JournalEvent::ActionScheduledTicket { .. })),
        "replayed journal contains scheduled ticket"
    );
    assert!(
        replayed
            .iter()
            .any(|e| matches!(e, JournalEvent::ActionCompletedEnvelope { .. })),
        "replayed journal contains completion envelope"
    );

    let after = journal
        .events_for_run(run1)
        .map_err(|error| error.to_string())?;
    assert_eq!(after, before, "journal unchanged by replay");
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
fn given_same_run_different_sequence_when_appended_then_order_is_preserved() -> Result<(), String> {
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
    let before = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
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
    let after = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        after, before,
        "duplicate retry must not append new journal events"
    );
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
    let before = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
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
        .map_err(|error| error.to_string())?;
    assert_eq!(
        after, before,
        "stale retry must not append new journal events"
    );
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

fn count_retried(events: &[JournalEvent]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, JournalEvent::RunRetried { .. }))
        .count()
}

#[test]
fn given_no_schedule_event_when_expected_abi_then_replay_journal_rejects_with_abi_mismatch()
-> Result<(), String> {
    // Negative regression: when expected_action_abi_digests is nonempty but
    // the journal contains no ActionScheduled or ActionScheduledTicket event,
    // replay_journal must reject with ActionAbiMismatch.
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let journal = FjallJournal::open(dir.path(), None).map_err(|e| e.to_string())?;
    let run = RunId::new(999);

    // Append only a RunAdmission event — no schedule event.
    let admission_event = JournalEvent::RunAdmission {
        run,
        seq: EventSeq::new(0),
        artifact_digest: digest(0x11),
        input_digest: vb_storage::RunInputBinding::LegacyUnbound,
        policy_pin: vb_storage::RunPolicyPin::Unverified,
        granted_capabilities: vb_core::CapabilitySet::empty(),
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    };
    vb_storage::append_journal_event(&journal, &admission_event).map_err(|e| e.to_string())?;

    let mut tracker = vb_storage::ActionReplayTracker::default();
    let expected = vec![(ActionId::new(1), digest(0x33))];
    let result = vb_storage::replay_journal(&journal, run, &mut tracker, &expected, &[]);

    match result {
        Err(vb_storage::recovery::RecoveryError::ActionAbiMismatch { action_id }) => {
            assert_eq!(
                action_id,
                ActionId::new(1),
                "must report the expected action"
            );
            Ok(())
        }
        other => Err(format!("expected ActionAbiMismatch, got {other:?}")),
    }
}

#[test]
fn given_real_producer_wrong_expected_abi_when_replayed_then_rejected_with_abi_mismatch()
-> Result<(), String> {
    // Real producer schedule guard: if the expected ABI digest differs from the
    // actual scheduled action's contract ABI, replay_journal must reject with
    // ActionAbiMismatch, not accept a mismatched completion.
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let run = RunId::new(2001);
    let (ticket, abi_digest) = vb_ko29_5_fixture::produce_completed_run_with_abi(dir.path(), run)?;
    let journal = FjallJournal::open(dir.path(), None).map_err(|e| e.to_string())?;

    let before = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;

    // Expect a different ABI than what was actually scheduled.
    let wrong_abi = digest(0xAA);
    assert_ne!(wrong_abi, abi_digest, "wrong ABI must differ from actual");
    let expected = vec![(ticket.action, wrong_abi)];

    let mut tracker = vb_storage::ActionReplayTracker::default();
    let result = vb_storage::replay_journal(&journal, run, &mut tracker, &expected, &[]);

    match result {
        Err(vb_storage::recovery::RecoveryError::ActionAbiMismatch { action_id }) => {
            assert_eq!(action_id, ticket.action, "must report the expected action");
        }
        other => return Err(format!("expected ActionAbiMismatch, got {other:?}")),
    }

    // Journal must be unchanged.
    let after = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(after, before, "journal unchanged by rejected replay");
    Ok(())
}

#[test]
fn given_real_producer_zero_expected_abi_when_replayed_then_rejected_with_abi_mismatch()
-> Result<(), String> {
    // Real producer schedule guard: a zero expected ABI digest must be rejected
    // because real actions always have a nonzero contract ABI. This prevents
    // callers from bypassing the schedule guard by passing a null digest.
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let run = RunId::new(2002);
    let (ticket, abi_digest) = vb_ko29_5_fixture::produce_completed_run_with_abi(dir.path(), run)?;
    let journal = FjallJournal::open(dir.path(), None).map_err(|e| e.to_string())?;

    let before = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;

    // Expect a zero ABI digest — should be rejected because real scheduled
    // actions have nonzero contract ABIs.
    let zero_abi = WorkflowDigest::from_bytes([0u8; 32]);
    assert_ne!(zero_abi, abi_digest, "zero ABI must differ from actual");
    let expected = vec![(ticket.action, zero_abi)];

    let mut tracker = vb_storage::ActionReplayTracker::default();
    let result = vb_storage::replay_journal(&journal, run, &mut tracker, &expected, &[]);

    match result {
        Err(vb_storage::recovery::RecoveryError::ActionAbiMismatch { action_id }) => {
            assert_eq!(action_id, ticket.action, "must report the expected action");
        }
        other => return Err(format!("expected ActionAbiMismatch, got {other:?}")),
    }

    // Journal must be unchanged.
    let after = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(after, before, "journal unchanged by rejected replay");
    Ok(())
}
