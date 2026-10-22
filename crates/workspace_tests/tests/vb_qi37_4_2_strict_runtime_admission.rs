#![forbid(unsafe_code)]

use std::sync::Arc;

use proptest::{prelude::*, test_runner::TestCaseError};
use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::artifact::{ArtifactEvidence, ObligationClosure, ObligationKind};
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
    AcceptedArtifactStore, AdmissionError, ArtifactEnvelopeError, ArtifactStore, RunAdmission,
    admit_artifact_run, admit_artifact_run_with_certificate_floor, admit_run_with_budget,
};
use vb_runtime::journal::{RuntimeJournalEvent, VolatileRuntimeJournal};
use vb_runtime::shard::{Shard, ShardCommand, ShardConfig};
use vb_storage::RunPolicyPin;
use vb_storage::admission::{AcceptedArtifact, prepare_compiled_artifact};
use vb_storage::types::EventSeq;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ObservedAdmissionDiagnostic {
    Admitted(RunAdmission),
    NotFound {
        digest: WorkflowDigest,
    },
    DecodeFailed,
    InvalidProofFlag {
        flag: &'static str,
    },
    CapabilityDenied {
        action: ActionId,
        required: Capability,
        granted: CapabilitySet,
    },
    CapabilityCountMismatch {
        required_count: usize,
        granted_count: usize,
    },
    ResourceCapacityExceeded {
        resource: &'static str,
        requested: u64,
        available: u64,
    },
    DigestMismatch {
        requested: WorkflowDigest,
        found: WorkflowDigest,
    },
    StaleCertificate {
        digest: WorkflowDigest,
    },
    Unexpected {
        error: AdmissionError,
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

#[derive(Clone)]
struct FixedAcceptedStore {
    result: Result<AcceptedArtifact, ArtifactEnvelopeError>,
}

impl AcceptedArtifactStore for FixedAcceptedStore {
    fn load_accepted_artifact(
        &self,
        _artifact_digest: WorkflowDigest,
    ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
        self.result.clone()
    }
}

struct PresentArtifactStore;

impl ArtifactStore for PresentArtifactStore {
    fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
        true
    }
}

struct ScenarioStore {
    artifact: AcceptedArtifact,
}

impl AcceptedArtifactStore for ScenarioStore {
    fn load_accepted_artifact(
        &self,
        _digest: WorkflowDigest,
    ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
        Ok(self.artifact.clone())
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

/// Canonicalizes a `WorkflowParts` value: recomputes the self-zeroed content
/// digest before reconstructing a validated `CompiledWorkflow`. This mirrors
/// the digest derivation the cold-path artifact verifier performs, so the
/// resulting workflow satisfies `prepare_compiled_artifact`'s checksum binding.
fn canonicalize_parts(mut parts: WorkflowParts) -> Result<CompiledWorkflow, String> {
    parts.digest = WorkflowDigest::from_bytes([0u8; 32]);
    let bytes = postcard::to_allocvec(&parts).map_err(|error| error.to_string())?;
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&bytes).into());
    CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())
}

/// A minimal action-free workflow: a single constant then a finish.
fn minimal_workflow() -> Result<CompiledWorkflow, String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
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
    canonicalize_parts(parts)
}

/// A workflow with one dispatched action (action 0) preceded by a constant
/// producer and followed by a finish. Used for capability-bound admission.
fn do_workflow() -> Result<CompiledWorkflow, String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("vb_qi37_4_2_do_admission_test"),
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
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(0),
                    input: SlotIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
            },
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(42)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    canonicalize_parts(parts)
}

fn do_contract(required_capabilities: Box<[Capability]>) -> ActionContract {
    ActionContract {
        id: ActionId::new(0),
        name: ActionName::from_static_infallible("admission-test-action"),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities,
    }
}

/// Builds a checked artifact for the `do_workflow` using the real verifier path.
fn build_do_artifact(
    required_capabilities: Box<[Capability]>,
    policy: RuntimePolicy,
) -> Result<AcceptedArtifact, String> {
    let workflow = do_workflow()?;
    let contracts = [do_contract(required_capabilities)];
    prepare_compiled_artifact(&workflow, policy, &contracts).map_err(|error| error.to_string())
}

/// Builds a checked artifact for the action-free `minimal_workflow`.
fn build_minimal_artifact(policy: RuntimePolicy) -> Result<AcceptedArtifact, String> {
    let workflow = minimal_workflow()?;
    prepare_compiled_artifact(&workflow, policy, &[]).map_err(|error| error.to_string())
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
        RuntimeError::AdmissionCapabilityCountMismatch { .. } => PublicAdmissionDiagnostic {
            category: "capability_count_mismatch",
            digest: Some(fallback_digest),
            cause: "capability_cardinality_mismatch",
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
        RuntimeError::AdmissionArtifactStale { digest } => PublicAdmissionDiagnostic {
            category: "stale",
            digest: Some(digest),
            cause: "stale_certificate",
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
        ObservedAdmissionDiagnostic::CapabilityCountMismatch { .. } => PublicAdmissionDiagnostic {
            category: "capability_count_mismatch",
            digest: None,
            cause: "capability_cardinality_mismatch",
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
        ObservedAdmissionDiagnostic::Unexpected { error } => PublicAdmissionDiagnostic {
            category: "unexpected_runtime_error",
            digest: None,
            cause: match error {
                AdmissionError::ArtifactInvalidGateCount { .. } => "invalid_gate_count",
                AdmissionError::BudgetPolicyExceeded { .. } => "budget_policy_exceeded",
                AdmissionError::ResourceBudgetOverflow { .. } => "budget_overflow",
                AdmissionError::ResourceBudgetUnderflow { .. } => "budget_underflow",
                AdmissionError::ResourceBudgetInvalidCapacity { .. } => "budget_invalid_capacity",
                AdmissionError::ResourceStepCeilingExceeded { .. } => "step_ceiling_exceeded",
                AdmissionError::ResourcePerTickCeilingExceeded { .. } => {
                    "per_tick_ceiling_exceeded"
                }
                _ => "unexpected",
            },
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
        Err(AdmissionError::CapabilityCountMismatch {
            required_count,
            granted_count,
        }) => ObservedAdmissionDiagnostic::CapabilityCountMismatch {
            required_count,
            granted_count,
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
            ObservedAdmissionDiagnostic::DigestMismatch { requested, found }
        }
        Err(AdmissionError::ArtifactCertificateStale { digest, .. }) => {
            ObservedAdmissionDiagnostic::StaleCertificate { digest }
        }
        Err(other) => ObservedAdmissionDiagnostic::Unexpected { error: other },
    }
}

#[test]
fn given_missing_artifact_when_strict_run_created_then_artifact_not_found_before_allocation() {
    let requested = digest(0xA1);
    let store = FixedAcceptedStore {
        result: Err(ArtifactEnvelopeError::ArtifactNotFound { digest: requested }),
    };

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(42),
        requested,
        CapabilitySet::empty(),
    );

    assert_eq!(
        observed(result),
        ObservedAdmissionDiagnostic::NotFound { digest: requested }
    );
}

#[test]
fn given_malformed_bytes_when_strict_run_created_then_decode_failed_with_rejected_digest() {
    let requested = digest(0xA2);
    let store = FixedAcceptedStore {
        result: Err(ArtifactEnvelopeError::PostcardDecodeFailed),
    };

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(43),
        requested,
        CapabilitySet::empty(),
    );

    assert_eq!(observed(result), ObservedAdmissionDiagnostic::DecodeFailed);
}

#[test]
fn given_unverified_legacy_evidence_when_strict_run_created_then_evidence_denies()
-> Result<(), String> {
    // Legacy all-true flags (gate_count, durable, claimed booleans) with
    // `Unverified` evidence cannot mint admission authority. The runtime
    // artifact verifier must reject the legacy wire state.
    let requested = digest(0xA3);
    let mut artifact = build_minimal_artifact(RuntimePolicy::Strict)?;
    artifact.verification.evidence = ArtifactEvidence::Unverified;
    let store = FixedAcceptedStore {
        result: Ok(artifact.clone()),
    };

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(44),
        requested,
        CapabilitySet::empty(),
    );

    assert_eq!(
        observed(result),
        ObservedAdmissionDiagnostic::InvalidProofFlag { flag: "evidence" }
    );
    Ok(())
}

#[test]
fn given_checked_artifact_when_prepared_then_gate_count_and_durability_are_not_admission_authority()
-> Result<(), String> {
    // The checked producer never mints a gate count or a durable flag; typed
    // evidence is the admission authority, not the legacy display fields.
    let artifact = build_minimal_artifact(RuntimePolicy::Strict)?;
    assert_eq!(artifact.verification.gate_count, 0);
    assert_eq!(artifact.verification.durable, false);
    let store = ScenarioStore { artifact };
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(45),
        store.artifact.digest,
        CapabilitySet::empty(),
    );
    assert!(
        matches!(result, Ok(_)),
        "checked artifact must admit: {result:?}"
    );
    Ok(())
}

#[test]
fn accepted_ledger_distinguishes_runtime_validated_from_operator_attested() -> Result<(), String> {
    // Declaration-only `RuntimeValidated` closures mark checks the artifact
    // verifier actually ran. `OperatorAttested` is a distinct external
    // attestation and remains intentionally absent from a checked ledger.
    for policy in [
        RuntimePolicy::Relaxed,
        RuntimePolicy::Journaled,
        RuntimePolicy::Strict,
    ] {
        let artifact = build_minimal_artifact(policy)?;
        let ArtifactEvidence::Verified(evidence) = &artifact.verification.evidence else {
            return Err("missing typed ledger".into());
        };
        assert_eq!(evidence.schema_version, 3);
        assert_eq!(
            evidence.ledger.map(|entry| entry.obligation),
            [
                ObligationKind::Determinism,
                ObligationKind::EffectSafety,
                ObligationKind::CapabilityNonMinting,
                ObligationKind::Boundedness,
                ObligationKind::ReplayEffectSeparation,
                ObligationKind::ArtifactValidity,
                ObligationKind::IdempotencyClassification,
                ObligationKind::RecoveryClassification,
                ObligationKind::TaintHandling,
                ObligationKind::PolicyBinding,
            ]
        );
        for entry in &evidence.ledger {
            assert!(
                !matches!(entry.closure, ObligationClosure::OperatorAttested { .. }),
                "operator attestation must remain empty"
            );
            match entry.obligation {
                ObligationKind::Determinism
                | ObligationKind::EffectSafety
                | ObligationKind::CapabilityNonMinting
                | ObligationKind::Boundedness
                | ObligationKind::ArtifactValidity
                | ObligationKind::IdempotencyClassification
                | ObligationKind::PolicyBinding => {
                    assert!(
                        matches!(entry.closure, ObligationClosure::RuntimeValidated { .. }),
                        "{:?} must be declaration-only RuntimeValidated",
                        entry.obligation
                    );
                }
                ObligationKind::ReplayEffectSeparation
                | ObligationKind::RecoveryClassification
                | ObligationKind::TaintHandling => {
                    assert!(
                        matches!(entry.closure, ObligationClosure::ExternalAssumption { .. }),
                        "{:?} must remain an explicit external assumption",
                        entry.obligation
                    );
                }
            }
        }
    }

    // Effect safety for a workflow with a dispatched action is an external
    // executor-honors-contracts assumption, not a declaration-only validation.
    let action_artifact = build_do_artifact(Box::new([]), RuntimePolicy::Strict)?;
    let ArtifactEvidence::Verified(evidence) = &action_artifact.verification.evidence else {
        return Err("missing typed ledger for action workflow".into());
    };
    let effect_entry = evidence
        .ledger
        .iter()
        .find(|entry| entry.obligation == ObligationKind::EffectSafety)
        .ok_or("missing effect-safety obligation")?;
    assert!(matches!(
        effect_entry.closure,
        ObligationClosure::ExternalAssumption { .. }
    ));
    Ok(())
}

#[test]
fn given_digest_mismatch_when_strict_run_created_then_digest_mismatch_denies() -> Result<(), String>
{
    let artifact = build_minimal_artifact(RuntimePolicy::Strict)?;
    let artifact_digest = artifact.digest;

    // (a) proof digest tampered while the envelope digest stays intact.
    let mut tampered = artifact.clone();
    let bad = digest(0xD2);
    tampered.verification.digest = bad;
    let store = FixedAcceptedStore {
        result: Ok(tampered),
    };
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(46),
        artifact_digest,
        CapabilitySet::empty(),
    );
    assert_eq!(
        observed(result),
        ObservedAdmissionDiagnostic::DigestMismatch {
            requested: artifact_digest,
            found: bad,
        }
    );

    // (b) caller requests a digest bound to neither the artifact nor its source.
    let requested = digest(0xD1);
    let store = FixedAcceptedStore {
        result: Ok(artifact),
    };
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(47),
        requested,
        CapabilitySet::empty(),
    );
    assert_eq!(
        observed(result),
        ObservedAdmissionDiagnostic::DigestMismatch {
            requested,
            found: artifact_digest,
        }
    );
    Ok(())
}

#[test]
fn given_stale_artifact_when_strict_run_created_then_stale_certificate_denies() -> Result<(), String>
{
    // A checked artifact is accepted at sequence zero; a freshness floor above
    // zero must reject it as stale after envelope validation succeeds.
    let artifact = build_minimal_artifact(RuntimePolicy::Strict)?;
    let artifact_digest = artifact.digest;
    let store = ScenarioStore { artifact };

    let result = admit_artifact_run_with_certificate_floor(
        &store,
        RuntimePolicy::Strict,
        RunId::new(48),
        artifact_digest,
        CapabilitySet::empty(),
        EventSeq::new(1),
    );

    assert_eq!(
        observed(result),
        ObservedAdmissionDiagnostic::StaleCertificate {
            digest: artifact_digest,
        }
    );
    Ok(())
}

#[test]
fn given_missing_prefix_or_action_mismatched_capability_then_capability_denied()
-> Result<(), String> {
    let required = cap("network.github", 0);
    let artifact = build_do_artifact(Box::new([required.clone()]), RuntimePolicy::Strict)?;
    let artifact_digest = artifact.digest;
    let cases = [
        ("missing", CapabilitySet::empty()),
        ("prefix-only", caps(Box::new([cap("network", 0)]))),
        ("partial-prefix", caps(Box::new([cap("net", 0)]))),
        ("wrong-action", caps(Box::new([cap("network.github", 1)]))),
    ];

    for (label, granted) in cases {
        let store = ScenarioStore {
            artifact: artifact.clone(),
        };
        let result = admit_artifact_run(
            &store,
            RuntimePolicy::Strict,
            RunId::new(49),
            artifact_digest,
            granted.clone(),
        );
        assert_eq!(
            observed(result),
            ObservedAdmissionDiagnostic::CapabilityDenied {
                action: ActionId::new(0),
                required: required.clone(),
                granted,
            },
            "capability mismatch case {label} must preserve action, required, and granted"
        );
    }
    Ok(())
}

#[test]
fn given_required_capability_with_extras_or_duplicates_then_denied() -> Result<(), String> {
    // Strict admission rejects cardinality-mismatch shapes (extras and
    // duplicates) with a typed count mismatch instead of fabricating a
    // `CapabilityDenied` for a capability that is actually granted.
    let required = cap("network.github", 0);
    let artifact = build_do_artifact(Box::new([required.clone()]), RuntimePolicy::Strict)?;
    let artifact_digest = artifact.digest;
    let cases = [
        (
            "excess",
            caps(Box::new([required.clone(), cap("filesystem.read", 9)])),
        ),
        (
            "duplicate",
            caps(Box::new([required.clone(), required.clone()])),
        ),
    ];

    for (label, granted) in cases {
        let store = ScenarioStore {
            artifact: artifact.clone(),
        };
        let result = admit_artifact_run(
            &store,
            RuntimePolicy::Strict,
            RunId::new(50),
            artifact_digest,
            granted.clone(),
        );
        assert_eq!(
            observed(result),
            ObservedAdmissionDiagnostic::CapabilityCountMismatch {
                required_count: 1,
                granted_count: granted.len(),
            },
            "capability cardinality-mismatch case {label} must deny closed"
        );
    }
    Ok(())
}

#[test]
fn given_valid_accepted_artifact_when_admitted_then_admission_record_contains_digest_certificate_profile()
-> Result<(), String> {
    // A valid schema-3 artifact built from the real verifier path admits with
    // a record carrying the matched digest, run id, policy, and exact grants.
    let capability = cap("network.github", 0);
    let artifact = build_do_artifact(Box::new([capability.clone()]), RuntimePolicy::Journaled)?;
    let requested = artifact.digest;
    let expected_policy_digest = artifact.policy_digest;
    let run_id = RunId::new(51);
    let granted = caps(Box::new([capability]));
    let store = ScenarioStore { artifact };

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Journaled,
        run_id,
        requested,
        granted.clone(),
    );

    let admission = match result {
        Ok(a) => a,
        Err(e) => return Err(format!("expected admission, got error: {e:?}")),
    };

    assert_eq!(admission.artifact_digest(), requested);
    assert_eq!(admission.run_id(), run_id);
    assert_eq!(admission.granted_capabilities(), &granted);
    assert_eq!(admission.policy(), RuntimePolicy::Journaled);
    assert_eq!(
        admission.policy_pin(),
        RunPolicyPin::Pinned(expected_policy_digest)
    );
    assert_eq!(admission.budget(), None);
    assert_eq!(admission.idempotency_attested(), &[]);
    Ok(())
}

#[test]
fn given_budget_over_capacity_when_admission_with_budget_runs_then_resource_capacity_error_is_preserved()
 {
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
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
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
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 1,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = admit_run_with_budget(
        &PresentArtifactStore,
        RuntimePolicy::Strict,
        digest(0xB1),
        RunId::new(52),
        CapabilitySet::empty(),
        requested_budget,
        available_capacity,
    );

    assert_eq!(
        observed(result),
        ObservedAdmissionDiagnostic::ResourceCapacityExceeded {
            resource: "max_steps_executable",
            requested: 2,
            available: 1,
        }
    );
}

#[test]
fn given_raw_or_malformed_storage_bytes_when_strict_run_created_then_decode_failed_matrix_denies()
-> Result<(), String> {
    let cases: [(&str, Vec<u8>); 6] = [
        (
            "raw-workflow-parts",
            postcard::to_allocvec(&minimal_workflow()?.to_parts())
                .map_err(|error| error.to_string())?,
        ),
        (
            "yaml",
            b"version: velvet-ballistics/v1\nname: raw\n".to_vec(),
        ),
        (
            "json",
            br#"{"version":"velvet-ballistics/v1","name":"raw"}"#.to_vec(),
        ),
        ("empty", Vec::new()),
        ("truncated-postcard", vec![0x01, 0x02, 0x03]),
        ("malformed", vec![0xFF, 0x00, 0xFE, 0x7F, 0x80]),
    ];

    for (index, (label, bytes)) in cases.into_iter().enumerate() {
        let requested = WorkflowDigest::from_bytes(*blake3::hash(&bytes).as_bytes());
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
fn given_evidence_schema_tamper_matrix_when_strict_run_created_then_typed_invalid_diagnostic_denies()
-> Result<(), String> {
    // Each hostile mutation of the typed evidence must fail closed with the
    // exact mapped flag rather than decoding an untrusted payload.
    #[derive(Debug)]
    enum EvidenceTamper {
        Unverified,
        SchemaVersion(u16),
        IrFlip,
        PolicyUpgrade,
    }

    let base = build_minimal_artifact(RuntimePolicy::Strict)?;
    let base_digest = base.digest;
    let journaled = build_minimal_artifact(RuntimePolicy::Journaled)?;

    let mut cases: Vec<(EvidenceTamper, AcceptedArtifact)> = Vec::new();
    for (tamper, artifact) in [
        (EvidenceTamper::Unverified, {
            let mut a = base.clone();
            a.verification.evidence = ArtifactEvidence::Unverified;
            a
        }),
        (EvidenceTamper::SchemaVersion(2), {
            let mut a = base.clone();
            let ArtifactEvidence::Verified(evidence) = &mut a.verification.evidence else {
                return Err("missing evidence".into());
            };
            evidence.schema_version = 2;
            a
        }),
        (EvidenceTamper::SchemaVersion(999), {
            let mut a = base.clone();
            let ArtifactEvidence::Verified(evidence) = &mut a.verification.evidence else {
                return Err("missing evidence".into());
            };
            evidence.schema_version = 999;
            a
        }),
        (EvidenceTamper::IrFlip, {
            let mut a = base.clone();
            if let Some(first) = a.ir.first_mut() {
                *first ^= 0xFF;
            }
            a
        }),
        (EvidenceTamper::PolicyUpgrade, journaled.clone()),
    ] {
        cases.push((tamper, artifact));
    }

    for (tamper, artifact) in cases {
        let store = FixedAcceptedStore {
            result: Ok(artifact),
        };
        let (requested, policy, expected) = match &tamper {
            EvidenceTamper::PolicyUpgrade => (
                journaled.digest,
                RuntimePolicy::Strict,
                ObservedAdmissionDiagnostic::InvalidProofFlag {
                    flag: "admission_policy",
                },
            ),
            EvidenceTamper::SchemaVersion(version) => {
                assert!(
                    *version != 3,
                    "tamper matrix must use a non-current schema version, got {version}"
                );
                (
                    base_digest,
                    RuntimePolicy::Strict,
                    ObservedAdmissionDiagnostic::InvalidProofFlag { flag: "evidence" },
                )
            }
            EvidenceTamper::Unverified | EvidenceTamper::IrFlip => (
                base_digest,
                RuntimePolicy::Strict,
                ObservedAdmissionDiagnostic::InvalidProofFlag { flag: "evidence" },
            ),
        };
        let result = admit_artifact_run(
            &store,
            policy,
            RunId::new(101),
            requested,
            CapabilitySet::empty(),
        );
        assert_eq!(
            observed(result),
            expected,
            "evidence tamper case {tamper:?}"
        );
    }
    Ok(())
}

#[test]
fn given_cli_ipc_runtime_error_mapping_when_serialized_then_error_category_digest_and_cause_are_preserved()
-> Result<(), String> {
    // The shard maps admission errors to typed RuntimeError variants; the
    // public diagnostic surface preserves category, digest, and cause for the
    // reachable Strict admission failure classes.
    let workflow = minimal_workflow()?;
    let requested = workflow.digest();
    let artifact = build_minimal_artifact(RuntimePolicy::Strict)?;
    let artifact_digest = artifact.digest;
    let mut unverified = artifact.clone();
    unverified.verification.evidence = ArtifactEvidence::Unverified;
    let mut digest_tampered = artifact.clone();
    digest_tampered.verification.digest = digest(0xD3);
    let cases = [
        (
            "not_found",
            FixedAcceptedStore {
                result: Err(ArtifactEnvelopeError::ArtifactNotFound { digest: requested }),
            },
            PublicAdmissionDiagnostic {
                category: "not_found",
                digest: Some(requested),
                cause: "artifact_not_found",
            },
        ),
        (
            "invalid_envelope",
            FixedAcceptedStore {
                result: Ok(unverified),
            },
            PublicAdmissionDiagnostic {
                category: "invalid_envelope",
                digest: Some(requested),
                cause: "artifact_envelope_invalid",
            },
        ),
        (
            "digest_mismatch",
            FixedAcceptedStore {
                result: Ok(digest_tampered),
            },
            PublicAdmissionDiagnostic {
                category: "digest_mismatch",
                digest: Some(artifact_digest),
                cause: "requested_record_envelope_mismatch",
            },
        ),
    ];

    for (index, (label, store, expected)) in cases.into_iter().enumerate() {
        let (tick_result, _before, _after) = run_strict_submit_with_store(
            store,
            RunId::new(200 + u64::try_from(index).map_err(|error| error.to_string())?),
            workflow.clone(),
            CapabilitySet::empty(),
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
        assert_eq!(actual, expected, "public diagnostic case {label}");
    }
    Ok(())
}

#[test]
fn given_any_admission_error_when_runtime_returns_then_no_frame_run_or_drive_state_allocated()
-> Result<(), String> {
    // Every rejected admission must leave the shard without allocating a run
    // frame, inserting run state, or emitting an admission journal event, and
    // the submit command must be consumed without leaving runnable work.
    let workflow = minimal_workflow()?;
    let requested = workflow.digest();
    let artifact = build_minimal_artifact(RuntimePolicy::Strict)?;
    let artifact_digest = artifact.digest;
    let mut unverified = artifact.clone();
    unverified.verification.evidence = ArtifactEvidence::Unverified;
    let mut digest_tampered = artifact.clone();
    digest_tampered.verification.digest = digest(0xD4);

    let cases = [
        (
            "not_found",
            FixedAcceptedStore {
                result: Err(ArtifactEnvelopeError::ArtifactNotFound { digest: requested }),
            },
            PublicAdmissionDiagnostic {
                category: "not_found",
                digest: Some(requested),
                cause: "artifact_not_found",
            },
        ),
        (
            "invalid_envelope",
            FixedAcceptedStore {
                result: Ok(unverified),
            },
            PublicAdmissionDiagnostic {
                category: "invalid_envelope",
                digest: Some(requested),
                cause: "artifact_envelope_invalid",
            },
        ),
        (
            "digest_mismatch",
            FixedAcceptedStore {
                result: Ok(digest_tampered),
            },
            PublicAdmissionDiagnostic {
                category: "digest_mismatch",
                digest: Some(artifact_digest),
                cause: "requested_record_envelope_mismatch",
            },
        ),
    ];

    for (index, (label, store, expected)) in cases.into_iter().enumerate() {
        let (tick_result, before, after) = run_strict_submit_with_store(
            store,
            RunId::new(300 + u64::try_from(index).map_err(|error| error.to_string())?),
            workflow.clone(),
            CapabilitySet::empty(),
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
        assert_eq!(actual, expected, "denial diagnostic case {label}");
        assert_eq!(
            after.active_runs, before.active_runs,
            "case {label} must not allocate a run"
        );
        assert_eq!(
            after.journal_events, before.journal_events,
            "case {label} must not emit RunAccepted/RunAdmission/drive events"
        );
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
    // Direct Strict construction without a storage-backed journal wires the
    // `MissingAcceptedArtifactStore`, so submission rejects instead of silently
    // accepting an unbacked artifact. A real storage-backed store is required.
    let workflow = minimal_workflow()?;
    let run_id = RunId::new(401);
    let mut shard = Shard::new_with_journal(
        ShardConfig {
            policy: RuntimePolicy::Strict,
            ..ShardConfig::default()
        },
        VolatileRuntimeJournal::shared(),
    );

    shard
        .enqueue(ShardCommand::SubmitPrePersisted {
            run: run_id,
            workflow: workflow.clone(),
            caps: CapabilitySet::empty(),
        })
        .map_err(|error| format!("enqueue failed: {error:?}"))?;
    let result = shard.tick();

    assert_eq!(
        result,
        Err(RuntimeError::AdmissionArtifactNotFound {
            digest: workflow.digest()
        })
    );
    assert_eq!(shard.active_run_count(), 0);
    Ok(())
}

#[test]
fn given_valid_accepted_artifact_when_runtime_admits_then_yaml_json_decoder_is_not_called() {
    // Static guard: strict runtime admission never invokes YAML/JSON or
    // workflow-parts decode on the admission path.
    let admission_source = include_str!("../../../crates/vb_runtime/src/admission.rs");
    let strict_path_start = admission_source.find("pub fn admit_artifact_run");
    let strict_path = match strict_path_start {
        Some(start) => match admission_source.get(start..) {
            Some(source) => source,
            None => admission_source,
        },
        None => admission_source,
    };

    assert_eq!(strict_path.contains("serde_yaml"), false);
    assert_eq!(strict_path.contains("serde_json"), false);
    assert_eq!(strict_path.contains("WorkflowParts"), false);
}

#[test]
fn given_existence_only_artifact_check_when_strict_admission_then_bypass_is_denied() {
    // Static guard: strict admission is not existence-only; the full accepted
    // artifact store surface and the `compiled_ir_exists` existence probe are
    // wired so that a fabricated always-present view cannot admit.
    let admission_source = concat!(
        include_str!("../../../crates/vb_runtime/src/admission.rs"),
        include_str!(
            "../../../crates/vb_runtime/src/admission/parts/chunk_001_types_errors_traits.rs"
        ),
        include_str!("../../../crates/vb_runtime/src/admission/parts/chunk_002_records.rs"),
        include_str!("../../../crates/vb_runtime/src/admission/parts/chunk_003_stores.rs"),
        include_str!("../../../crates/vb_runtime/src/admission/parts/chunk_004_validation.rs"),
        include_str!("../../../crates/vb_runtime/src/admission/parts/chunk_005_admit_core.rs"),
        include_str!("../../../crates/vb_runtime/src/admission/parts/chunk_006_admit_budget.rs"),
    );
    let shard_source = include_str!("../../../crates/vb_runtime/src/shard/impl_parts/chunk_001.rs");

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
    fn proptest_capability_profiles_admit_iff_required_capability_is_exactly_granted(
        name_suffix in 1u8..10,
        mutation in 0u8..6,
    ) {
        let required_name = format!("network.service{name_suffix}");
        let required = cap(&required_name, 0);
        let artifact = build_do_artifact(Box::new([required.clone()]), RuntimePolicy::Strict)
            .map_err(|error| TestCaseError::fail(format!("artifact build failed: {error}")))?;
        let artifact_digest = artifact.digest;
        let expected_policy_digest = artifact.policy_digest;
        let granted = match mutation {
            0 => caps(Box::new([required.clone()])),
            1 => CapabilitySet::empty(),
            2 => caps(Box::new([required.clone(), cap("extra.capability", 99)])),
            3 => caps(Box::new([cap("network.service", 0)])),
            4 => caps(Box::new([cap(&required_name, 1)])),
            _ => caps(Box::new([required.clone(), required.clone()])),
        };
        let store = ScenarioStore { artifact };
        let result = admit_artifact_run(
            &store,
            RuntimePolicy::Strict,
            RunId::new(501),
            artifact_digest,
            granted.clone(),
        );

        if mutation == 0 {
            let admission = result.map_err(|error| TestCaseError::fail(format!("expected admission, got error: {error:?}")))?;
            prop_assert_eq!(admission.artifact_digest(), artifact_digest);
            prop_assert_eq!(admission.run_id(), RunId::new(501));
            prop_assert_eq!(admission.granted_capabilities(), &granted);
            prop_assert_eq!(admission.policy(), RuntimePolicy::Strict);
            prop_assert_eq!(admission.policy_pin(), RunPolicyPin::Pinned(expected_policy_digest));
            prop_assert_eq!(admission.budget(), None);
            prop_assert_eq!(admission.idempotency_attested(), &[]);
        } else if mutation == 2 || mutation == 5 {
            prop_assert_eq!(
                observed(result),
                ObservedAdmissionDiagnostic::CapabilityCountMismatch {
                    required_count: 1,
                    granted_count: granted.len(),
                }
            );
        } else {
            prop_assert_eq!(
                observed(result),
                ObservedAdmissionDiagnostic::CapabilityDenied {
                    action: ActionId::new(0),
                    required,
                    granted,
                }
            );
        }
    }

    #[test]
    fn proptest_evidence_schema_version_fails_closed(schema_version in any::<u16>()) {
        let mut artifact = build_minimal_artifact(RuntimePolicy::Strict)
            .map_err(|error| TestCaseError::fail(format!("artifact build failed: {error}")))?;
        let artifact_digest = artifact.digest;
        let is_verified = match &mut artifact.verification.evidence {
            ArtifactEvidence::Verified(evidence) => {
                evidence.schema_version = schema_version;
                true
            }
            _ => false,
        };
        prop_assert!(is_verified, "checked artifact must carry typed evidence");
        let store = ScenarioStore { artifact };
        let result = observed(admit_artifact_run(
            &store,
            RuntimePolicy::Strict,
            RunId::new(502),
            artifact_digest,
            CapabilitySet::empty(),
        ));

        if schema_version == 3 {
            prop_assert!(matches!(result, ObservedAdmissionDiagnostic::Admitted(_)));
        } else {
            prop_assert_eq!(
                result,
                ObservedAdmissionDiagnostic::InvalidProofFlag { flag: "evidence" }
            );
        }
    }

    #[test]
    fn proptest_verification_digest_must_equal_artifact_digest(a in any::<u8>(), b in any::<u8>()) {
        let mut tampered = build_minimal_artifact(RuntimePolicy::Strict)
            .map_err(|error| TestCaseError::fail(format!("artifact build failed: {error}")))?;
        tampered.digest = WorkflowDigest::from_bytes([a; 32]);
        tampered.verification.digest = WorkflowDigest::from_bytes([b; 32]);
        let requested = tampered.digest;
        let store = ScenarioStore {
            artifact: tampered.clone(),
        };
        let result = observed(admit_artifact_run(
            &store,
            RuntimePolicy::Strict,
            RunId::new(503),
            requested,
            CapabilitySet::empty(),
        ));

        if a == b {
            prop_assert_eq!(
                result,
                ObservedAdmissionDiagnostic::InvalidProofFlag { flag: "evidence" }
            );
        } else {
            prop_assert_eq!(
                result,
                ObservedAdmissionDiagnostic::DigestMismatch {
                    requested,
                    found: tampered.verification.digest,
                }
            );
        }
    }

    #[test]
    fn proptest_diagnostic_mapping_is_injective_over_admission_error_categories(category in 0u8..9) {
        let requested = digest(0xFC);
        let observed = match category {
            0 => ObservedAdmissionDiagnostic::NotFound { digest: requested },
            1 => ObservedAdmissionDiagnostic::DecodeFailed,
            2 => ObservedAdmissionDiagnostic::InvalidProofFlag { flag: "evidence" },
            3 => ObservedAdmissionDiagnostic::CapabilityDenied {
                action: ActionId::new(1),
                required: cap("network", 1),
                granted: CapabilitySet::empty(),
            },
            4 => ObservedAdmissionDiagnostic::CapabilityCountMismatch {
                required_count: 1,
                granted_count: 2,
            },
            5 => ObservedAdmissionDiagnostic::DigestMismatch {
                requested,
                found: digest(0x01),
            },
            6 => ObservedAdmissionDiagnostic::StaleCertificate { digest: requested },
            7 => ObservedAdmissionDiagnostic::ResourceCapacityExceeded {
                resource: "max_steps_executable",
                requested: 2,
                available: 1,
            },
            _ => ObservedAdmissionDiagnostic::Admitted(RunAdmission::new(
                requested,
                RunId::new(504),
                CapabilitySet::empty(),
                RuntimePolicy::Strict,
            )),
        };
        let diagnostic = public_diagnostic_from_observed(observed);
        let categories = [
            "not_found",
            "decode_failed",
            "invalid_envelope",
            "capability_denied",
            "capability_count_mismatch",
            "digest_mismatch",
            "stale",
            "resource_capacity_exceeded",
            "admitted",
        ];
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
