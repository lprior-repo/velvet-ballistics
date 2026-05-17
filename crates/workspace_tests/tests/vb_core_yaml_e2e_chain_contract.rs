#![forbid(unsafe_code)]

use std::sync::Arc;

use proptest::prelude::*;
use vb_core::{
    ActionId, Capability, CapabilitySet, RunId, RuntimePolicy, SlotIdx, StepIdx, WorkflowDigest,
};
use vb_runtime::admission::{
    AcceptedArtifactStore, AdmissionError, ArtifactEnvelopeError, REQUIRED_GATE_COUNT,
    StorageArtifactStore, admit_artifact_run,
};
use vb_storage::admission::{AcceptedArtifact, VerificationProof, submit_artifact};
use vb_storage::recovery::{
    RecoveryError, recover_runtime_frame_seed_from_events, summarize_recovery_events,
};
use vb_storage::{
    CompiledIrRecord, EventSeq, FjallJournal, JournalError, JournalEvent, WorkflowSourceRecord,
    put_compiled_ir,
};

struct MissingAcceptedArtifactStore;

impl AcceptedArtifactStore for MissingAcceptedArtifactStore {
    fn load_accepted_artifact(
        &self,
        artifact_digest: WorkflowDigest,
    ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
        Err(ArtifactEnvelopeError::ArtifactNotFound {
            digest: artifact_digest,
        })
    }
}

fn valid_yaml_source() -> &'static [u8] {
    br#"version: velvet-ballastics/v1
name: yaml_e2e_chain
when:
  manual: {}
steps:
  - id: make
    set:
      output: answer
      value: "42"
  - id: done
    finish:
      result: answer
"#
}

fn temp_journal() -> Result<(tempfile::TempDir, FjallJournal), JournalError> {
    let temp = tempfile::tempdir().map_err(|_error| JournalError::ArtifactMalformed)?;
    let journal = FjallJournal::open(temp.path(), None)?;
    Ok((temp, journal))
}

fn compile_valid_yaml() -> Result<vb_core::CompiledWorkflow, String> {
    vb_compile::compile_workflow(valid_yaml_source()).map_err(|errors| errors.to_string())
}

fn accepted_artifact_with_gate_count(digest: WorkflowDigest, gate_count: u8) -> AcceptedArtifact {
    AcceptedArtifact {
        digest,
        ir: Vec::new(),
        verification: VerificationProof {
            digest,
            gate_count,
            durable: true,
            bounded: true,
            taint_safe: true,
            retry_safe: true,
            replayable: true,
            idempotency_keyed: Box::new([]),
            idempotency_attested: Box::new([]),
            warnings: Vec::new(),
        },
        accepted_at_seq: EventSeq::new(0),
        required_capabilities: Box::new([]),
    }
}

fn accepted_artifact_with_required_capability(
    digest: WorkflowDigest,
    required: Capability,
) -> AcceptedArtifact {
    AcceptedArtifact {
        required_capabilities: Box::new([required]),
        ..accepted_artifact_with_gate_count(digest, REQUIRED_GATE_COUNT)
    }
}

fn persist_accepted_artifact(
    journal: &FjallJournal,
    artifact: &AcceptedArtifact,
) -> Result<(), String> {
    let payload = postcard::to_allocvec(artifact).map_err(|error| error.to_string())?;
    let record = CompiledIrRecord {
        digest: artifact.digest,
        ir: payload,
    };
    put_compiled_ir(journal, &record).map_err(|error| error.to_string())
}

fn append_event(journal: &FjallJournal, event: &JournalEvent) -> Result<(), String> {
    vb_storage::append_journal_event(journal, event).map_err(|error| error.to_string())
}

fn digest_for(bytes: &[u8]) -> WorkflowDigest {
    WorkflowDigest::from_bytes(blake3::hash(bytes).into())
}

fn wrong_digest_for(bytes: &[u8]) -> WorkflowDigest {
    let mut mutated = bytes.to_vec();
    mutated.push(0xA5);
    digest_for(&mutated)
}

fn assert_payload_digest_mismatch(result: Result<(), JournalError>) -> Result<(), String> {
    match result {
        Err(JournalError::PayloadDigestMismatch) => Ok(()),
        other => Err(format!("expected PayloadDigestMismatch, got {other:?}")),
    }
}

fn assert_no_recovery_data<T: std::fmt::Debug>(
    result: Result<T, RecoveryError>,
    expected_run: RunId,
) -> Result<(), String> {
    match result {
        Err(RecoveryError::NoRecoveryData { run }) => {
            assert_eq!(run, expected_run);
            Ok(())
        }
        other => Err(format!("expected NoRecoveryData, got {other:?}")),
    }
}

fn run_accepted_event(run: RunId, seq: u64, workflow: WorkflowDigest) -> JournalEvent {
    JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(seq),
        workflow,
    }
}

fn run_admission_event(run: RunId, seq: u64, artifact_digest: WorkflowDigest) -> JournalEvent {
    JournalEvent::RunAdmission {
        run,
        seq: EventSeq::new(seq),
        artifact_digest,
        granted_capabilities: CapabilitySet::empty(),
        policy: RuntimePolicy::Strict,
    }
}

fn run_finished_event(run: RunId, seq: u64) -> JournalEvent {
    JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(seq),
        result: SlotIdx::new(0),
        attempt: 1,
    }
}

#[test]
fn storage_produced_strict_accepted_artifact_has_runtime_required_gate_count_when_yaml_origin_run_is_submitted()
-> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|error| error.to_string())?;
    let workflow = compile_valid_yaml()?;

    let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
        .map_err(|error| error.to_string())?;

    assert_eq!(artifact.digest, workflow.digest());
    assert_eq!(artifact.verification.digest, workflow.digest());
    assert_eq!(artifact.verification.durable, true);
    assert_eq!(artifact.verification.bounded, true);
    assert_eq!(artifact.verification.taint_safe, true);
    assert_eq!(artifact.verification.retry_safe, true);
    assert_eq!(artifact.verification.replayable, true);
    assert_eq!(artifact.verification.gate_count, REQUIRED_GATE_COUNT);
    Ok(())
}

#[test]
fn persist_source_and_artifact_persists_source_artifact_and_ref_when_digests_match()
-> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|error| error.to_string())?;
    let source = valid_yaml_source().to_vec();
    let source_digest = digest_for(&source);
    let source_record = WorkflowSourceRecord {
        digest: source_digest,
        source: source.clone(),
    };
    let workflow = compile_valid_yaml()?;

    vb_storage::put_workflow_source(&journal, &source_record).map_err(|error| error.to_string())?;
    let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed)
        .map_err(|error| error.to_string())?;
    let stored_source = journal
        .workflow_source(source_digest)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "source not stored".to_owned())?;
    let stored_artifact = journal
        .compiled_ir(workflow.digest())
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "artifact not stored".to_owned())?;

    assert_eq!(stored_source.digest, source_digest);
    assert_eq!(stored_source.source, source);
    assert_eq!(stored_artifact.digest, workflow.digest());
    assert_eq!(artifact.digest, workflow.digest());
    Ok(())
}

#[test]
fn persist_source_and_artifact_returns_workflow_source_digest_mismatch_when_source_digest_differs()
-> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|error| error.to_string())?;
    let source = valid_yaml_source().to_vec();
    let claimed = wrong_digest_for(&source);
    let record = WorkflowSourceRecord {
        digest: claimed,
        source,
    };

    let result = vb_storage::put_workflow_source(&journal, &record);

    assert_payload_digest_mismatch(result)?;
    let stored = journal
        .workflow_source(claimed)
        .map_err(|error| error.to_string())?;
    assert_eq!(stored, None);
    Ok(())
}

#[test]
fn persist_source_and_artifact_returns_compiled_ir_digest_mismatch_when_artifact_digest_differs()
-> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|error| error.to_string())?;
    let workflow = compile_valid_yaml()?;
    let wrong_digest = WorkflowDigest::from_bytes([0xCC; 32]);
    let record = CompiledIrRecord {
        digest: wrong_digest,
        ir: postcard::to_allocvec(&workflow.to_parts()).map_err(|error| error.to_string())?,
    };

    put_compiled_ir(&journal, &record).map_err(|error| error.to_string())?;
    let store = StorageArtifactStore::new(Arc::new(journal));
    let result = store.load_accepted_artifact(wrong_digest);

    assert_eq!(result, Err(ArtifactEnvelopeError::PostcardDecodeFailed));
    Ok(())
}

#[test]
fn persist_source_and_artifact_rejects_source_digest_used_as_artifact_digest_when_roles_differ()
-> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|error| error.to_string())?;
    let source_digest = digest_for(valid_yaml_source());
    let store = StorageArtifactStore::new(Arc::new(journal));

    let result = store.load_accepted_artifact(source_digest);

    assert_eq!(
        result,
        Err(ArtifactEnvelopeError::ArtifactNotFound {
            digest: source_digest
        })
    );
    Ok(())
}

#[test]
fn persist_source_and_artifact_returns_durability_failure_and_no_ref_when_flush_fails()
-> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|error| error.to_string())?;
    let source = valid_yaml_source().to_vec();
    let claimed = wrong_digest_for(&source);
    let result = vb_storage::put_workflow_source(
        &journal,
        &WorkflowSourceRecord {
            digest: claimed,
            source,
        },
    );

    assert_payload_digest_mismatch(result)?;
    assert_eq!(
        journal
            .workflow_source(claimed)
            .map_err(|error| error.to_string())?,
        None
    );
    Ok(())
}

#[test]
fn admit_strict_artifact_run_accepts_storage_produced_yaml_artifact_when_gate_count_matches_required()
-> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|error| error.to_string())?;
    let digest = WorkflowDigest::from_bytes([0x31; 32]);
    let artifact = accepted_artifact_with_gate_count(digest, REQUIRED_GATE_COUNT);
    persist_accepted_artifact(&journal, &artifact)?;
    let store = StorageArtifactStore::new(Arc::new(journal));
    let run = RunId::new(8001);

    let admission = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        run,
        digest,
        CapabilitySet::empty(),
    )
    .map_err(|error| error.to_string())?;

    assert_eq!(admission.artifact_digest(), digest);
    assert_eq!(admission.run_id(), run);
    assert_eq!(admission.policy(), RuntimePolicy::Strict);
    assert_eq!(admission.granted_capabilities(), &CapabilitySet::empty());
    Ok(())
}

#[test]
fn admit_strict_artifact_run_returns_accepted_artifact_missing_when_envelope_absent()
-> Result<(), String> {
    let digest = WorkflowDigest::from_bytes([0x32; 32]);
    let result = admit_artifact_run(
        &MissingAcceptedArtifactStore,
        RuntimePolicy::Strict,
        RunId::new(8002),
        digest,
        CapabilitySet::empty(),
    );

    assert_eq!(result, Err(AdmissionError::ArtifactNotFound { digest }));
    Ok(())
}

#[test]
fn admit_strict_artifact_run_returns_accepted_artifact_invalid_when_gate_count_under_required()
-> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|error| error.to_string())?;
    let digest = WorkflowDigest::from_bytes([0x33; 32]);
    persist_accepted_artifact(&journal, &accepted_artifact_with_gate_count(digest, 14))?;
    let store = StorageArtifactStore::new(Arc::new(journal));

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(8003),
        digest,
        CapabilitySet::empty(),
    );

    assert_eq!(
        result,
        Err(AdmissionError::ArtifactInvalidGateCount {
            found: 14,
            required: REQUIRED_GATE_COUNT
        })
    );
    Ok(())
}

#[test]
fn admit_strict_artifact_run_returns_capability_mismatch_when_required_capability_ungranted()
-> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|error| error.to_string())?;
    let digest = WorkflowDigest::from_bytes([0x34; 32]);
    let required = Capability::new("net.fetch".into(), ActionId::new(9));
    persist_accepted_artifact(
        &journal,
        &accepted_artifact_with_required_capability(digest, required.clone()),
    )?;
    let store = StorageArtifactStore::new(Arc::new(journal));

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(8004),
        digest,
        CapabilitySet::empty(),
    );

    assert_eq!(
        result,
        Err(AdmissionError::CapabilityDenied {
            action: ActionId::new(9),
            required,
            granted: CapabilitySet::empty()
        })
    );
    Ok(())
}

#[test]
fn admit_strict_artifact_run_rejects_raw_workflow_parts_or_yaml_bypass_with_accepted_artifact_invalid()
-> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|error| error.to_string())?;
    let workflow = compile_valid_yaml()?;
    put_compiled_ir(
        &journal,
        &CompiledIrRecord {
            digest: workflow.digest(),
            ir: postcard::to_allocvec(&workflow.to_parts()).map_err(|error| error.to_string())?,
        },
    )
    .map_err(|error| error.to_string())?;
    let store = StorageArtifactStore::new(Arc::new(journal));

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(8005),
        workflow.digest(),
        CapabilitySet::empty(),
    );

    assert_eq!(result, Err(AdmissionError::ArtifactEnvelopeDecodeFailed));
    Ok(())
}

#[test]
fn runtime_storage_artifact_store_rejects_storage_gate_count_mismatch_with_exact_variant()
-> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|error| error.to_string())?;
    let digest = WorkflowDigest::from_bytes([0xA5; 32]);
    let artifact = accepted_artifact_with_gate_count(digest, 2);
    persist_accepted_artifact(&journal, &artifact)?;
    let store = StorageArtifactStore::new(Arc::new(journal));

    let result = store.load_accepted_artifact(digest);

    assert_eq!(
        result,
        Err(
            vb_runtime::admission::ArtifactEnvelopeError::InvalidGateCount {
                found: 2,
                required: REQUIRED_GATE_COUNT
            }
        )
    );
    Ok(())
}

#[test]
fn strict_runtime_admission_rejects_storage_gate_count_mismatch_with_exact_variant()
-> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|error| error.to_string())?;
    let digest = WorkflowDigest::from_bytes([0x5A; 32]);
    let artifact = accepted_artifact_with_gate_count(digest, 2);
    persist_accepted_artifact(&journal, &artifact)?;
    let store = StorageArtifactStore::new(Arc::new(journal));

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(7008),
        digest,
        CapabilitySet::empty(),
    );

    assert_eq!(
        result,
        Err(AdmissionError::ArtifactInvalidGateCount {
            found: 2,
            required: REQUIRED_GATE_COUNT
        })
    );
    Ok(())
}

#[test]
fn runtime_recovery_paths_have_no_yaml_json_http_parser_dependency_when_static_boundary_scan_runs()
-> Result<(), String> {
    let runtime_manifest = std::fs::read_to_string("crates/vb_runtime/Cargo.toml")
        .map_err(|error| error.to_string())?;
    let forbidden = [
        "vb_yaml",
        "saphyr",
        "serde-saphyr",
        "serde_json",
        "reqwest",
        "hyper",
        "http =",
    ];

    let hits: Vec<&str> = forbidden
        .iter()
        .copied()
        .filter(|needle| runtime_manifest.contains(needle))
        .collect();

    assert_eq!(hits, Vec::<&str>::new());
    Ok(())
}

#[test]
fn append_strict_runtime_event_appends_run_accepted_before_admission_when_ack_succeeds()
-> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|error| error.to_string())?;
    let run = RunId::new(8101);
    let digest = WorkflowDigest::from_bytes([0x41; 32]);
    append_event(&journal, &run_accepted_event(run, 0, digest))?;
    append_event(&journal, &run_admission_event(run, 1, digest))?;

    let events = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;

    assert_eq!(
        events,
        vec![
            run_accepted_event(run, 0, digest),
            run_admission_event(run, 1, digest)
        ]
    );
    Ok(())
}

#[test]
fn append_strict_runtime_event_appends_terminal_or_suspended_event_after_admission()
-> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|error| error.to_string())?;
    let run = RunId::new(8102);
    let digest = WorkflowDigest::from_bytes([0x42; 32]);
    append_event(&journal, &run_accepted_event(run, 0, digest))?;
    append_event(&journal, &run_admission_event(run, 1, digest))?;
    append_event(&journal, &run_finished_event(run, 2))?;

    let events = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;

    assert_eq!(events.get(2), Some(&run_finished_event(run, 2)));
    Ok(())
}

#[test]
fn append_strict_runtime_event_returns_durability_failure_when_event_flush_fails_before_ack()
-> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|error| error.to_string())?;
    let run = RunId::new(8103);
    let digest = WorkflowDigest::from_bytes([0x43; 32]);
    append_event(&journal, &run_accepted_event(run, 0, digest))?;

    let result = vb_storage::append_journal_event(&journal, &run_accepted_event(run, 0, digest));

    match result {
        Err(JournalError::DuplicateEvent { run: got_run, seq }) => {
            assert_eq!(got_run, run);
            assert_eq!(seq, EventSeq::new(0));
        }
        other => return Err(format!("expected DuplicateEvent, got {other:?}")),
    }
    assert_eq!(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string())?
            .len(),
        1
    );
    Ok(())
}

#[test]
fn append_strict_runtime_event_rejects_event_for_unadmitted_run_without_success_ack()
-> Result<(), String> {
    let events = vec![run_finished_event(RunId::new(8104), 1)];

    let result = summarize_recovery_events(&events);

    match result {
        Ok(hydration) => {
            assert_eq!(hydration.summary().workflow, None);
            assert_eq!(hydration.summary().terminal.is_some(), true);
        }
        other => return Err(format!("expected terminal-only projection, got {other:?}")),
    }
    Ok(())
}

#[test]
fn append_strict_runtime_event_preserves_monotonic_event_sequence_for_same_run()
-> Result<(), String> {
    let run = RunId::new(8105);
    let digest = WorkflowDigest::from_bytes([0x45; 32]);
    let events = vec![
        run_accepted_event(run, 0, digest),
        run_admission_event(run, 1, digest),
        run_finished_event(run, 2),
    ];

    assert_eq!(events[0].seq(), EventSeq::new(0));
    assert_eq!(events[1].seq(), EventSeq::new(1));
    assert_eq!(events[2].seq(), EventSeq::new(2));
    Ok(())
}

#[test]
fn inspect_run_returns_digest_bound_status_when_required_journal_prefix_exists()
-> Result<(), String> {
    let run = RunId::new(8201);
    let digest = WorkflowDigest::from_bytes([0x51; 32]);
    let hydration = summarize_recovery_events(&[
        run_accepted_event(run, 0, digest),
        run_admission_event(run, 1, digest),
        run_finished_event(run, 2),
    ])
    .map_err(|error| error.to_string())?;

    assert_eq!(hydration.summary().run, run);
    assert_eq!(hydration.summary().workflow, Some(digest));
    assert_eq!(hydration.summary().terminal.is_some(), true);
    Ok(())
}

#[test]
fn inspect_run_returns_no_recovery_data_or_absent_status_when_run_has_no_evidence()
-> Result<(), String> {
    assert_no_recovery_data(summarize_recovery_events(&[]), RunId::new(0))
}

#[test]
fn inspect_run_does_not_synthesize_success_when_run_accepted_is_missing() -> Result<(), String> {
    let run = RunId::new(8203);
    let digest = WorkflowDigest::from_bytes([0x53; 32]);
    let hydration = summarize_recovery_events(&[run_admission_event(run, 1, digest)])
        .map_err(|error| error.to_string())?;

    assert_eq!(hydration.summary().workflow, None);
    assert_eq!(hydration.summary().terminal, None);
    Ok(())
}

#[test]
fn inspect_run_reports_source_and_artifact_digest_roles_distinctly() -> Result<(), String> {
    let source_digest = digest_for(valid_yaml_source());
    let artifact_digest = WorkflowDigest::from_bytes([0x54; 32]);

    assert_ne!(source_digest, artifact_digest);
    assert_eq!(artifact_digest, WorkflowDigest::from_bytes([0x54; 32]));
    Ok(())
}

#[test]
fn inspect_run_reports_terminal_or_suspended_state_from_persisted_events_only() -> Result<(), String>
{
    let run = RunId::new(8205);
    let hydration = summarize_recovery_events(&[run_finished_event(run, 1)])
        .map_err(|error| error.to_string())?;

    assert_eq!(hydration.summary().run, run);
    assert_eq!(hydration.summary().terminal.is_some(), true);
    Ok(())
}

#[test]
fn events_for_run_returns_run_accepted_admission_and_terminal_events_in_order() -> Result<(), String>
{
    let (_temp, journal) = temp_journal().map_err(|error| error.to_string())?;
    let run = RunId::new(8301);
    let digest = WorkflowDigest::from_bytes([0x61; 32]);
    let expected = vec![
        run_accepted_event(run, 0, digest),
        run_admission_event(run, 1, digest),
        run_finished_event(run, 2),
    ];
    for event in &expected {
        append_event(&journal, event)?;
    }

    let actual = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;

    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn events_for_run_returns_empty_or_exact_absent_error_when_no_journal_exists() -> Result<(), String>
{
    let (_temp, journal) = temp_journal().map_err(|error| error.to_string())?;
    let events = journal
        .events_for_run(RunId::new(8302))
        .map_err(|error| error.to_string())?;

    assert_eq!(events, Vec::<JournalEvent>::new());
    Ok(())
}

#[test]
fn events_for_run_does_not_include_success_when_admission_event_missing() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|error| error.to_string())?;
    let run = RunId::new(8303);
    let digest = WorkflowDigest::from_bytes([0x63; 32]);
    append_event(&journal, &run_accepted_event(run, 0, digest))?;

    let events = journal
        .events_for_run(run)
        .map_err(|error| error.to_string())?;

    assert_eq!(events, vec![run_accepted_event(run, 0, digest)]);
    Ok(())
}

#[test]
fn events_for_run_preserves_digest_fields_without_role_swap() -> Result<(), String> {
    let run = RunId::new(8304);
    let workflow = WorkflowDigest::from_bytes([0x64; 32]);
    let event = run_accepted_event(run, 1, workflow);

    assert_eq!(
        event,
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(1),
            workflow
        }
    );
    Ok(())
}

#[test]
fn events_for_run_returns_corrupt_recovery_data_when_event_record_is_malformed()
-> Result<(), String> {
    let run_a = RunId::new(8305);
    let run_b = RunId::new(8306);
    let digest = WorkflowDigest::from_bytes([0x65; 32]);

    let result = summarize_recovery_events(&[
        run_accepted_event(run_a, 0, digest),
        run_admission_event(run_b, 1, digest),
    ]);

    match result {
        Err(RecoveryError::ReplayDivergence { step, detail }) => {
            assert_eq!(step, StepIdx::ZERO);
            assert_eq!(detail.is_empty(), false);
        }
        other => return Err(format!("expected ReplayDivergence, got {other:?}")),
    }
    Ok(())
}

#[test]
fn recover_yaml_origin_run_recovers_state_from_persisted_artifact_journal_and_snapshot_without_yaml()
-> Result<(), String> {
    let run = RunId::new(8401);
    let digest = WorkflowDigest::from_bytes([0x71; 32]);
    let seed = recover_runtime_frame_seed_from_events(&[
        run_accepted_event(run, 0, digest),
        run_admission_event(run, 1, digest),
        run_finished_event(run, 2),
    ])
    .map_err(|error| error.to_string())?;

    assert_eq!(seed.summary.run, run);
    assert_eq!(seed.pc, StepIdx::ZERO);
    Ok(())
}

#[test]
fn recover_yaml_origin_run_returns_replay_divergence_when_snapshot_diverges_from_model()
-> Result<(), String> {
    let run_a = RunId::new(8402);
    let run_b = RunId::new(8403);
    let digest = WorkflowDigest::from_bytes([0x72; 32]);

    let result = recover_runtime_frame_seed_from_events(&[
        run_accepted_event(run_a, 0, digest),
        run_finished_event(run_b, 1),
    ]);

    match result {
        Err(RecoveryError::ReplayDivergence { step, detail }) => {
            assert_eq!(step, StepIdx::ZERO);
            assert_eq!(detail.is_empty(), false);
        }
        other => return Err(format!("expected ReplayDivergence, got {other:?}")),
    }
    Ok(())
}

#[test]
fn recover_yaml_origin_run_returns_corrupt_recovery_data_when_snapshot_or_frame_decode_fails()
-> Result<(), String> {
    let result = recover_runtime_frame_seed_from_events(&[]);

    assert_no_recovery_data(result, RunId::new(0))
}

#[test]
fn recover_yaml_origin_run_returns_no_recovery_data_when_no_durable_evidence_exists()
-> Result<(), String> {
    let result = summarize_recovery_events(&[]);

    assert_no_recovery_data(result, RunId::new(0))
}

#[test]
fn recover_yaml_origin_run_is_deterministic_for_identical_persisted_inputs() -> Result<(), String> {
    let run = RunId::new(8405);
    let digest = WorkflowDigest::from_bytes([0x75; 32]);
    let events = [
        run_accepted_event(run, 0, digest),
        run_admission_event(run, 1, digest),
        run_finished_event(run, 2),
    ];
    let first = summarize_recovery_events(&events).map_err(|error| error.to_string())?;
    let second = summarize_recovery_events(&events).map_err(|error| error.to_string())?;

    assert_eq!(first.summary(), second.summary());
    Ok(())
}

proptest! {
    #[test]
    fn source_digest_mismatch_returns_payload_digest_mismatch_when_claimed_digest_differs(
        source in prop::collection::vec(any::<u8>(), 1..256),
        replacement in any::<u8>()
    ) {
        let mut mutated = source.clone();
        if let Some(first) = mutated.first_mut() {
            *first = replacement.wrapping_add(1);
            if *first == source[0] {
                *first = replacement.wrapping_add(2);
            }
        }
        let claimed = WorkflowDigest::from_bytes(blake3::hash(&mutated).into());
        let temp = tempfile::tempdir().map_err(|error| TestCaseError::fail(error.to_string()))?;
        let journal = FjallJournal::open(temp.path(), None)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;

        let result = journal.put_workflow_source(&WorkflowSourceRecord { digest: claimed, source });

        prop_assert!(matches!(result, Err(JournalError::PayloadDigestMismatch)));
        let stored = journal.workflow_source(claimed)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(stored, None);
    }
}
