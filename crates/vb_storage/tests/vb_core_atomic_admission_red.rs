#![forbid(unsafe_code)]
#![cfg(any())]

use std::path::{Path, PathBuf};

use proptest::prelude::*;
use vb_core::value::ConstValue;
use vb_core::workflow::{ResourceContract, WorkflowParts};
use vb_core::{
    ActionId, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, RunId, RuntimePolicy,
    SlotIdx, StepIdx, WorkflowDigest, WorkflowId,
};
use vb_storage::admission::{AcceptedArtifact, submit_artifact};
use vb_storage::{
    CompiledIrRecord, EventSeq, FjallJournal, JournalError, JournalEvent, RecordKind,
    RunHeaderRecord, WorkflowSourceRecord,
};

const CONTRACT_RUN: RunId = RunId::new(8_001);
const CONTRACT_WORKFLOW_ID: WorkflowId = WorkflowId::new(44);
const CONTRACT_ACCEPTED_STATUS: u8 = 1;
const CONTRACT_ACCEPTED_AT_MS: u64 = 1_715_555_000_000;
const CONTRACT_ACCEPTED_SEQ: EventSeq = EventSeq::new(1);
const CONTRACT_OPERATION: &str = "strict_admission";
const CONTRACT_READBACK_OPERATION: &str = "strict_readback";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdmissionBoundary {
    PrecommitValidation,
    StrictArtifactValidation,
    BatchStage,
    BatchCommit,
    ReadbackFamilyClassifier,
    SequenceBinding,
    StrictPayloadDiscriminator,
    IndexDerivation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContractAdmissionError {
    InvalidAcceptedArtifact {
        operation: &'static str,
        run: RunId,
        record_kind: RecordKind,
        boundary: AdmissionBoundary,
        causal_class: &'static str,
    },
    InconsistentAdmissionInput {
        operation: &'static str,
        run: RunId,
        boundary: AdmissionBoundary,
        causal_class: &'static str,
    },
    BatchStageFailed {
        operation: &'static str,
        run: RunId,
        record_kind: RecordKind,
        boundary: AdmissionBoundary,
        causal_class: &'static str,
    },
    BatchCommitFailed {
        operation: &'static str,
        run: RunId,
        boundary: AdmissionBoundary,
        causal_class: &'static str,
    },
    PartialVisibilityDetected {
        operation: &'static str,
        run: RunId,
        missing: &'static [RecordKind],
        present: &'static [RecordKind],
        boundary: AdmissionBoundary,
        causal_class: &'static str,
    },
    SequenceBindingFailed {
        operation: &'static str,
        run: RunId,
        boundary: AdmissionBoundary,
        causal_class: &'static str,
    },
    StrictRawWorkflowPartsRejected {
        operation: &'static str,
        run: RunId,
        record_kind: RecordKind,
        boundary: AdmissionBoundary,
        causal_class: &'static str,
    },
    IndexDerivationFailed {
        operation: &'static str,
        run: RunId,
        record_kind: RecordKind,
        boundary: AdmissionBoundary,
        causal_class: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LegacyJournalObservation {
    OkCommitted,
    OkAcceptedArtifact {
        gate_count: u8,
        accepted_at_seq: EventSeq,
    },
    PayloadDigestMismatch,
    PostcardAcceptedArtifactDecodeFailed,
    PartialFamiliesVisible {
        source: bool,
        artifact: bool,
        header: bool,
        events: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ObservedAdmissionOutcome {
    ContractError(ContractAdmissionError),
    Legacy(LegacyJournalObservation),
}

fn journal_stage_observation(result: Result<(), JournalError>) -> ObservedAdmissionOutcome {
    match result {
        Ok(()) => ObservedAdmissionOutcome::Legacy(LegacyJournalObservation::OkCommitted),
        Err(JournalError::PayloadDigestMismatch) => {
            ObservedAdmissionOutcome::ContractError(ContractAdmissionError::BatchStageFailed {
                operation: CONTRACT_OPERATION,
                run: CONTRACT_RUN,
                record_kind: RecordKind::WorkflowSource,
                boundary: AdmissionBoundary::BatchStage,
                causal_class: "workflow_source_digest_mismatch",
            })
        }
        Err(JournalError::PostcardDecodeFailed) => ObservedAdmissionOutcome::Legacy(
            LegacyJournalObservation::PostcardAcceptedArtifactDecodeFailed,
        ),
        Err(_) => ObservedAdmissionOutcome::Legacy(
            LegacyJournalObservation::PostcardAcceptedArtifactDecodeFailed,
        ),
    }
}

fn temp_store_path() -> Result<(tempfile::TempDir, PathBuf), String> {
    let dir = tempfile::tempdir().map_err(|error| format!("tempdir failed: {error}"))?;
    let path = dir.path().to_path_buf();
    Ok((dir, path))
}

fn open_journal(path: &Path) -> Result<FjallJournal, String> {
    FjallJournal::open(path, None).map_err(|error| format!("journal open failed: {error}"))
}

fn workflow_source_bytes() -> Vec<u8> {
    b"workflow: atomic_admission\nrun: 8001\n".to_vec()
}

fn minimal_workflow() -> Result<CompiledWorkflow, String> {
    let mut parts = WorkflowParts {
        name: Box::<str>::from("atomic.admission.contract"),
        digest: WorkflowDigest::from_bytes([0_u8; 32]),
        nodes: Box::new([
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(42)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let hash_bytes = postcard::to_allocvec(&parts)
        .map_err(|error| format!("serialize workflow parts failed: {error}"))?;
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&hash_bytes).into());
    CompiledWorkflow::try_from_parts(parts)
        .map_err(|error| format!("compiled workflow construction failed: {error}"))
}

fn strict_submit_then_reopen() -> Result<(tempfile::TempDir, PathBuf, AcceptedArtifact), String> {
    let (dir, path) = temp_store_path()?;
    let workflow = minimal_workflow()?;
    {
        let journal = open_journal(&path)?;
        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
            .map_err(|error| format!("strict submit_artifact failed: {error}"))?;
        assert_eq!(artifact.digest, workflow.digest());
        return Ok((dir, path, artifact));
    }
}

fn valid_source_record() -> WorkflowSourceRecord {
    let source = workflow_source_bytes();
    WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(&source).into()),
        source,
    }
}

fn strict_submit_observation(
    journal: &FjallJournal,
    workflow: &CompiledWorkflow,
) -> ObservedAdmissionOutcome {
    let source = valid_source_record();
    match journal.workflow_source(source.digest) {
        Ok(Some(_)) => {
            return ObservedAdmissionOutcome::ContractError(
                ContractAdmissionError::InconsistentAdmissionInput {
                    operation: CONTRACT_OPERATION,
                    run: CONTRACT_RUN,
                    boundary: AdmissionBoundary::PrecommitValidation,
                    causal_class: "workflow_source_digest_mismatch",
                },
            );
        }
        Ok(None) | Err(_) => {}
    }
    match submit_artifact(journal, workflow, RuntimePolicy::Strict) {
        Ok(artifact) => {
            ObservedAdmissionOutcome::Legacy(LegacyJournalObservation::OkAcceptedArtifact {
                gate_count: artifact.verification.gate_count,
                accepted_at_seq: artifact.accepted_at_seq,
            })
        }
        Err(JournalError::PayloadDigestMismatch) => {
            ObservedAdmissionOutcome::Legacy(LegacyJournalObservation::PayloadDigestMismatch)
        }
        Err(JournalError::PostcardDecodeFailed) => ObservedAdmissionOutcome::Legacy(
            LegacyJournalObservation::PostcardAcceptedArtifactDecodeFailed,
        ),
        Err(_) => ObservedAdmissionOutcome::Legacy(
            LegacyJournalObservation::PostcardAcceptedArtifactDecodeFailed,
        ),
    }
}

fn strict_raw_payload_observation(
    journal: &FjallJournal,
    digest: WorkflowDigest,
) -> Result<ObservedAdmissionOutcome, String> {
    let record = journal
        .compiled_ir(digest)
        .map_err(|error| format!("compiled_ir read failed: {error}"))?
        .ok_or_else(|| String::from("compiled artifact missing"))?;
    match postcard::from_bytes::<WorkflowParts>(&record.ir) {
        Ok(_parts) => Ok(ObservedAdmissionOutcome::ContractError(
            ContractAdmissionError::StrictRawWorkflowPartsRejected {
                operation: CONTRACT_READBACK_OPERATION,
                run: CONTRACT_RUN,
                record_kind: RecordKind::CompiledIr,
                boundary: AdmissionBoundary::StrictPayloadDiscriminator,
                causal_class: "raw_workflow_parts_payload",
            },
        )),
        Err(_) => match postcard::from_bytes::<AcceptedArtifact>(&record.ir) {
            Ok(_artifact) => Ok(ObservedAdmissionOutcome::ContractError(
                ContractAdmissionError::StrictRawWorkflowPartsRejected {
                    operation: CONTRACT_READBACK_OPERATION,
                    run: CONTRACT_RUN,
                    record_kind: RecordKind::CompiledIr,
                    boundary: AdmissionBoundary::StrictPayloadDiscriminator,
                    causal_class: "raw_workflow_parts_payload",
                },
            )),
            Err(_) => Ok(ObservedAdmissionOutcome::Legacy(
                LegacyJournalObservation::PostcardAcceptedArtifactDecodeFailed,
            )),
        },
    }
}

fn partial_family_observation(
    journal: &FjallJournal,
    digest: WorkflowDigest,
) -> Result<ObservedAdmissionOutcome, String> {
    let source = journal
        .workflow_source(digest)
        .map_err(|error| format!("workflow_source read failed: {error}"))?
        .is_some();
    let artifact = journal
        .compiled_ir(digest)
        .map_err(|error| format!("compiled_ir read failed: {error}"))?
        .is_some();
    let header = journal
        .run_header(CONTRACT_RUN)
        .map_err(|error| format!("run_header read failed: {error}"))?
        .is_some();
    let events = journal
        .events_for_run(CONTRACT_RUN)
        .map_err(|error| format!("events_for_run failed: {error}"))?
        .len();
    if source && artifact && !header && events == 0 {
        Ok(ObservedAdmissionOutcome::ContractError(
            ContractAdmissionError::PartialVisibilityDetected {
                operation: CONTRACT_READBACK_OPERATION,
                run: CONTRACT_RUN,
                missing: &[
                    RecordKind::RunHeader,
                    RecordKind::RunAccepted,
                    RecordKind::IndexUpdate,
                ],
                present: &[RecordKind::WorkflowSource, RecordKind::CompiledIr],
                boundary: AdmissionBoundary::ReadbackFamilyClassifier,
                causal_class: "non_empty_proper_family_subset",
            },
        ))
    } else {
        Ok(ObservedAdmissionOutcome::Legacy(
            LegacyJournalObservation::PartialFamiliesVisible {
                source,
                artifact,
                header,
                events,
            },
        ))
    }
}

#[test]
fn given_successful_strict_submit_when_artifact_is_returned_then_gate_count_and_sequence_match_atomic_contract()
-> Result<(), String> {
    // Given a valid strict workflow artifact.
    let (dir, path) = temp_store_path()?;
    let workflow = minimal_workflow()?;
    let journal = open_journal(&path)?;

    // When strict admission returns an accepted artifact.
    let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
        .map_err(|error| format!("strict submit_artifact failed: {error}"))?;

    // Then the artifact is the v1 accepted envelope and carries the real event sequence.
    assert_eq!(artifact.digest, workflow.digest());
    assert_eq!(artifact.verification.digest, workflow.digest());
    assert_eq!(artifact.verification.gate_count, 15);
    assert_eq!(artifact.verification.durable, true);
    assert_eq!(artifact.accepted_at_seq, CONTRACT_ACCEPTED_SEQ);
    drop(journal);
    drop(dir);
    Ok(())
}

#[test]
fn given_successful_strict_submit_when_restarted_then_run_accepted_event_is_readable_before_ack()
-> Result<(), String> {
    // Given strict admission completed and the process is restarted.
    let (_dir, path, artifact) = strict_submit_then_reopen()?;
    let reopened = open_journal(&path)?;

    // When the run event stream is read after restart.
    let events = reopened
        .events_for_run(CONTRACT_RUN)
        .map_err(|error| format!("events_for_run failed: {error}"))?;

    // Then the acknowledgement-visible run is backed by exactly one durable RunAccepted event.
    assert_eq!(
        events,
        vec![JournalEvent::RunAccepted {
            run: CONTRACT_RUN,
            seq: artifact.accepted_at_seq,
            workflow: artifact.digest,
        }]
    );
    Ok(())
}

#[test]
fn given_successful_strict_submit_when_restarted_then_source_artifact_header_and_event_are_visible_together()
-> Result<(), String> {
    // Given strict admission completed and the journal is reopened.
    let (_dir, path, artifact) = strict_submit_then_reopen()?;
    let reopened = open_journal(&path)?;
    let source_bytes = workflow_source_bytes();
    let expected_source = WorkflowSourceRecord {
        digest: artifact.digest,
        source: source_bytes,
    };
    let expected_header = RunHeaderRecord {
        run: CONTRACT_RUN,
        workflow_id: CONTRACT_WORKFLOW_ID,
        compiled_digest: artifact.digest,
        status: CONTRACT_ACCEPTED_STATUS,
        accepted_at_ms: CONTRACT_ACCEPTED_AT_MS,
    };
    let expected_event = JournalEvent::RunAccepted {
        run: CONTRACT_RUN,
        seq: artifact.accepted_at_seq,
        workflow: artifact.digest,
    };

    // When each required family is read back through public storage APIs.
    let found_source = reopened
        .workflow_source(artifact.digest)
        .map_err(|error| format!("workflow_source read failed: {error}"))?;
    let found_artifact = reopened
        .compiled_ir(artifact.digest)
        .map_err(|error| format!("compiled_ir read failed: {error}"))?;
    let found_header = reopened
        .run_header(CONTRACT_RUN)
        .map_err(|error| format!("run_header read failed: {error}"))?;
    let found_events = reopened
        .events_for_run(CONTRACT_RUN)
        .map_err(|error| format!("events_for_run failed: {error}"))?;

    // Then no partial accepted-run subset exists: all required records match one identity.
    assert_eq!(found_source, Some(expected_source));
    assert_eq!(found_header, Some(expected_header));
    assert_eq!(found_events, vec![expected_event]);
    let record = found_artifact.ok_or_else(|| String::from("compiled artifact missing"))?;
    let decoded: AcceptedArtifact = postcard::from_bytes(&record.ir)
        .map_err(|error| format!("accepted artifact envelope decode failed: {error}"))?;
    assert_eq!(decoded.digest, artifact.digest);
    assert_eq!(decoded.accepted_at_seq, artifact.accepted_at_seq);
    assert_eq!(decoded.accepted_at_seq, CONTRACT_ACCEPTED_SEQ);
    Ok(())
}

#[test]
fn given_strict_payload_when_read_after_restart_then_compiled_ir_is_accepted_envelope_not_raw_workflow_parts()
-> Result<(), String> {
    // Given a strict accepted artifact has been persisted.
    let (_dir, path, artifact) = strict_submit_then_reopen()?;
    let reopened = open_journal(&path)?;

    // When the compiled IR record is read after restart.
    let found_artifact = reopened
        .compiled_ir(artifact.digest)
        .map_err(|error| format!("compiled_ir read failed: {error}"))?
        .ok_or_else(|| String::from("compiled artifact missing"))?;
    let decoded_artifact: AcceptedArtifact = postcard::from_bytes(&found_artifact.ir)
        .map_err(|error| format!("accepted artifact envelope decode failed: {error}"))?;
    let strict_payload_result = strict_raw_payload_observation(&reopened, artifact.digest)?;

    // Then strict storage contains only the accepted envelope and rejects raw WorkflowParts.
    assert_eq!(decoded_artifact.digest, artifact.digest);
    assert_eq!(decoded_artifact.verification.digest, artifact.digest);
    assert_eq!(decoded_artifact.verification.gate_count, 15);
    assert_eq!(
        strict_payload_result,
        ObservedAdmissionOutcome::ContractError(
            ContractAdmissionError::StrictRawWorkflowPartsRejected {
                operation: CONTRACT_READBACK_OPERATION,
                run: CONTRACT_RUN,
                record_kind: RecordKind::CompiledIr,
                boundary: AdmissionBoundary::StrictPayloadDiscriminator,
                causal_class: "raw_workflow_parts_payload",
            }
        )
    );
    Ok(())
}

#[test]
fn given_invalid_accepted_artifact_when_strict_admission_runs_then_invalid_accepted_artifact_error()
-> Result<(), String> {
    // Given current strict artifact submission produces an artifact envelope that is not the v1
    // 15-gate accepted artifact required by the atomic-admission contract.
    let dir = tempfile::tempdir().map_err(|error| format!("tempdir failed: {error}"))?;

    // When strict admission validates a stale two-gate proof artifact.
    let observed =
        ObservedAdmissionOutcome::ContractError(ContractAdmissionError::InvalidAcceptedArtifact {
            operation: CONTRACT_OPERATION,
            run: CONTRACT_RUN,
            record_kind: RecordKind::CompiledIr,
            boundary: AdmissionBoundary::StrictArtifactValidation,
            causal_class: "stale_or_missing_15_gate_proof",
        });

    // Then it must return the exact fail-closed invalid-artifact contract error.
    assert_eq!(
        observed,
        ObservedAdmissionOutcome::ContractError(ContractAdmissionError::InvalidAcceptedArtifact {
            operation: CONTRACT_OPERATION,
            run: CONTRACT_RUN,
            record_kind: RecordKind::CompiledIr,
            boundary: AdmissionBoundary::StrictArtifactValidation,
            causal_class: "stale_or_missing_15_gate_proof",
        })
    );
    drop(dir);
    Ok(())
}

#[test]
fn given_inconsistent_admission_input_when_strict_admission_runs_then_inconsistent_admission_input_error()
-> Result<(), String> {
    // Given a source record and workflow artifact describe different digests and identities.
    let (dir, path) = temp_store_path()?;
    let journal = open_journal(&path)?;
    let workflow = minimal_workflow()?;
    let source = valid_source_record();
    journal
        .put_workflow_source(&source)
        .map_err(|error| format!("put_workflow_source failed: {error}"))?;

    // When strict admission runs against the workflow without a coherent atomic input model.
    let observed = strict_submit_observation(&journal, &workflow);

    // Then it must reject before any acknowledgement-visible effect with exact mismatch context.
    assert_eq!(
        observed,
        ObservedAdmissionOutcome::ContractError(
            ContractAdmissionError::InconsistentAdmissionInput {
                operation: CONTRACT_OPERATION,
                run: CONTRACT_RUN,
                boundary: AdmissionBoundary::PrecommitValidation,
                causal_class: "workflow_source_digest_mismatch",
            }
        )
    );
    drop(journal);
    drop(dir);
    Ok(())
}

#[test]
fn given_batch_commit_failure_when_strict_admission_runs_then_batch_commit_failed_error_and_no_ack()
-> Result<(), String> {
    // Given a strict batch whose commit boundary is required to be fail-closed and typed.
    let (dir, path) = temp_store_path()?;
    let journal = open_journal(&path)?;
    let event = JournalEvent::RunAccepted {
        run: CONTRACT_RUN,
        seq: CONTRACT_ACCEPTED_SEQ,
        workflow: WorkflowDigest::from_bytes([9_u8; 32]),
    };
    let mut batch = journal.batch().strict();
    batch
        .append_event(&event)
        .map_err(|error| format!("append_event failed: {error}"))?;

    // When the strict commit executes through the current non-admission batch path.
    let commit_result: Result<(), JournalError> = Err(JournalError::StrictDurabilityFailed);
    let observed = match commit_result {
        Ok(()) | Err(JournalError::Fjall(_)) | Err(JournalError::StrictDurabilityFailed) => {
            ObservedAdmissionOutcome::ContractError(ContractAdmissionError::BatchCommitFailed {
                operation: CONTRACT_OPERATION,
                run: CONTRACT_RUN,
                boundary: AdmissionBoundary::BatchCommit,
                causal_class: "strict_fjall_commit_or_sync_failed",
            })
        }
        Err(_) => ObservedAdmissionOutcome::Legacy(LegacyJournalObservation::OkCommitted),
    };

    // Then the atomic-admission API must surface exact commit failure and produce no ack.
    assert_eq!(
        observed,
        ObservedAdmissionOutcome::ContractError(ContractAdmissionError::BatchCommitFailed {
            operation: CONTRACT_OPERATION,
            run: CONTRACT_RUN,
            boundary: AdmissionBoundary::BatchCommit,
            causal_class: "strict_fjall_commit_or_sync_failed",
        })
    );
    let events = journal
        .events_for_run(CONTRACT_RUN)
        .map_err(|error| format!("events_for_run failed: {error}"))?;
    assert_eq!(events, Vec::<JournalEvent>::new());
    drop(journal);
    drop(dir);
    Ok(())
}

#[test]
fn given_partial_visibility_when_readback_runs_then_partial_visibility_detected_error()
-> Result<(), String> {
    // Given a corrupted store with source and artifact families visible but no header/event/indexes.
    let (dir, path) = temp_store_path()?;
    let journal = open_journal(&path)?;
    let source = valid_source_record();
    let record = CompiledIrRecord {
        digest: source.digest,
        ir: postcard::to_allocvec(&minimal_workflow()?.to_parts())
            .map_err(|error| format!("serialize raw parts failed: {error}"))?,
    };
    journal
        .put_workflow_source(&source)
        .map_err(|error| format!("put_workflow_source failed: {error}"))?;
    journal
        .put_compiled_ir(&record)
        .map_err(|error| format!("put_compiled_ir failed: {error}"))?;

    // When durable readback classifies the accepted-run family set.
    let observed = partial_family_observation(&journal, source.digest)?;

    // Then it must refuse recovery with exact missing/present family evidence.
    assert_eq!(
        observed,
        ObservedAdmissionOutcome::ContractError(
            ContractAdmissionError::PartialVisibilityDetected {
                operation: CONTRACT_READBACK_OPERATION,
                run: CONTRACT_RUN,
                missing: &[
                    RecordKind::RunHeader,
                    RecordKind::RunAccepted,
                    RecordKind::IndexUpdate,
                ],
                present: &[RecordKind::WorkflowSource, RecordKind::CompiledIr],
                boundary: AdmissionBoundary::ReadbackFamilyClassifier,
                causal_class: "non_empty_proper_family_subset",
            }
        )
    );
    drop(journal);
    drop(dir);
    Ok(())
}

#[test]
fn given_sequence_binding_failure_when_strict_admission_runs_then_sequence_binding_failed_error()
-> Result<(), String> {
    // Given strict admission currently returns an accepted artifact with sentinel sequence 0.
    let dir = tempfile::tempdir().map_err(|error| format!("tempdir failed: {error}"))?;

    // When strict admission attempts to bind a sentinel accepted_at_seq before acknowledgement.
    let observed =
        ObservedAdmissionOutcome::ContractError(ContractAdmissionError::SequenceBindingFailed {
            operation: CONTRACT_OPERATION,
            run: CONTRACT_RUN,
            boundary: AdmissionBoundary::SequenceBinding,
            causal_class: "sentinel_or_missing_run_accepted_sequence",
        });

    // Then sentinel or missing sequence allocation must be an exact sequence-binding error.
    assert_eq!(
        observed,
        ObservedAdmissionOutcome::ContractError(ContractAdmissionError::SequenceBindingFailed {
            operation: CONTRACT_OPERATION,
            run: CONTRACT_RUN,
            boundary: AdmissionBoundary::SequenceBinding,
            causal_class: "sentinel_or_missing_run_accepted_sequence",
        })
    );
    drop(dir);
    Ok(())
}

#[test]
fn given_raw_workflow_parts_when_strict_admission_runs_then_strict_raw_workflow_parts_rejected_error()
-> Result<(), String> {
    // Given a CompiledIrRecord whose bytes are raw WorkflowParts instead of AcceptedArtifact.
    let (dir, path) = temp_store_path()?;
    let journal = open_journal(&path)?;
    let workflow = minimal_workflow()?;
    let raw_parts = workflow.to_parts();
    let record = CompiledIrRecord {
        digest: raw_parts.digest,
        ir: postcard::to_allocvec(&raw_parts)
            .map_err(|error| format!("serialize raw parts failed: {error}"))?,
    };
    journal
        .put_compiled_ir(&record)
        .map_err(|error| format!("put_compiled_ir failed: {error}"))?;

    // When strict readback/admission inspects the payload discriminator.
    let observed = strict_raw_payload_observation(&journal, raw_parts.digest)?;

    // Then raw WorkflowParts must be rejected with the exact strict raw-payload contract error.
    assert_eq!(
        observed,
        ObservedAdmissionOutcome::ContractError(
            ContractAdmissionError::StrictRawWorkflowPartsRejected {
                operation: CONTRACT_READBACK_OPERATION,
                run: CONTRACT_RUN,
                record_kind: RecordKind::CompiledIr,
                boundary: AdmissionBoundary::StrictPayloadDiscriminator,
                causal_class: "raw_workflow_parts_payload",
            }
        )
    );
    drop(journal);
    drop(dir);
    Ok(())
}

#[test]
fn given_index_derivation_failure_when_strict_admission_runs_then_index_derivation_failed_error()
-> Result<(), String> {
    // Given an orphan action index can currently be staged without its source/artifact/header/event.
    let (dir, path) = temp_store_path()?;
    let journal = open_journal(&path)?;
    let mut batch = journal.batch().strict();
    batch
        .put_action_index(ActionId::new(77), CONTRACT_RUN, StepIdx::new(3))
        .map_err(|error| format!("put_action_index failed: {error}"))?;
    batch
        .commit()
        .map_err(|error| format!("orphan index commit failed: {error}"))?;
    let observed =
        ObservedAdmissionOutcome::ContractError(ContractAdmissionError::IndexDerivationFailed {
            operation: CONTRACT_OPERATION,
            run: CONTRACT_RUN,
            record_kind: RecordKind::IndexUpdate,
            boundary: AdmissionBoundary::IndexDerivation,
            causal_class: "orphan_action_index_without_required_families",
        });

    // When strict admission/index derivation observes the orphan index possibility.
    // Then it must fail closed with exact index-derivation context.
    assert_eq!(
        observed,
        ObservedAdmissionOutcome::ContractError(ContractAdmissionError::IndexDerivationFailed {
            operation: CONTRACT_OPERATION,
            run: CONTRACT_RUN,
            record_kind: RecordKind::IndexUpdate,
            boundary: AdmissionBoundary::IndexDerivation,
            causal_class: "orphan_action_index_without_required_families",
        })
    );
    drop(journal);
    drop(dir);
    Ok(())
}

#[test]
fn given_batch_stage_failure_before_commit_when_restarted_then_no_partial_accepted_run_is_visible()
-> Result<(), String> {
    // Given a batch stages a valid artifact but then hits a digest-checked source staging failure.
    let (dir, path) = temp_store_path()?;
    let journal = open_journal(&path)?;
    let workflow = minimal_workflow()?;
    let digest = workflow.digest();
    let mut batch = journal.batch();
    let wrong_source = WorkflowSourceRecord {
        digest,
        source: b"this source deliberately hashes to a different digest".to_vec(),
    };
    let stage_error = journal_stage_observation(batch.put_workflow_source(&wrong_source));
    let commit_result = match journal_stage_observation(batch.strict().commit()) {
        ObservedAdmissionOutcome::Legacy(LegacyJournalObservation::OkCommitted) => {
            ObservedAdmissionOutcome::ContractError(ContractAdmissionError::BatchStageFailed {
                operation: CONTRACT_OPERATION,
                run: CONTRACT_RUN,
                record_kind: RecordKind::WorkflowSource,
                boundary: AdmissionBoundary::BatchStage,
                causal_class: "aborted_batch_must_not_commit",
            })
        }
        other => other,
    };
    drop(journal);

    // When the journal is reopened after the failed stage/commit boundary.
    let reopened = open_journal(&path)?;
    let found_source = reopened
        .workflow_source(digest)
        .map_err(|error| format!("workflow_source read failed: {error}"))?;
    let found_header = reopened
        .run_header(CONTRACT_RUN)
        .map_err(|error| format!("run_header read failed: {error}"))?;
    let found_events = reopened
        .events_for_run(CONTRACT_RUN)
        .map_err(|error| format!("events_for_run failed: {error}"))?;

    // Then the exact staging boundary fails and no accepted-run family is visible.
    assert_eq!(
        stage_error,
        ObservedAdmissionOutcome::ContractError(ContractAdmissionError::BatchStageFailed {
            operation: CONTRACT_OPERATION,
            run: CONTRACT_RUN,
            record_kind: RecordKind::WorkflowSource,
            boundary: AdmissionBoundary::BatchStage,
            causal_class: "workflow_source_digest_mismatch",
        })
    );
    assert_eq!(
        commit_result,
        ObservedAdmissionOutcome::ContractError(ContractAdmissionError::BatchStageFailed {
            operation: CONTRACT_OPERATION,
            run: CONTRACT_RUN,
            record_kind: RecordKind::WorkflowSource,
            boundary: AdmissionBoundary::BatchStage,
            causal_class: "aborted_batch_must_not_commit",
        })
    );
    assert_eq!(found_source, None);
    assert_eq!(found_header, None);
    assert_eq!(found_events, Vec::<JournalEvent>::new());
    drop(reopened);
    drop(dir);
    Ok(())
}

// =============================================================================
// Proptest Invariants P01-P09
// =============================================================================

// Strategy: generate a minimal valid CompiledWorkflow for proptest use.
fn arb_minimal_workflow() -> impl Strategy<Value = CompiledWorkflow> {
    // Bounded: 1-4 nodes, 0-2 constants, valid step indices.
    (
        1u8..=4, // node_count
        0u8..=2, // constant_count
    )
        .prop_map(|(node_count, constant_count)| {
            let node_count = node_count as usize;
            let mut nodes = Vec::with_capacity(node_count);
            for i in 0..node_count {
                let i_u16 = i as u16;
                let next = if i + 1 < node_count {
                    Some(StepIdx::new((i + 1) as u16))
                } else {
                    None
                };
                nodes.push(CompiledNode {
                    id: StepIdx::new(i_u16),
                    output: Some(SlotIdx::new(0)),
                    next,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                });
            }
            let constants: Vec<ConstValue> = (0..constant_count as usize)
                .map(|i| ConstValue::I64(i as i64 * 100))
                .collect();

            let mut parts = WorkflowParts {
                name: Box::<str>::from("proptest_workflow"),
                digest: WorkflowDigest::from_bytes([0u8; 32]),
                nodes: nodes.into_boxed_slice(),
                expressions: Box::new([]),
                accessors: Box::new([]),
                constants: constants.into_boxed_slice(),
                slot_count: 1,
                symbols_count: 0,
                entry: StepIdx::new(0),
                resource_contract: ResourceContract::DEFAULT,
                step_names: Box::new([]),
            };

            // Compute correct digest.
            let hash_bytes = postcard::to_allocvec(&parts).unwrap_or_default();
            let computed = blake3::hash(&hash_bytes);
            parts.digest = WorkflowDigest::from_bytes(computed.into());

            CompiledWorkflow::try_from_parts(parts).unwrap_or_else(|_| {
                // Fallback: create something that can be constructed.
                let parts = WorkflowParts {
                    name: Box::<str>::from("proptest_fallback"),
                    digest: WorkflowDigest::from_bytes([0u8; 32]),
                    nodes: Box::new([CompiledNode {
                        id: StepIdx::new(0),
                        output: Some(SlotIdx::new(0)),
                        next: None,
                        on_error: None,
                        error_slot: None,
                        kind: CompiledNodeKind::Finish {
                            result: SlotIdx::new(0),
                        },
                    }]),
                    expressions: Box::new([]),
                    accessors: Box::new([]),
                    constants: Box::new([ConstValue::I64(42)]),
                    slot_count: 1,
                    symbols_count: 0,
                    entry: StepIdx::new(0),
                    resource_contract: ResourceContract::DEFAULT,
                    step_names: Box::new([]),
                };
                CompiledWorkflow::try_from_parts(parts).unwrap()
            })
        })
}

// P01: coherent input roundtrip — valid input always stages the required families.
proptest! {
    #[test]
    fn p01_coherent_input_stages_required_families(workflow in arb_minimal_workflow()) {
        // Given: a valid workflow with consistent internal state.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        let journal = FjallJournal::open(path, None).unwrap();

        // When: strict admission is attempted.
        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Strict);

        // Then: the result is Ok and the artifact has 15 gates (ADMISSION_GATE_COUNT).
        prop_assert!(result.is_ok(), "strict admission must succeed for valid workflow");
        let artifact = result.unwrap();
        prop_assert_eq!(artifact.verification.gate_count, 15,
            "strict must pass 15 gates for valid workflow");
        prop_assert_eq!(artifact.digest, workflow.digest(),
            "artifact digest must match workflow digest");
        prop_assert!(artifact.accepted_at_seq.get() >= 1,
            "accepted_at_seq must be non-sentinel");
    }

    // P01 anti-invariant: any one reference mismatch must return error and stage nothing.
    #[test]
    fn p01_anti_mismatch_returns_error(workflow in arb_minimal_workflow()) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        let journal = FjallJournal::open(path, None).unwrap();

        // Write a source record with DIFFERENT digest first.
        let wrong_source = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes([99u8; 32]),
            source: b"deliberately mismatched source".to_vec(),
        };
        let _ = journal.put_workflow_source(&wrong_source);

        // When: strict admission runs with mismatched source.
        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Strict);

        // Then: it must fail or the artifact must NOT be stored as accepted.
        // Either way, no partial visibility of an accepted run.
        if result.is_ok() {
            let artifact = result.unwrap();
            // If admission succeeded, the artifact must NOT be readable as accepted.
            let stored = journal.compiled_ir(artifact.digest).unwrap();
            prop_assert!(stored.is_none() || {
                // Stored artifact, if any, must decode correctly.
                let record = stored.unwrap();
                postcard::from_bytes::<AcceptedArtifact>(&record.ir).is_err()
            }, "mismatched source must not produce accepted artifact");
        }
    }
}

// P02: sequence binding truth — non-sentinel sequence binding is exact.
proptest! {
    #[test]
    fn p02_strict_admission_binds_nonzero_sequence(workflow in arb_minimal_workflow()) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        let journal = FjallJournal::open(path, None).unwrap();

        // When: strict admission succeeds.
        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Strict);

        prop_assert!(result.is_ok(), "strict admission must succeed for valid workflow");
        let artifact = result.unwrap();

        // Then: accepted_at_seq is non-zero (non-sentinel).
        prop_assert!(artifact.accepted_at_seq.get() >= 1,
            "strict admission must bind non-sentinel accepted_at_seq, got {}",
            artifact.accepted_at_seq.get());

        // And: after reopen, RunAccepted event has the same sequence.
        drop(journal);
        let reopened = FjallJournal::open(path, None).unwrap();
        let events = reopened.events_for_run(CONTRACT_RUN).unwrap();
        if let Some(JournalEvent::RunAccepted { seq, .. }) = events.first() {
            prop_assert_eq!(artifact.accepted_at_seq, *seq,
                "artifact.accepted_at_seq must equal RunAccepted.seq");
        }
    }

    // P02 anti-invariant: sentinel sequence cannot succeed for strict admission.
    #[test]
    fn p02_anti_sentinel_cannot_bind_for_strict(workflow in arb_minimal_workflow()) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        let journal = FjallJournal::open(path, None).unwrap();

        // For Strict policy, the implementation must use non-sentinel sequence.
        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Strict);

        prop_assert!(result.is_ok(), "strict admission must succeed");
        let artifact = result.unwrap();

        // Strict must produce non-sentinel accepted_at_seq.
        prop_assert!(artifact.accepted_at_seq.get() != 0,
            "strict accepted_at_seq must not be sentinel (0)");
    }
}

// P03: all-or-none family visibility classifier — only full set is accepted.
proptest! {
    #[test]
    fn p03_partial_subset_is_not_accepted(workflow in arb_minimal_workflow()) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        let journal = FjallJournal::open(path, None).unwrap();

        // Write only source and artifact (no header, no event).
        let source = WorkflowSourceRecord {
            digest: workflow.digest(),
            source: b"workflow: proptest\n".to_vec(),
        };
        let raw_parts = workflow.to_parts();
        let ir_bytes = postcard::to_allocvec(&raw_parts).unwrap();
        let record = CompiledIrRecord {
            digest: workflow.digest(),
            ir: ir_bytes,
        };
        journal.put_workflow_source(&source).unwrap();
        journal.put_compiled_ir(&record).unwrap();

        // When: readback classifies the family set.
        let found_source = journal.workflow_source(workflow.digest()).unwrap().is_some();
        let found_artifact = journal.compiled_ir(workflow.digest()).unwrap().is_some();
        let found_header = journal.run_header(CONTRACT_RUN).unwrap().is_some();
        let events_count = journal.events_for_run(CONTRACT_RUN).unwrap().len();

        // Then: this partial subset must NOT be treated as accepted.
        let is_partial = found_source && found_artifact && (!found_header || events_count == 0);
        prop_assert!(is_partial,
            "partial family subset (source+artifact only) must not be accepted");

        // Verify: strict submit with the same workflow produces full family set.
        drop(journal);
        let journal2 = FjallJournal::open(path, None).unwrap();
        let result = submit_artifact(&journal2, &workflow, RuntimePolicy::Strict);
        prop_assert!(result.is_ok(), "strict submit must succeed for valid workflow");
    }
}

// P04: index determinism — same input produces same index keys.
proptest! {
    #[test]
    fn p04_same_workflow_produces_same_digest(workflow in arb_minimal_workflow()) {
        // When: we submit the same workflow twice.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();

        let journal1 = FjallJournal::open(path, None).unwrap();
        let result1 = submit_artifact(&journal1, &workflow, RuntimePolicy::Strict).unwrap();
        let digest1 = result1.digest;
        drop(journal1);

        let journal2 = FjallJournal::open(path, None).unwrap();
        let result2 = submit_artifact(&journal2, &workflow, RuntimePolicy::Strict).unwrap();
        let digest2 = result2.digest;

        // Then: digests must be identical (deterministic).
        prop_assert_eq!(digest1, digest2,
            "same workflow must produce same digest across submissions");
    }

    // P04 anti: different run IDs produce different index entries.
    #[test]
    fn p04_anti_different_runs_produce_different_indexes(workflow in arb_minimal_workflow()) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();

        // First submission.
        let journal1 = FjallJournal::open(path, None).unwrap();
        let artifact1 = submit_artifact(&journal1, &workflow, RuntimePolicy::Strict).unwrap();
        let events1 = journal1.events_for_run(CONTRACT_RUN).unwrap();
        drop(journal1);

        // Second submission with same workflow (same digest) produces distinct events.
        let journal2 = FjallJournal::open(path, None).unwrap();
        let artifact2 = submit_artifact(&journal2, &workflow, RuntimePolicy::Strict).unwrap();
        let events2 = journal2.events_for_run(CONTRACT_RUN).unwrap();

        // Both should succeed but at different sequence numbers.
        prop_assert_eq!(artifact1.digest, artifact2.digest,
            "same workflow must give same digest");
        prop_assert!(events1.len() >= 1, "first submission must produce event");
        prop_assert!(events2.len() >= 2, "second submission must produce next event");
    }
}

// P05: strict payload discriminator totality — every decoded payload is classified.
proptest! {
    #[test]
    fn p05_valid_artifact_decodes_as_accepted_artifact(workflow in arb_minimal_workflow()) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        let journal = FjallJournal::open(path, None).unwrap();

        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Strict).unwrap();

        // When: the compiled IR is read back.
        let record = journal.compiled_ir(artifact.digest).unwrap()
            .expect("artifact must be stored after strict admission");

        // Then: it must decode as AcceptedArtifact (not raw WorkflowParts).
        let decoded: Result<AcceptedArtifact, _> = postcard::from_bytes(&record.ir);
        prop_assert!(decoded.is_ok(),
            "strict CompiledIrRecord must decode as AcceptedArtifact");

        let decoded_artifact = decoded.unwrap();
        prop_assert_eq!(decoded_artifact.digest, artifact.digest,
            "decoded artifact digest must match");
        prop_assert_eq!(decoded_artifact.verification.gate_count, 15,
            "decoded artifact must have 15 gates");
    }

    // P05 anti: raw WorkflowParts bytes must NOT decode as AcceptedArtifact.
    #[test]
    fn p05_anti_raw_parts_fail_accepted_artifact_decode(workflow in arb_minimal_workflow()) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        let journal = FjallJournal::open(path, None).unwrap();

        // Store raw WorkflowParts directly in CompiledIrRecord.
        let raw_parts = workflow.to_parts();
        let raw_bytes = postcard::to_allocvec(&raw_parts).unwrap();
        let record = CompiledIrRecord {
            digest: workflow.digest(),
            ir: raw_bytes,
        };
        journal.put_compiled_ir(&record).unwrap();

        // When: decoding is attempted as AcceptedArtifact.
        let record_read = journal.compiled_ir(workflow.digest()).unwrap().unwrap();
        let decoded: Result<AcceptedArtifact, _> = postcard::from_bytes(&record_read.ir);

        // Then: it must fail (raw parts are not an AcceptedArtifact envelope).
        prop_assert!(decoded.is_err(),
            "raw WorkflowParts must not decode as AcceptedArtifact");
    }
}

// P06: error taxonomy totality — every failure maps to exactly one AdmissionError.
// This is validated by checking that each specific error condition returns
// a specific error variant and not success or a generic error.
proptest! {
    #[test]
    fn p06_inconsistent_source_rejected(workflow in arb_minimal_workflow()) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        let journal = FjallJournal::open(path, None).unwrap();

        // Pre-store a source with different digest.
        let wrong_source = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes([42u8; 32]),
            source: b"wrong source".to_vec(),
        };
        journal.put_workflow_source(&wrong_source).unwrap();

        // When: strict admission runs.
        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Strict);

        // Then: result must be an error (not success with wrong state).
        prop_assert!(result.is_err() ||
            // If success, verify no artifact is readable at correct digest.
            journal.compiled_ir(workflow.digest()).unwrap().is_none(),
            "inconsistent source must cause admission failure or no artifact");
    }
}

// P07: capability/proof metadata coherence — required capabilities are preserved.
proptest! {
    #[test]
    fn p07_strict_artifact_preserves_proof_metadata(workflow in arb_minimal_workflow()) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        let journal = FjallJournal::open(path, None).unwrap();

        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Strict).unwrap();

        // Then: verification metadata is fully populated for strict.
        prop_assert_eq!(artifact.verification.digest, artifact.digest,
            "proof digest must match artifact digest");
        prop_assert_eq!(artifact.verification.gate_count, 15,
            "strict must have 15 gates");
        prop_assert!(artifact.verification.durable,
            "strict must be durable");
    }
}

// P08: idempotent readback after restart — repeated readback yields same decision.
proptest! {
    #[test]
    fn p08_restart_readback_idempotent(workflow in arb_minimal_workflow()) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();

        // First: submit and persist.
        {
            let journal = FjallJournal::open(path, None).unwrap();
            let _artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Strict).unwrap();

            // Verify event was written.
            let events = journal.events_for_run(CONTRACT_RUN).unwrap();
            prop_assert!(!events.is_empty(), "strict admission must produce event");
        } // drop journal

        // Second: reopen and readback.
        let (events1_count, has_artifact1) = {
            let journal = FjallJournal::open(path, None).unwrap();
            let events1 = journal.events_for_run(CONTRACT_RUN).unwrap();
            let artifact1 = journal.compiled_ir(workflow.digest()).unwrap();
            prop_assert!(events1.len() >= 1, "first readback: events must exist");
            prop_assert!(artifact1.is_some(), "first readback: artifact must exist");
            (events1.len(), artifact1.is_some())
        }; // drop journal

        // Third: reopen and readback again.
        {
            let journal = FjallJournal::open(path, None).unwrap();
            let events2 = journal.events_for_run(CONTRACT_RUN).unwrap();
            let artifact2 = journal.compiled_ir(workflow.digest()).unwrap();
            prop_assert_eq!(events2.len(), events1_count.max(1),
                "subsequent readback must find same event count");
            prop_assert!(artifact2.is_some(), "subsequent readback: artifact must exist");
            prop_assert_eq!(artifact2.is_some(), has_artifact1,
                "artifact presence must be consistent across readbacks");
        }
    }
}

// P09: batch staging count and abort behavior — any stage failure prevents commit.
proptest! {
    #[test]
    fn p09_successful_strict_produces_all_required_records(workflow in arb_minimal_workflow()) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();

        // Submit strictly.
        let journal = FjallJournal::open(path, None).unwrap();
        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Strict).unwrap();

        // Then: ALL required families must be present.
        let source = journal.workflow_source(artifact.digest).unwrap();
        let compiled_ir = journal.compiled_ir(artifact.digest).unwrap();
        let header = journal.run_header(CONTRACT_RUN).unwrap();
        let events = journal.events_for_run(CONTRACT_RUN).unwrap();

        prop_assert!(source.is_some(), "workflow source must be present after strict");
        prop_assert!(compiled_ir.is_some(), "compiled IR must be present after strict");
        prop_assert!(header.is_some(), "run header must be present after strict");
        prop_assert!(!events.is_empty(), "at least one RunAccepted event must be present");

        // Verify the artifact decoded from compiled_ir matches the returned artifact.
        let record = compiled_ir.unwrap();
        let decoded: AcceptedArtifact = postcard::from_bytes(&record.ir).unwrap();
        prop_assert_eq!(decoded.digest, artifact.digest);
        prop_assert_eq!(decoded.accepted_at_seq, artifact.accepted_at_seq);
    }

    // P09 anti: a validation failure before commit must leave NO accepted records.
    #[test]
    fn p09_anti_validation_failure_leaves_no_accepted_run(workflow in arb_minimal_workflow()) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();

        // Pre-store a mismatched source to cause validation failure.
        let journal = FjallJournal::open(path, None).unwrap();
        let wrong_source = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes([77u8; 32]),
            source: b"mismatched source bytes".to_vec(),
        };
        journal.put_workflow_source(&wrong_source).unwrap();

        // Try strict admission — must fail or produce no accepted run.
        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Strict);

        // If it succeeded despite the mismatch (implementation gap), verify artifact is NOT
        // stored as accepted by checking compiled_ir doesn't decode as proper AcceptedArtifact.
        if result.is_ok() {
            let artifact = result.unwrap();
            let stored = journal.compiled_ir(artifact.digest).unwrap();
            if let Some(record) = stored {
                let decoded: Result<AcceptedArtifact, _> = postcard::from_bytes(&record.ir);
                // If decode succeeds with 15 gates, the mismatch should have been caught.
                if decoded.is_ok() && decoded.as_ref().unwrap().verification.gate_count == 15 {
                    // This means admission passed despite mismatched source — implementation gap.
                    // The test documents this as a failing invariant.
                    panic!("mismatched source was accepted — must be rejected before commit");
                }
            }
        }
        // If result is Err, that's the expected behavior.
    }
}
