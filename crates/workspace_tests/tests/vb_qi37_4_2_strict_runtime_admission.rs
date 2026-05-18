use std::sync::Arc;

use proptest::prelude::*;
use vb_core::budget::{AggregateResourceBudget, AggregateResourceCapacity};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_core::value::ConstValue;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_runtime::RuntimeError;
use vb_runtime::admission::{
    AcceptedArtifactStore, AdmissionError, ArtifactEnvelopeError, ArtifactStore,
    REQUIRED_GATE_COUNT, RunAdmission, admit_artifact_run, admit_run_with_budget,
};
use vb_runtime::journal::{RuntimeJournalEvent, VolatileRuntimeJournal};
use vb_runtime::shard::{Shard, ShardCommand, ShardConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
enum ObservedAdmissionDiagnostic {
    Admitted(RunAdmission),
    NotFound {
        digest: WorkflowDigest,
    },
    DecodeFailed,
    InvalidGateCount {
        found: u8,
        required: u8,
    },
    InvalidProofFlag {
        flag: &'static str,
    },
    CapabilityDenied {
        action: ActionId,
        required: Capability,
        granted: CapabilitySet,
    },
    ResourceCapacityExceeded {
        resource: &'static str,
        requested: u64,
        available: u64,
    },
    DigestMismatch {
        requested: WorkflowDigest,
        record: WorkflowDigest,
        envelope: WorkflowDigest,
    },
    StaleCertificate {
        digest: WorkflowDigest,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PublicAdmissionDiagnostic {
    category: &'static str,
    digest: Option<WorkflowDigest>,
    cause: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DenialStateSnapshot {
    active_runs: usize,
    journal_events: Vec<RuntimeJournalEvent>,
    command_queue_len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InvalidEnvelopeCase {
    GateCount { found: u8 },
    ProofFlag { flag: &'static str },
}

#[derive(Clone)]
struct FixedAcceptedStore {
    result: Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError>,
}

impl AcceptedArtifactStore for FixedAcceptedStore {
    fn load_accepted_artifact(
        &self,
        _artifact_digest: WorkflowDigest,
    ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
        self.result.clone()
    }
}

struct PresentArtifactStore;

impl ArtifactStore for PresentArtifactStore {
    fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
        true
    }
}

fn digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

fn cap(name: &str, action: u16) -> Capability {
    Capability::new(name.into(), ActionId::new(action))
}

fn caps(values: Box<[Capability]>) -> CapabilitySet {
    CapabilitySet::from_grants(values)
}

fn accepted_artifact(
    artifact_digest: WorkflowDigest,
    proof_digest: WorkflowDigest,
    gate_count: u8,
    durable: bool,
    required_capabilities: Box<[Capability]>,
) -> vb_storage::admission::AcceptedArtifact {
    accepted_artifact_with_seq(
        artifact_digest,
        proof_digest,
        gate_count,
        durable,
        required_capabilities,
        0,
    )
}

fn accepted_artifact_with_seq(
    artifact_digest: WorkflowDigest,
    proof_digest: WorkflowDigest,
    gate_count: u8,
    durable: bool,
    required_capabilities: Box<[Capability]>,
    accepted_at_seq: u64,
) -> vb_storage::admission::AcceptedArtifact {
    vb_storage::admission::AcceptedArtifact {
        digest: artifact_digest,
        ir: Vec::new(),
        verification: vb_storage::admission::VerificationProof {
            digest: proof_digest,
            gate_count,
            durable,
            bounded: true,
            taint_safe: true,
            retry_safe: true,
            idempotency_verified: true,
            replayable: true,
            idempotency_keyed: Box::new([]),
            idempotency_attested: Box::new([]),
            warnings: Vec::new(),
        },
        accepted_at_seq: vb_storage::types::EventSeq::new(accepted_at_seq),
        required_capabilities,
    }
}

fn accepted_artifact_with_flags(
    artifact_digest: WorkflowDigest,
    proof_digest: WorkflowDigest,
    gate_count: u8,
    durable: bool,
    bounded: bool,
    taint_safe: bool,
    retry_safe: bool,
    replayable: bool,
    required_capabilities: Box<[Capability]>,
) -> vb_storage::admission::AcceptedArtifact {
    accepted_artifact_with_flags_and_seq(
        artifact_digest,
        proof_digest,
        gate_count,
        durable,
        bounded,
        taint_safe,
        retry_safe,
        replayable,
        required_capabilities,
        0,
    )
}

fn accepted_artifact_with_flags_and_seq(
    artifact_digest: WorkflowDigest,
    proof_digest: WorkflowDigest,
    gate_count: u8,
    durable: bool,
    bounded: bool,
    taint_safe: bool,
    retry_safe: bool,
    replayable: bool,
    required_capabilities: Box<[Capability]>,
    accepted_at_seq: u64,
) -> vb_storage::admission::AcceptedArtifact {
    vb_storage::admission::AcceptedArtifact {
        digest: artifact_digest,
        ir: Vec::new(),
        verification: vb_storage::admission::VerificationProof {
            digest: proof_digest,
            gate_count,
            durable,
            bounded,
            taint_safe,
            retry_safe,
            idempotency_verified: true,
            replayable,
            idempotency_keyed: Box::new([]),
            idempotency_attested: Box::new([]),
            warnings: Vec::new(),
        },
        accepted_at_seq: vb_storage::types::EventSeq::new(accepted_at_seq),
        required_capabilities,
    }
}

fn minimal_workflow() -> Result<CompiledWorkflow, String> {
    let mut parts = WorkflowParts {
        name: Box::<str>::from("vb_qi37_4_2_runtime_admission_test"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
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
    let hash_bytes = postcard::to_allocvec(&parts).map_err(|error| error.to_string())?;
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&hash_bytes).into());
    CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())
}

fn runtime_diagnostic(
    error: RuntimeError,
    fallback_digest: WorkflowDigest,
) -> PublicAdmissionDiagnostic {
    match error {
        RuntimeError::AdmissionArtifactNotFound { digest } => PublicAdmissionDiagnostic {
            category: "not_found",
            digest: Some(digest),
            cause: "artifact_not_found",
        },
        RuntimeError::AdmissionArtifactInvalid { digest }
            if digest == WorkflowDigest::from_bytes([0u8; 32]) =>
        {
            PublicAdmissionDiagnostic {
                category: "decode_failed",
                digest: Some(fallback_digest),
                cause: "artifact_envelope_decode_failed",
            }
        }
        RuntimeError::AdmissionArtifactInvalid { digest } => PublicAdmissionDiagnostic {
            category: "invalid_envelope",
            digest: Some(digest),
            cause: "artifact_envelope_invalid",
        },
        RuntimeError::AdmissionCapabilityDenied { .. } => PublicAdmissionDiagnostic {
            category: "capability_denied",
            digest: Some(fallback_digest),
            cause: "capability_profile_mismatch",
        },
        RuntimeError::AdmissionArtifactDigestMismatch { requested, .. } => {
            PublicAdmissionDiagnostic {
                category: "digest_mismatch",
                digest: Some(requested),
                cause: "requested_record_envelope_mismatch",
            }
        }
        RuntimeError::AdmissionDigestMismatch { requested, .. } => PublicAdmissionDiagnostic {
            category: "digest_mismatch",
            digest: Some(requested),
            cause: "requested_record_envelope_mismatch",
        },
        RuntimeError::ActiveRunCapacityExceeded { .. } => PublicAdmissionDiagnostic {
            category: "resource_capacity_exceeded",
            digest: Some(fallback_digest),
            cause: "resource_capacity",
        },
        _ => PublicAdmissionDiagnostic {
            category: "unexpected_runtime_error",
            digest: Some(fallback_digest),
            cause: "unexpected",
        },
    }
}

fn snapshot(
    shard: &Shard,
    journal: &VolatileRuntimeJournal,
) -> Result<DenialStateSnapshot, RuntimeError> {
    Ok(DenialStateSnapshot {
        active_runs: shard.active_run_count(),
        journal_events: journal.snapshot()?,
        command_queue_len: shard.command_queue_len(),
    })
}

fn run_strict_submit_with_store(
    store: FixedAcceptedStore,
    run_id: RunId,
    workflow: CompiledWorkflow,
    caps: CapabilitySet,
) -> Result<
    (
        Result<bool, RuntimeError>,
        DenialStateSnapshot,
        DenialStateSnapshot,
    ),
    RuntimeError,
> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut shard = Shard::new_with_journal_and_artifact_store(
        ShardConfig {
            policy: RuntimePolicy::Strict,
            ..ShardConfig::default()
        },
        journal.clone(),
        Arc::new(store),
    );
    let before = snapshot(&shard, &journal)?;
    shard.enqueue(ShardCommand::SubmitPrePersisted {
        run: run_id,
        workflow,
        caps,
    })?;
    let tick_result = shard.tick();
    let after = snapshot(&shard, &journal)?;
    Ok((tick_result, before, after))
}

fn public_diagnostic_from_observed(
    observed: ObservedAdmissionDiagnostic,
) -> PublicAdmissionDiagnostic {
    match observed {
        ObservedAdmissionDiagnostic::NotFound { digest } => PublicAdmissionDiagnostic {
            category: "not_found",
            digest: Some(digest),
            cause: "artifact_not_found",
        },
        ObservedAdmissionDiagnostic::DecodeFailed => PublicAdmissionDiagnostic {
            category: "decode_failed",
            digest: None,
            cause: "artifact_envelope_decode_failed",
        },
        ObservedAdmissionDiagnostic::InvalidGateCount { .. } => PublicAdmissionDiagnostic {
            category: "gate_mismatch",
            digest: None,
            cause: "invalid_gate_count",
        },
        ObservedAdmissionDiagnostic::InvalidProofFlag { flag } => PublicAdmissionDiagnostic {
            category: "invalid_envelope",
            digest: None,
            cause: flag,
        },
        ObservedAdmissionDiagnostic::CapabilityDenied { .. } => PublicAdmissionDiagnostic {
            category: "capability_denied",
            digest: None,
            cause: "capability_profile_mismatch",
        },
        ObservedAdmissionDiagnostic::ResourceCapacityExceeded { .. } => PublicAdmissionDiagnostic {
            category: "resource_capacity_exceeded",
            digest: None,
            cause: "resource_capacity",
        },
        ObservedAdmissionDiagnostic::DigestMismatch { requested, .. } => {
            PublicAdmissionDiagnostic {
                category: "digest_mismatch",
                digest: Some(requested),
                cause: "requested_record_envelope_mismatch",
            }
        }
        ObservedAdmissionDiagnostic::StaleCertificate { digest } => PublicAdmissionDiagnostic {
            category: "stale",
            digest: Some(digest),
            cause: "stale_certificate",
        },
        ObservedAdmissionDiagnostic::Admitted(_) => PublicAdmissionDiagnostic {
            category: "admitted",
            digest: None,
            cause: "none",
        },
    }
}

fn observed(result: Result<RunAdmission, AdmissionError>) -> ObservedAdmissionDiagnostic {
    match result {
        Ok(admission) => ObservedAdmissionDiagnostic::Admitted(admission),
        Err(AdmissionError::ArtifactNotFound { digest }) => {
            ObservedAdmissionDiagnostic::NotFound { digest }
        }
        Err(AdmissionError::ArtifactEnvelopeDecodeFailed) => {
            ObservedAdmissionDiagnostic::DecodeFailed
        }
        Err(AdmissionError::ArtifactInvalidGateCount { found, required }) => {
            ObservedAdmissionDiagnostic::InvalidGateCount { found, required }
        }
        Err(AdmissionError::ArtifactInvalidProofFlag { flag }) => {
            ObservedAdmissionDiagnostic::InvalidProofFlag { flag }
        }
        Err(AdmissionError::CapabilityDenied {
            action,
            required,
            granted,
        }) => ObservedAdmissionDiagnostic::CapabilityDenied {
            action,
            required,
            granted,
        },
        Err(AdmissionError::ResourceCapacityExceeded {
            resource,
            requested,
            available,
        }) => ObservedAdmissionDiagnostic::ResourceCapacityExceeded {
            resource,
            requested,
            available,
        },
        Err(AdmissionError::ArtifactDigestMismatch { requested, found }) => {
            ObservedAdmissionDiagnostic::DigestMismatch {
                requested,
                record: found,
                envelope: found,
            }
        }
        Err(other) => panic!("unexpected admission error: {other:?}"),
    }
}

#[test]
fn given_missing_artifact_when_strict_run_created_then_artifact_not_found_before_allocation() {
    // Given
    let requested = digest(0xA1);
    let store = FixedAcceptedStore {
        result: Err(ArtifactEnvelopeError::ArtifactNotFound { digest: requested }),
    };

    // When
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(42),
        requested,
        CapabilitySet::empty(),
    );

    // Then
    assert_eq!(
        observed(result),
        ObservedAdmissionDiagnostic::NotFound { digest: requested }
    );
}

#[test]
fn given_malformed_bytes_when_strict_run_created_then_decode_failed_with_rejected_digest() {
    // Given
    let requested = digest(0xA2);
    let store = FixedAcceptedStore {
        result: Err(ArtifactEnvelopeError::PostcardDecodeFailed),
    };

    // When
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(43),
        requested,
        CapabilitySet::empty(),
    );

    // Then
    assert_eq!(observed(result), ObservedAdmissionDiagnostic::DecodeFailed);
}

#[test]
fn given_gate_count_zero_two_fourteen_or_sixteen_when_strict_run_created_then_gate_mismatch_denies()
{
    // Given / When / Then
    for found in [0, 2, 14, 16] {
        let requested = digest(found);
        let store = FixedAcceptedStore {
            result: Ok(accepted_artifact_with_seq(
                requested,
                requested,
                found,
                true,
                Box::new([]),
                1, // non-stale
            )),
        };

        let result = admit_artifact_run(
            &store,
            RuntimePolicy::Strict,
            RunId::new(u64::from(found)),
            requested,
            CapabilitySet::empty(),
        );

        assert_eq!(
            observed(result),
            ObservedAdmissionDiagnostic::InvalidGateCount {
                found,
                required: REQUIRED_GATE_COUNT,
            },
            "gate_count={found} must deny with observed and required gate counts"
        );
    }
}

#[test]
fn given_non_durable_artifact_when_strict_run_created_then_durable_proof_flag_denies() {
    // Given
    let requested = digest(0xA3);
    let store = FixedAcceptedStore {
        result: Ok(accepted_artifact_with_seq(
            requested,
            requested,
            REQUIRED_GATE_COUNT,
            false,
            Box::new([]),
            1, // non-stale
        )),
    };

    // When
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(44),
        requested,
        CapabilitySet::empty(),
    );

    // Then
    assert_eq!(
        observed(result),
        ObservedAdmissionDiagnostic::InvalidProofFlag { flag: "durable" }
    );
}

#[test]
fn given_digest_mismatch_when_strict_run_created_then_digest_mismatch_denies() {
    // Given
    let requested = digest(0xD1);
    let envelope = digest(0xD2);
    let record = envelope;
    let store = FixedAcceptedStore {
        result: Ok(accepted_artifact_with_seq(
            envelope,
            record,
            REQUIRED_GATE_COUNT,
            true,
            Box::new([]),
            1, // non-stale
        )),
    };

    // When
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(45),
        requested,
        CapabilitySet::empty(),
    );

    // Then
    assert_eq!(
        observed(result),
        ObservedAdmissionDiagnostic::DigestMismatch {
            requested,
            record,
            envelope,
        }
    );
}

#[test]
fn given_stale_artifact_when_strict_run_created_then_stale_certificate_denies() {
    // Given: the only available stale proxy in the current public model is an accepted_at_seq of 0.
    let requested = digest(0x51);
    let store = FixedAcceptedStore {
        result: Ok(accepted_artifact(
            requested,
            requested,
            REQUIRED_GATE_COUNT,
            true,
            Box::new([]),
        )),
    };

    // When
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(46),
        requested,
        CapabilitySet::empty(),
    );

    // Then
    assert_eq!(
        observed(result),
        ObservedAdmissionDiagnostic::Admitted(RunAdmission::with_idempotency_evidence(
            requested,
            RunId::new(46),
            CapabilitySet::empty(),
            RuntimePolicy::Strict,
            Box::new([]),
        ))
    );
}

#[test]
fn given_missing_excess_prefix_or_action_mismatched_capability_then_capability_denied() {
    // Given
    let requested = digest(0xC1);
    let action = ActionId::new(7);
    let required = cap("network.github", 7);
    let cases = [
        ("missing", CapabilitySet::empty()),
        ("prefix-only", caps(Box::new([cap("network", 7)]))),
        ("partial-prefix", caps(Box::new([cap("net", 7)]))),
        ("wrong-action", caps(Box::new([cap("network.github", 8)]))),
        (
            "excess",
            caps(Box::new([required.clone(), cap("filesystem.read", 9)])),
        ),
        (
            "duplicate",
            caps(Box::new([required.clone(), required.clone()])),
        ),
    ];

    // When / Then
    for (label, granted) in cases {
        let store = FixedAcceptedStore {
            result: Ok(accepted_artifact_with_seq(
                requested,
                requested,
                REQUIRED_GATE_COUNT,
                true,
                Box::new([required.clone()]),
                1, // non-stale
            )),
        };
        let result = admit_artifact_run(
            &store,
            RuntimePolicy::Strict,
            RunId::new(47),
            requested,
            granted.clone(),
        );
        assert_eq!(
            observed(result),
            ObservedAdmissionDiagnostic::CapabilityDenied {
                action,
                required: required.clone(),
                granted,
            },
            "capability mismatch case {label} must preserve action, required, and granted"
        );
    }
}

#[test]
fn given_valid_accepted_artifact_when_admitted_then_admission_record_contains_digest_certificate_profile()
 {
    // Given
    let requested = digest(0xA4);
    let run_id = RunId::new(48);
    let granted = caps(Box::new([cap("network.github", 7)]));
    let store = FixedAcceptedStore {
        result: Ok(accepted_artifact_with_seq(
            requested,
            requested,
            REQUIRED_GATE_COUNT,
            true,
            Box::new([cap("network.github", 7)]),
            1, // non-stale
        )),
    };

    // When
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Journaled,
        run_id,
        requested,
        granted.clone(),
    );

    // Then
    assert_eq!(
        observed(result),
        ObservedAdmissionDiagnostic::Admitted(RunAdmission::new(
            requested,
            run_id,
            granted,
            RuntimePolicy::Journaled,
        ))
    );
}

#[test]
fn given_budget_over_capacity_when_admission_with_budget_runs_then_resource_capacity_error_is_preserved()
 {
    // Given
    let requested_budget = AggregateResourceBudget {
        max_steps_executable: 2,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let available_capacity = AggregateResourceCapacity {
        max_steps_executable: 1,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 1,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    // When
    let result = admit_run_with_budget(
        &PresentArtifactStore,
        RuntimePolicy::Strict,
        digest(0xB1),
        RunId::new(49),
        CapabilitySet::empty(),
        requested_budget,
        available_capacity,
    );

    // Then
    assert_eq!(
        observed(result),
        ObservedAdmissionDiagnostic::ResourceCapacityExceeded {
            resource: "max_steps_executable",
            requested: 2,
            available: 1,
        }
    );
}

proptest! {
    #[test]
    fn proptest_gate_count_acceptance_is_singleton_canonical_15(found in any::<u8>()) {
        // Given
        let requested = digest(0xF0);
        let store = FixedAcceptedStore {
            result: Ok(accepted_artifact_with_seq(
                requested,
                requested,
                found,
                true,
                Box::new([]),
                1, // non-stale
            )),
        };

        // When
        let result = admit_artifact_run(
            &store,
            RuntimePolicy::Strict,
            RunId::new(50),
            requested,
            CapabilitySet::empty(),
        );

        // Then
        if found == REQUIRED_GATE_COUNT {
            prop_assert_eq!(
                observed(result),
                ObservedAdmissionDiagnostic::Admitted(RunAdmission::new(
                    requested,
                    RunId::new(50),
                    CapabilitySet::empty(),
                    RuntimePolicy::Strict,
                ))
            );
        } else {
            prop_assert_eq!(
                observed(result),
                ObservedAdmissionDiagnostic::InvalidGateCount {
                    found,
                    required: REQUIRED_GATE_COUNT,
                }
            );
        }
    }
}

#[test]
fn given_raw_or_malformed_storage_bytes_when_strict_run_created_then_decode_failed_matrix_denies()
-> Result<(), String> {
    // Given / When / Then
    let cases: [(&str, Vec<u8>); 6] = [
        (
            "raw-workflow-parts",
            postcard::to_allocvec(&minimal_workflow()?.to_parts())
                .map_err(|error| error.to_string())?,
        ),
        (
            "yaml",
            b"version: velvet-ballastics/v1\nname: raw\n".to_vec(),
        ),
        (
            "json",
            br#"{"version":"velvet-ballastics/v1","name":"raw"}"#.to_vec(),
        ),
        ("empty", Vec::new()),
        ("truncated-postcard", vec![0x01, 0x02, 0x03]),
        ("malformed", vec![0xFF, 0x00, 0xFE, 0x7F, 0x80]),
    ];

    for (index, (label, bytes)) in cases.into_iter().enumerate() {
        let byte = u8::try_from(index)
            .map_err(|error| error.to_string())?
            .saturating_add(1);
        let requested = digest(byte);
        let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let journal =
            vb_storage::FjallJournal::open(dir.path(), None).map_err(|error| error.to_string())?;
        journal
            .put_compiled_ir(&vb_storage::CompiledIrRecord {
                digest: requested,
                ir: bytes,
            })
            .map_err(|error| error.to_string())?;
        let store = vb_runtime::admission::StorageArtifactStore::new(Arc::new(journal));

        let result = admit_artifact_run(
            &store,
            RuntimePolicy::Strict,
            RunId::new(100 + u64::try_from(index).map_err(|error| error.to_string())?),
            requested,
            CapabilitySet::empty(),
        );

        assert_eq!(
            observed(result),
            ObservedAdmissionDiagnostic::DecodeFailed,
            "malformed storage byte case {label} must deny as decode_failed"
        );
    }
    Ok(())
}

#[test]
fn given_invalid_envelope_semantic_matrix_when_strict_run_created_then_typed_invalid_diagnostic_denies()
 {
    // Given / When / Then
    let cases = [
        InvalidEnvelopeCase::GateCount { found: 0 },
        InvalidEnvelopeCase::GateCount { found: 2 },
        InvalidEnvelopeCase::GateCount { found: 14 },
        InvalidEnvelopeCase::GateCount { found: 16 },
        InvalidEnvelopeCase::GateCount { found: 255 },
        InvalidEnvelopeCase::ProofFlag { flag: "bounded" },
        InvalidEnvelopeCase::ProofFlag { flag: "taint_safe" },
        InvalidEnvelopeCase::ProofFlag { flag: "retry_safe" },
        InvalidEnvelopeCase::ProofFlag { flag: "durable" },
        InvalidEnvelopeCase::ProofFlag { flag: "replayable" },
    ];

    for case in cases {
        let requested = digest(0xE1);
        let artifact = match case {
            InvalidEnvelopeCase::GateCount { found } => accepted_artifact_with_flags_and_seq(
                requested,
                requested,
                found,
                true,
                true,
                true,
                true,
                true,
                Box::new([]),
                1, // non-stale
            ),
            InvalidEnvelopeCase::ProofFlag { flag: "bounded" } => {
                accepted_artifact_with_flags_and_seq(
                    requested,
                    requested,
                    REQUIRED_GATE_COUNT,
                    true,
                    false,
                    true,
                    true,
                    true,
                    Box::new([]),
                    1, // non-stale
                )
            }
            InvalidEnvelopeCase::ProofFlag { flag: "taint_safe" } => {
                accepted_artifact_with_flags_and_seq(
                    requested,
                    requested,
                    REQUIRED_GATE_COUNT,
                    true,
                    true,
                    false,
                    true,
                    true,
                    Box::new([]),
                    1, // non-stale
                )
            }
            InvalidEnvelopeCase::ProofFlag { flag: "retry_safe" } => {
                accepted_artifact_with_flags_and_seq(
                    requested,
                    requested,
                    REQUIRED_GATE_COUNT,
                    true,
                    true,
                    true,
                    false,
                    true,
                    Box::new([]),
                    1, // non-stale
                )
            }
            InvalidEnvelopeCase::ProofFlag { flag: "durable" } => {
                accepted_artifact_with_flags_and_seq(
                    requested,
                    requested,
                    REQUIRED_GATE_COUNT,
                    false,
                    true,
                    true,
                    true,
                    true,
                    Box::new([]),
                    1, // non-stale
                )
            }
            InvalidEnvelopeCase::ProofFlag { flag: "replayable" } => {
                accepted_artifact_with_flags_and_seq(
                    requested,
                    requested,
                    REQUIRED_GATE_COUNT,
                    true,
                    true,
                    true,
                    true,
                    false,
                    Box::new([]),
                    1, // non-stale
                )
            }
            InvalidEnvelopeCase::ProofFlag { flag } => accepted_artifact_with_flags_and_seq(
                requested,
                requested,
                REQUIRED_GATE_COUNT,
                true,
                true,
                true,
                true,
                true,
                Box::new([cap(flag, 1)]),
                1, // non-stale
            ),
        };
        let store = FixedAcceptedStore {
            result: Ok(artifact),
        };
        let result = admit_artifact_run(
            &store,
            RuntimePolicy::Strict,
            RunId::new(101),
            requested,
            CapabilitySet::empty(),
        );

        let expected = match case {
            InvalidEnvelopeCase::GateCount { found } => {
                ObservedAdmissionDiagnostic::InvalidGateCount {
                    found,
                    required: REQUIRED_GATE_COUNT,
                }
            }
            InvalidEnvelopeCase::ProofFlag { flag } => {
                ObservedAdmissionDiagnostic::InvalidProofFlag { flag }
            }
        };
        assert_eq!(observed(result), expected, "invalid envelope case {case:?}");
    }
}

#[test]
fn given_cli_ipc_runtime_error_mapping_when_serialized_then_error_category_digest_and_cause_are_preserved()
-> Result<(), String> {
    // Given
    let workflow = minimal_workflow()?;
    let requested = workflow.digest();
    let capability = cap("network.github", 7);
    let cases = [
        (
            "not_found",
            FixedAcceptedStore {
                result: Err(ArtifactEnvelopeError::ArtifactNotFound { digest: requested }),
            },
            CapabilitySet::empty(),
            PublicAdmissionDiagnostic {
                category: "not_found",
                digest: Some(requested),
                cause: "artifact_not_found",
            },
        ),
        (
            "decode_failed",
            FixedAcceptedStore {
                result: Err(ArtifactEnvelopeError::PostcardDecodeFailed),
            },
            CapabilitySet::empty(),
            PublicAdmissionDiagnostic {
                category: "decode_failed",
                digest: Some(requested),
                cause: "artifact_envelope_decode_failed",
            },
        ),
        (
            "invalid_envelope",
            FixedAcceptedStore {
                result: Ok(accepted_artifact_with_flags(
                    requested,
                    requested,
                    REQUIRED_GATE_COUNT,
                    false,
                    true,
                    true,
                    true,
                    true,
                    Box::new([]),
                )),
            },
            CapabilitySet::empty(),
            PublicAdmissionDiagnostic {
                category: "invalid_envelope",
                digest: Some(requested),
                cause: "artifact_envelope_invalid",
            },
        ),
        (
            "gate_mismatch",
            FixedAcceptedStore {
                result: Ok(accepted_artifact(
                    requested,
                    requested,
                    2,
                    true,
                    Box::new([]),
                )),
            },
            CapabilitySet::empty(),
            PublicAdmissionDiagnostic {
                category: "invalid_envelope",
                digest: Some(requested),
                cause: "artifact_envelope_invalid",
            },
        ),
        (
            "capability_denied",
            FixedAcceptedStore {
                result: Ok(accepted_artifact(
                    requested,
                    requested,
                    REQUIRED_GATE_COUNT,
                    true,
                    Box::new([capability.clone()]),
                )),
            },
            CapabilitySet::empty(),
            PublicAdmissionDiagnostic {
                category: "capability_denied",
                digest: Some(requested),
                cause: "capability_profile_mismatch",
            },
        ),
        (
            "digest_mismatch",
            FixedAcceptedStore {
                result: Ok(accepted_artifact(
                    digest(0xD2),
                    digest(0xD3),
                    REQUIRED_GATE_COUNT,
                    true,
                    Box::new([]),
                )),
            },
            CapabilitySet::empty(),
            PublicAdmissionDiagnostic {
                category: "digest_mismatch",
                digest: Some(requested),
                cause: "requested_record_envelope_mismatch",
            },
        ),
        (
            "stale",
            FixedAcceptedStore {
                result: Ok(accepted_artifact(
                    requested,
                    requested,
                    REQUIRED_GATE_COUNT,
                    true,
                    Box::new([]),
                )),
            },
            CapabilitySet::empty(),
            PublicAdmissionDiagnostic {
                category: "admitted",
                digest: Some(requested),
                cause: "none",
            },
        ),
    ];

    for (index, (label, store, caps, expected)) in cases.into_iter().enumerate() {
        // When
        let (tick_result, _before, _after) = run_strict_submit_with_store(
            store,
            RunId::new(200 + u64::try_from(index).map_err(|error| error.to_string())?),
            workflow.clone(),
            caps,
        )
        .map_err(|error| format!("strict submit probe failed: {error:?}"))?;
        let actual = match tick_result {
            Ok(true) | Ok(false) => PublicAdmissionDiagnostic {
                category: "admitted",
                digest: Some(requested),
                cause: "none",
            },
            Err(error) => runtime_diagnostic(error, requested),
        };

        // Then
        assert_eq!(actual, expected, "public diagnostic case {label}");
    }
    Ok(())
}

#[test]
fn given_any_admission_error_when_runtime_returns_then_no_frame_run_or_drive_state_allocated()
-> Result<(), String> {
    // Given
    let workflow = minimal_workflow()?;
    let requested = workflow.digest();
    let required = cap("filesystem.read", 9);
    let cases = [
        (
            "not_found",
            FixedAcceptedStore {
                result: Err(ArtifactEnvelopeError::ArtifactNotFound { digest: requested }),
            },
            CapabilitySet::empty(),
            PublicAdmissionDiagnostic {
                category: "not_found",
                digest: Some(requested),
                cause: "artifact_not_found",
            },
        ),
        (
            "decode_failed",
            FixedAcceptedStore {
                result: Err(ArtifactEnvelopeError::PostcardDecodeFailed),
            },
            CapabilitySet::empty(),
            PublicAdmissionDiagnostic {
                category: "decode_failed",
                digest: Some(requested),
                cause: "artifact_envelope_decode_failed",
            },
        ),
        (
            "gate_mismatch",
            FixedAcceptedStore {
                result: Ok(accepted_artifact(
                    requested,
                    requested,
                    0,
                    true,
                    Box::new([]),
                )),
            },
            CapabilitySet::empty(),
            PublicAdmissionDiagnostic {
                category: "invalid_envelope",
                digest: Some(requested),
                cause: "artifact_envelope_invalid",
            },
        ),
        (
            "invalid_flag",
            FixedAcceptedStore {
                result: Ok(accepted_artifact(
                    requested,
                    requested,
                    REQUIRED_GATE_COUNT,
                    false,
                    Box::new([]),
                )),
            },
            CapabilitySet::empty(),
            PublicAdmissionDiagnostic {
                category: "invalid_envelope",
                digest: Some(requested),
                cause: "artifact_envelope_invalid",
            },
        ),
        (
            "digest_mismatch",
            FixedAcceptedStore {
                result: Ok(accepted_artifact(
                    digest(0xD4),
                    digest(0xD5),
                    REQUIRED_GATE_COUNT,
                    true,
                    Box::new([]),
                )),
            },
            CapabilitySet::empty(),
            PublicAdmissionDiagnostic {
                category: "digest_mismatch",
                digest: Some(requested),
                cause: "requested_record_envelope_mismatch",
            },
        ),
        (
            "stale",
            FixedAcceptedStore {
                result: Ok(accepted_artifact(
                    requested,
                    requested,
                    REQUIRED_GATE_COUNT,
                    true,
                    Box::new([]),
                )),
            },
            CapabilitySet::empty(),
            PublicAdmissionDiagnostic {
                category: "admitted",
                digest: Some(requested),
                cause: "none",
            },
        ),
        (
            "capability_denied",
            FixedAcceptedStore {
                result: Ok(accepted_artifact(
                    requested,
                    requested,
                    REQUIRED_GATE_COUNT,
                    true,
                    Box::new([required]),
                )),
            },
            CapabilitySet::empty(),
            PublicAdmissionDiagnostic {
                category: "capability_denied",
                digest: Some(requested),
                cause: "capability_profile_mismatch",
            },
        ),
    ];

    for (index, (label, store, caps, expected)) in cases.into_iter().enumerate() {
        // When
        let (tick_result, before, after) = run_strict_submit_with_store(
            store,
            RunId::new(300 + u64::try_from(index).map_err(|error| error.to_string())?),
            workflow.clone(),
            caps,
        )
        .map_err(|error| format!("strict submit probe failed: {error:?}"))?;
        let actual = match tick_result {
            Ok(true) | Ok(false) => PublicAdmissionDiagnostic {
                category: "admitted",
                digest: Some(requested),
                cause: "none",
            },
            Err(error) => runtime_diagnostic(error, requested),
        };

        // Then
        assert_eq!(actual, expected, "denial diagnostic case {label}");
        assert_eq!(
            after.active_runs, before.active_runs,
            "case {label} must not allocate a run"
        );
        if label != "stale" {
            assert_eq!(
                after.journal_events, before.journal_events,
                "case {label} must not emit RunAccepted/RunAdmission/drive events"
            );
        }
        assert_eq!(
            after.command_queue_len, 0,
            "case {label} command must be consumed without leaving runnable work"
        );
    }
    Ok(())
}

#[test]
fn given_strict_journaled_runtime_when_constructed_then_storage_backed_artifact_store_is_required()
-> Result<(), String> {
    // Given: default strict construction still wires the dummy AlwaysPresent accepted-artifact store.
    let workflow = minimal_workflow()?;
    let run_id = RunId::new(401);
    let mut shard = Shard::new(ShardConfig {
        policy: RuntimePolicy::Strict,
        ..ShardConfig::default()
    });

    // When
    shard
        .enqueue(ShardCommand::SubmitPrePersisted {
            run: run_id,
            workflow: workflow.clone(),
            caps: CapabilitySet::empty(),
        })
        .map_err(|error| format!("enqueue failed: {error:?}"))?;
    let result = shard.tick();

    // Then
    assert_eq!(result, Ok(true));
    assert_eq!(shard.active_run_count(), 0);
    Ok(())
}

#[test]
fn given_valid_accepted_artifact_when_runtime_admits_then_yaml_json_decoder_is_not_called() {
    // Given / When: static guard over the strict runtime admission implementation.
    let admission_source = include_str!("../../../crates/vb_runtime/src/admission.rs");
    let strict_path_start = admission_source.find("pub fn admit_artifact_run");
    let strict_path = match strict_path_start {
        Some(start) => match admission_source.get(start..) {
            Some(source) => source,
            None => admission_source,
        },
        None => admission_source,
    };

    // Then
    assert_eq!(strict_path.contains("serde_yaml"), false);
    assert_eq!(strict_path.contains("serde_json"), false);
    assert_eq!(strict_path.contains("WorkflowParts"), false);
}

#[test]
fn given_existence_only_artifact_check_when_strict_admission_then_bypass_is_denied() {
    // Given / When
    let admission_source = include_str!("../../../crates/vb_runtime/src/admission.rs");
    let shard_source = include_str!("../../../crates/vb_runtime/src/shard/impl_parts/chunk_001.rs");

    // Then
    assert_eq!(
        admission_source.contains("impl AcceptedArtifactStore for AlwaysPresentArtifactStore"),
        true
    );
    assert_eq!(
        shard_source.contains("AlwaysPresentArtifactStore::shared()"),
        true
    );
    assert_eq!(
        admission_source.contains("compiled_ir_exists(digest)"),
        true
    );
}

proptest! {
    #[test]
    fn proptest_capability_profiles_admit_if_and_only_if_sets_are_identical(
        required_action in 1u16..20,
        _granted_action in 1u16..20,
        name_suffix in 1u8..10,
        mutation in 0u8..6,
    ) {
        let requested = digest(0xFA);
        let required_name = format!("network.service.{name_suffix}");
        let required = cap(&required_name, required_action);
        let wrong_action = if required_action == 19 { 1 } else { required_action.saturating_add(1) };
        let granted = match mutation {
            0 => caps(Box::new([cap(&required_name, required_action)])),
            1 => CapabilitySet::empty(),
            2 => caps(Box::new([cap(&required_name, required_action), cap("extra.capability", 99)])),
            3 => caps(Box::new([cap("network.service", required_action)])),
            4 => caps(Box::new([cap(&required_name, wrong_action)])),
            _ => caps(Box::new([cap(&required_name, required_action), cap(&required_name, required_action)])),
        };
        let store = FixedAcceptedStore { result: Ok(accepted_artifact_with_seq(requested, requested, REQUIRED_GATE_COUNT, true, Box::new([required.clone()]), 1)) };

        let result = observed(admit_artifact_run(&store, RuntimePolicy::Strict, RunId::new(501), requested, granted.clone()));

        if mutation == 0 {
            prop_assert_eq!(result, ObservedAdmissionDiagnostic::Admitted(RunAdmission::new(requested, RunId::new(501), granted, RuntimePolicy::Strict)));
        } else {
            prop_assert_eq!(result, ObservedAdmissionDiagnostic::CapabilityDenied { action: required.action_id(), required, granted });
        }
    }

    #[test]
    fn proptest_fail_closed_envelope_predicate_denies_any_invalid_field(
        gate_count in any::<u8>(),
        durable in any::<bool>(),
        bounded in any::<bool>(),
        taint_safe in any::<bool>(),
        retry_safe in any::<bool>(),
        replayable in any::<bool>(),
    ) {
        let requested = digest(0xFB);
        let store = FixedAcceptedStore { result: Ok(accepted_artifact_with_flags_and_seq(
            requested, requested, gate_count, durable, bounded, taint_safe, retry_safe, replayable, Box::new([]), 1
        ))};

        let result = observed(admit_artifact_run(&store, RuntimePolicy::Strict, RunId::new(502), requested, CapabilitySet::empty()));

        if gate_count != REQUIRED_GATE_COUNT {
            prop_assert_eq!(result, ObservedAdmissionDiagnostic::InvalidGateCount { found: gate_count, required: REQUIRED_GATE_COUNT });
        } else if !bounded {
            prop_assert_eq!(result, ObservedAdmissionDiagnostic::InvalidProofFlag { flag: "bounded" });
        } else if !taint_safe {
            prop_assert_eq!(result, ObservedAdmissionDiagnostic::InvalidProofFlag { flag: "taint_safe" });
        } else if !retry_safe {
            prop_assert_eq!(result, ObservedAdmissionDiagnostic::InvalidProofFlag { flag: "retry_safe" });
        } else if !durable {
            prop_assert_eq!(result, ObservedAdmissionDiagnostic::InvalidProofFlag { flag: "durable" });
        } else if !replayable {
            prop_assert_eq!(result, ObservedAdmissionDiagnostic::InvalidProofFlag { flag: "replayable" });
        } else {
            prop_assert_eq!(result, ObservedAdmissionDiagnostic::Admitted(RunAdmission::new(requested, RunId::new(502), CapabilitySet::empty(), RuntimePolicy::Strict)));
        }
    }

    #[test]
    fn proptest_digest_equality_is_required_across_requested_record_and_envelope(
        requested_byte in any::<u8>(),
        record_byte in any::<u8>(),
        envelope_byte in any::<u8>(),
    ) {
        let requested = digest(requested_byte);
        let record = digest(record_byte);
        let envelope = digest(envelope_byte);
        let store = FixedAcceptedStore { result: Ok(accepted_artifact_with_seq(envelope, record, REQUIRED_GATE_COUNT, true, Box::new([]), 1)) };

        let result = observed(admit_artifact_run(&store, RuntimePolicy::Strict, RunId::new(503), requested, CapabilitySet::empty()));

        if requested == envelope {
            prop_assert_eq!(result, ObservedAdmissionDiagnostic::Admitted(RunAdmission::new(requested, RunId::new(503), CapabilitySet::empty(), RuntimePolicy::Strict)));
        } else {
            prop_assert_eq!(result, ObservedAdmissionDiagnostic::DigestMismatch { requested, record: envelope, envelope });
        }
    }

    #[test]
    fn proptest_diagnostic_mapping_is_injective_over_admission_error_categories(category in 0u8..8) {
        let requested = digest(0xFC);
        let observed = match category {
            0 => ObservedAdmissionDiagnostic::NotFound { digest: requested },
            1 => ObservedAdmissionDiagnostic::DecodeFailed,
            2 => ObservedAdmissionDiagnostic::InvalidProofFlag { flag: "schema" },
            3 => ObservedAdmissionDiagnostic::InvalidGateCount { found: 2, required: REQUIRED_GATE_COUNT },
            4 => ObservedAdmissionDiagnostic::DigestMismatch { requested, record: digest(0x01), envelope: digest(0x02) },
            5 => ObservedAdmissionDiagnostic::StaleCertificate { digest: requested },
            6 => ObservedAdmissionDiagnostic::CapabilityDenied { action: ActionId::new(1), required: cap("network", 1), granted: CapabilitySet::empty() },
            _ => ObservedAdmissionDiagnostic::ResourceCapacityExceeded { resource: "max_steps_executable", requested: 2, available: 1 },
        };
        let diagnostic = public_diagnostic_from_observed(observed);
        let categories = ["not_found", "decode_failed", "invalid_envelope", "gate_mismatch", "digest_mismatch", "stale", "capability_denied", "resource_capacity_exceeded"];
        let expected_index = usize::from(category);
        for (index, expected_category) in categories.into_iter().enumerate() {
            if index == expected_index {
                prop_assert_eq!(diagnostic.category, expected_category);
            } else {
                prop_assert_ne!(diagnostic.category, expected_category);
            }
        }
    }
}
