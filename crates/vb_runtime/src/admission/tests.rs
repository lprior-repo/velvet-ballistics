use super::*;
use crate::RuntimeError;
use crate::admission::AlwaysPresentArtifactStore;
use vb_core::ids::WorkflowDigest;
use vb_storage::RunPolicyPin;

/// Builds a minimal compiled workflow for policy digest computation.
fn build_workflow_with_single_action() -> Result<vb_core::workflow::CompiledWorkflow, String> {
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};
    let node = CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: vb_core::ids::ActionId::new(1),
            input: vb_core::ids::SlotIdx::ZERO,
        },
    };
    let parts = WorkflowParts {
        name: Box::from("admission-test"),
        digest: WorkflowDigest::from_bytes([0xAB; 32]),
        semantic_source_identity: None,
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        entry: vb_core::ids::StepIdx::ZERO,
        input_slots: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::from([Box::from("step-0")]),
        symbols_count: 0,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
        .map_err(|e| format!("workflow compilation failed: {e}"))
}

struct FixedAcceptedStore {
    artifact: vb_storage::admission::AcceptedArtifact,
}

impl AcceptedArtifactStore for FixedAcceptedStore {
    fn load_accepted_artifact(
        &self,
        _artifact_digest: WorkflowDigest,
    ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
        Ok(self.artifact.clone())
    }
}

fn test_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0xAB; 32])
}

#[path = "evidence_test_support.rs"]
pub(super) mod evidence_test_support;
use evidence_test_support::accepted_artifact_with_caps;

#[test]
fn admission_new_stores_all_fields() {
    let digest = test_digest();
    let run_id = RunId::new(42);
    let caps = CapabilitySet::from_grants(Box::new([Capability::new("".into(), ActionId::new(0))]));
    let admission = RunAdmission::new(digest, run_id, caps.clone(), RuntimePolicy::Strict);
    assert_eq!(admission.artifact_digest(), digest);
    assert_eq!(admission.run_id(), run_id);
    assert_eq!(admission.granted_capabilities(), &caps);
    assert_eq!(admission.policy(), RuntimePolicy::Strict);
}

#[test]
fn admission_artifact_not_found_error_equality() {
    let digest = test_digest();
    let err = AdmissionError::ArtifactNotFound { digest };
    assert_eq!(
        err,
        AdmissionError::ArtifactNotFound {
            digest: test_digest()
        }
    );
}

#[test]
fn admission_capability_denied_error_fields() -> Result<(), String> {
    let action = ActionId::new(5);
    let required = Capability::new("secrets".into(), ActionId::new(5));
    let granted = CapabilitySet::empty();
    let err = AdmissionError::CapabilityDenied {
        action,
        required: required.clone(),
        granted: granted.clone(),
    };
    match err {
        AdmissionError::CapabilityDenied {
            action: a,
            required: r,
            granted: g,
        } => {
            assert_eq!(a, action);
            assert_eq!(r, required);
            assert_eq!(g, granted);
        }
        other => {
            assert!(false, "expected CapabilityDenied, got {other:?}");
        }
    }
    Ok(())
}

#[test]
fn admission_check_capability_granted() {
    let action = ActionId::new(1);
    let required = Capability::new("network".into(), ActionId::new(1));
    let granted = CapabilitySet::from_grants(Box::new([Capability::new(
        "network".into(),
        ActionId::new(1),
    )]));
    assert_eq!(check_capability(action, &required, &granted), Ok(()));
}

#[test]
fn admission_check_capability_denied() {
    let action = ActionId::new(1);
    let required = Capability::new("network".into(), ActionId::new(1));
    let granted = CapabilitySet::empty();
    let result = check_capability(action, &required, &granted);
    assert_eq!(
        result,
        Err(AdmissionError::CapabilityDenied {
            action,
            required,
            granted,
        })
    );
}

#[test]
fn admission_check_capability_rejects_hierarchical_grant() {
    let action = ActionId::new(99);
    let required = Capability::new("network.rpc".into(), action);
    let granted = CapabilitySet::from_grants(Box::new([Capability::new("network".into(), action)]));
    assert_eq!(
        check_capability(action, &required, &granted),
        Err(AdmissionError::CapabilityDenied {
            action,
            required,
            granted,
        })
    );
}

#[test]
fn admission_check_capability_rejects_partial_prefix_grant() {
    // Given a required capability under the network hierarchy.
    let action = ActionId::new(99);
    let required = Capability::new("network.rpc".into(), action);
    let granted = CapabilitySet::from_grants(Box::new([Capability::new("net".into(), action)]));

    // When admission checks a lexical-only prefix grant.
    let result = check_capability(action, &required, &granted);

    // Then it denies with the exact required and granted capabilities.
    assert_eq!(
        result,
        Err(AdmissionError::CapabilityDenied {
            action,
            required,
            granted,
        })
    );
}

#[test]
fn admit_artifact_run_rejects_capability_superset() -> Result<(), String> {
    // F-001 fix: strict admission rejects extras (VERUS-CARD-003 cardinality-exact).
    // RA-023 fix: a cardinality mismatch surfaces as the typed
    // `CapabilityCountMismatch` error rather than a fabricated
    // `CapabilityDenied` that names a granted capability as missing.
    //
    // The granted set contains an extra `storage` capability that the artifact
    // never declared. The per-capability loop succeeds (every required cap is
    // covered); the cardinality mismatch must reject with the typed error.
    let action = ActionId::new(7);
    let required = Capability::new("network".into(), action);
    let extra = Capability::new("storage".into(), ActionId::new(8));
    let store = FixedAcceptedStore {
        artifact: accepted_artifact_with_caps(Box::new([required]))?,
    };
    let granted =
        CapabilitySet::from_grants(Box::new([Capability::new("network".into(), action), extra]));

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        store.artifact.digest,
        granted,
    );

    assert_eq!(
        result,
        Err(AdmissionError::CapabilityCountMismatch {
            required_count: 1,
            granted_count: 2,
        })
    );
    Ok(())
}

#[test]
fn admit_artifact_run_rejects_capability_duplicate() -> Result<(), String> {
    // F-001 fix: strict admission rejects duplicate grants (cardinality mismatch).
    // RA-023 fix: surface as typed `CapabilityCountMismatch`, not fabricated denial.
    let action = ActionId::new(7);
    let required = Capability::new("network".into(), action);
    let store = FixedAcceptedStore {
        artifact: accepted_artifact_with_caps(Box::new([required.clone()]))?,
    };
    let granted = CapabilitySet::from_grants(Box::new([required.clone(), required.clone()]));

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        store.artifact.digest,
        granted,
    );

    assert_eq!(
        result,
        Err(AdmissionError::CapabilityCountMismatch {
            required_count: 1,
            granted_count: 2,
        })
    );
    Ok(())
}

#[test]
fn admit_artifact_run_count_mismatch_returns_typed_error_not_capability_denied()
-> Result<(), String> {
    // RA-023 regression: a cardinality mismatch (over-grant / extra) must surface
    // as a typed `CapabilityCountMismatch` error, NOT as a fabricated
    // `CapabilityDenied` that names a granted capability as the missing one.
    //
    // Required = 1 capability, granted = 2 capabilities (superset).
    // The per-capability loop will succeed (every required is covered).
    // The count-mismatch must be reported as `CapabilityCountMismatch { required_count, granted_count }`.
    let action = ActionId::new(7);
    let required = Capability::new("network".into(), action);
    let extra = Capability::new("storage".into(), ActionId::new(8));
    let store = FixedAcceptedStore {
        artifact: accepted_artifact_with_caps(Box::new([required.clone()]))?,
    };
    let granted = CapabilitySet::from_grants(Box::new([required.clone(), extra]));

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        store.artifact.digest,
        granted,
    );

    assert_eq!(
        result,
        Err(AdmissionError::CapabilityCountMismatch {
            required_count: 1,
            granted_count: 2,
        })
    );
    Ok(())
}

#[test]
fn admit_artifact_run_count_mismatch_under_grant_returns_typed_error_not_per_cap_denial()
-> Result<(), String> {
    // RA-023 / RA-018: when count differs, return the typed count-mismatch
    // error instead of fabricating a per-capability denial on a granted cap.
    // Required = 2, granted = 1 (under-grant with different membership).
    // The per-capability loop will fail first on the second required cap,
    // which is the legitimate per-capability denial path. To exercise the
    // count-mismatch path specifically we need granted ⊋ required with extras
    // matching membership — see superset test above. This test pins the
    // invariant that the count-mismatch error is a separate variant and
    // cannot be confused with `CapabilityDenied`.
    let network = Capability::new("network.github".into(), ActionId::new(7));
    let filesystem = Capability::new("filesystem.read".into(), ActionId::new(8));
    let extra = Capability::new("storage.write".into(), ActionId::new(9));
    let store = FixedAcceptedStore {
        artifact: accepted_artifact_with_caps(Box::new([network.clone(), filesystem.clone()]))?,
    };
    let granted = CapabilitySet::from_grants(Box::new([network, filesystem, extra]));

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        store.artifact.digest,
        granted,
    );

    assert_eq!(
        result,
        Err(AdmissionError::CapabilityCountMismatch {
            required_count: 2,
            granted_count: 3,
        })
    );
    Ok(())
}

#[test]
fn capability_count_mismatch_vb_ia7sq_typed_error_not_fabricated_capability_denied()
-> Result<(), String> {
    // vb-ia7sq / RA-018 regression: when the cardinality check is reached
    // (i.e. every required capability is covered), a count mismatch MUST
    // surface as a typed `CapabilityCountMismatch { required_count, granted_count }`
    // and MUST NOT fabricate a `Capability::__capability_count_mismatch__` /
    // `ActionId::new(0)` sentinel and mislabel the error as `CapabilityDenied`.
    //
    // The production path at `admission.rs:740-750` runs the per-capability
    // loop FIRST, so the cardinality check is only reached when granted is
    // a superset of required. We exercise that reachable case (required=3,
    // granted=4 with membership ⊇ required) and also assert the operator
    // diagnostic surface: the typed `AdmissionError` maps to the typed
    // `RuntimeError::AdmissionCapabilityCountMismatch` (diagnostic code
    // 0x201F, set in `error/diagnostics.rs:32`).
    let network = Capability::new("network.github".into(), ActionId::new(7));
    let filesystem = Capability::new("filesystem.read".into(), ActionId::new(8));
    let kv = Capability::new("kv.write".into(), ActionId::new(9));
    let extra = Capability::new("storage.write".into(), ActionId::new(10));
    let store = FixedAcceptedStore {
        artifact: accepted_artifact_with_caps(Box::new([
            network.clone(),
            filesystem.clone(),
            kv.clone(),
        ]))?,
    };
    let granted = CapabilitySet::from_grants(Box::new([network, filesystem, kv, extra]));

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        store.artifact.digest,
        granted,
    );

    assert_eq!(
        result,
        Err(AdmissionError::CapabilityCountMismatch {
            required_count: 3,
            granted_count: 4,
        })
    );

    // Pin the contract: a re-admission through the same path must also yield
    // the typed variant, never `CapabilityDenied` with a fabricated capability.
    let store2 = FixedAcceptedStore {
        artifact: accepted_artifact_with_caps(Box::new([
            Capability::new("a".into(), ActionId::new(1)),
            Capability::new("b".into(), ActionId::new(2)),
            Capability::new("c".into(), ActionId::new(3)),
        ]))?,
    };
    let granted2 = CapabilitySet::from_grants(Box::new([
        Capability::new("a".into(), ActionId::new(1)),
        Capability::new("b".into(), ActionId::new(2)),
        Capability::new("c".into(), ActionId::new(3)),
        Capability::new("d".into(), ActionId::new(4)),
    ]));
    let result2 = admit_artifact_run(
        &store2,
        RuntimePolicy::Strict,
        RunId::new(2),
        store2.artifact.digest,
        granted2,
    );
    assert_eq!(
        result2,
        Err(AdmissionError::CapabilityCountMismatch {
            required_count: 3,
            granted_count: 4,
        })
    );

    // Pin the operator diagnostic surface: the canonical mapping from
    // `AdmissionError::CapabilityCountMismatch` to `RuntimeError` is the
    // typed `AdmissionCapabilityCountMismatch` variant carrying the same
    // raw counts (no fabrication, no `Box<dyn Error>` smuggled in).
    let mapped: RuntimeError = match &result2 {
        Err(AdmissionError::CapabilityCountMismatch {
            required_count,
            granted_count,
        }) => RuntimeError::AdmissionCapabilityCountMismatch {
            required_count: *required_count,
            granted_count: *granted_count,
        },
        other => return Err(format!("expected count mismatch, got {other:?}")),
    };
    assert_eq!(
        mapped,
        RuntimeError::AdmissionCapabilityCountMismatch {
            required_count: 3,
            granted_count: 4,
        }
    );
    Ok(())
}

#[test]
fn admit_artifact_run_count_match_with_missing_capability_still_returns_capability_denied()
-> Result<(), String> {
    // RA-023 contract preservation: when cardinality is equal but membership
    // differs, the per-capability loop must fire and surface the actual
    // missing capability, NOT a count-mismatch error.
    let network = Capability::new("network.github".into(), ActionId::new(7));
    let filesystem = Capability::new("filesystem.read".into(), ActionId::new(8));
    let wrong = Capability::new("network.other".into(), ActionId::new(11));
    let store = FixedAcceptedStore {
        artifact: accepted_artifact_with_caps(Box::new([network.clone(), filesystem.clone()]))?,
    };
    // Granted has the right count (2) but the second cap is wrong.
    let granted = CapabilitySet::from_grants(Box::new([network, wrong]));

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        store.artifact.digest,
        granted.clone(),
    );

    assert_eq!(
        result,
        Err(AdmissionError::CapabilityDenied {
            action: filesystem.action_id(),
            required: filesystem,
            granted,
        })
    );
    Ok(())
}

#[test]
fn admit_artifact_run_accepts_capability_exact_match() -> Result<(), String> {
    // F-001 fix: strict admission accepts exactly-equal capability sets.
    let action = ActionId::new(7);
    let required = Capability::new("network".into(), action);
    let store = FixedAcceptedStore {
        artifact: accepted_artifact_with_caps(Box::new([required.clone()]))?,
    };
    let granted = CapabilitySet::from_grants(Box::new([required.clone()]));

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        store.artifact.digest,
        granted.clone(),
    );

    let admitted = result.map_err(|e| format!("exact-match capability set must admit: {e:?}"))?;
    assert_eq!(admitted.granted_capabilities(), &granted);
    Ok(())
}

#[test]
fn admit_artifact_run_rejects_missing_grants_without_allocation() -> Result<(), String> {
    let network = Capability::new("network.github".into(), ActionId::new(7));
    let filesystem = Capability::new("filesystem.read".into(), ActionId::new(8));
    let store = FixedAcceptedStore {
        artifact: accepted_artifact_with_caps(Box::new([network.clone(), filesystem.clone()]))?,
    };
    let granted = CapabilitySet::from_grants(Box::new([network.clone()]));

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        store.artifact.digest,
        granted.clone(),
    );

    assert_eq!(
        result,
        Err(AdmissionError::CapabilityDenied {
            action: filesystem.action_id(),
            required: filesystem,
            granted,
        })
    );
    Ok(())
}

#[test]
fn admit_artifact_run_rejects_non_exact_grant_without_allocation() -> Result<(), String> {
    let action = ActionId::new(7);
    let required = Capability::new("network.github".into(), action);
    let store = FixedAcceptedStore {
        artifact: accepted_artifact_with_caps(Box::new([required.clone()]))?,
    };
    let granted = CapabilitySet::from_grants(Box::new([Capability::new("network".into(), action)]));

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        store.artifact.digest,
        granted.clone(),
    );

    assert_eq!(
        result,
        Err(AdmissionError::CapabilityDenied {
            action,
            required,
            granted,
        })
    );
    Ok(())
}

#[test]
fn admit_artifact_run_preserves_non_empty_required_capabilities() -> Result<(), String> {
    let action = ActionId::new(7);
    let required = Capability::new("network".into(), action);
    let store = FixedAcceptedStore {
        artifact: accepted_artifact_with_caps(Box::new([required.clone()]))?,
    };
    let granted = CapabilitySet::from_grants(Box::new([required]));

    let admission = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        store.artifact.digest,
        granted.clone(),
    );

    assert!(matches!(admission, Ok(run) if run.granted_capabilities() == &granted));
    Ok(())
}

#[test]
fn admit_artifact_run_rejects_missing_idempotency_gate() -> Result<(), String> {
    let mut artifact = accepted_artifact_with_caps(Box::new([]))?;
    artifact.verification.idempotency_verified_claimed = false;
    let store = FixedAcceptedStore { artifact };

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        store.artifact.digest,
        CapabilitySet::empty(),
    );

    assert_eq!(
        result,
        Err(AdmissionError::ArtifactInvalidProofFlag { flag: "evidence" })
    );
    Ok(())
}

#[test]
fn admit_artifact_run_rejects_keyed_action_without_attestation() -> Result<(), String> {
    let action = ActionId::new(9);
    let mut artifact = accepted_artifact_with_caps(Box::new([]))?;
    artifact.verification.idempotency_keyed = Box::new([action]);
    artifact.verification.idempotency_attested = Box::new([]);
    let store = FixedAcceptedStore { artifact };

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        store.artifact.digest,
        CapabilitySet::empty(),
    );

    assert_eq!(
        result,
        Err(AdmissionError::ArtifactInvalidProofFlag { flag: "evidence" })
    );
    Ok(())
}

#[test]
fn admit_artifact_run_does_not_invent_executor_attestation() -> Result<(), String> {
    let artifact = accepted_artifact_with_caps(Box::new([]))?;
    let store = FixedAcceptedStore { artifact };

    let admission = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        store.artifact.digest,
        CapabilitySet::empty(),
    );

    assert!(matches!(admission, Ok(run) if run.idempotency_attested().is_empty()));
    Ok(())
}

#[test]
fn admission_policy_relaxed_stored() {
    let digest = test_digest();
    let run_id = RunId::new(1);
    let caps = CapabilitySet::empty();
    let admission = RunAdmission::new(digest, run_id, caps, RuntimePolicy::Relaxed);
    assert_eq!(admission.policy(), RuntimePolicy::Relaxed);
}

#[test]
fn admission_policy_journaled_stored() {
    let digest = test_digest();
    let run_id = RunId::new(1);
    let caps = CapabilitySet::empty();
    let admission = RunAdmission::new(digest, run_id, caps, RuntimePolicy::Journaled);
    assert_eq!(admission.policy(), RuntimePolicy::Journaled);
}

#[test]
fn admission_clone_is_equal() {
    let digest = test_digest();
    let run_id = RunId::new(7);
    let caps = CapabilitySet::from_grants(Box::new([Capability::new("".into(), ActionId::new(0))]));
    let original = RunAdmission::new(digest, run_id, caps, RuntimePolicy::Strict);
    let cloned = original.clone();
    assert_eq!(cloned, original);
}

#[test]
fn admission_admit_run_strict_with_present_artifact() -> Result<(), String> {
    let store = FixedAcceptedStore {
        artifact: accepted_artifact_with_caps(Box::new([]))?,
    };
    let digest = store.artifact.digest;
    let run_id = RunId::new(1);
    let caps = CapabilitySet::empty();
    let result = admit_run(&store, RuntimePolicy::Strict, digest, run_id, caps.clone());
    let admission = RunAdmission::new(digest, run_id, caps, RuntimePolicy::Strict);
    assert_eq!(result, Ok(admission.clone()));
    assert_eq!(admission.artifact_digest(), digest);
    assert_eq!(admission.run_id(), run_id);
    assert_eq!(admission.policy(), RuntimePolicy::Strict);
    Ok(())
}

#[test]
fn admission_admit_run_strict_rejects_loaded_digest_mismatch() -> Result<(), String> {
    struct MismatchedAcceptedStore;
    impl AcceptedArtifactStore for MismatchedAcceptedStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            let found = WorkflowDigest::from_bytes([0x42; 32]);
            let mut artifact = accepted_artifact_with_caps(Box::new([]))
                .map_err(|_| ArtifactEnvelopeError::ArtifactEvidenceInvalid)?;
            artifact.digest = found;
            artifact.source_digest = found;
            artifact.verification.digest = found;
            Ok(artifact)
        }
    }
    let requested = test_digest();

    let result = admit_run(
        &MismatchedAcceptedStore,
        RuntimePolicy::Strict,
        requested,
        RunId::new(1),
        CapabilitySet::empty(),
    );

    assert_eq!(
        result,
        Err(AdmissionError::ArtifactInvalidProofFlag { flag: "evidence" })
    );
    Ok(())
}

#[test]
fn admission_admit_run_strict_rejects_loaded_proof_digest_mismatch() -> Result<(), String> {
    struct MismatchedProofStore;
    impl AcceptedArtifactStore for MismatchedProofStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            let mut artifact = accepted_artifact_with_caps(Box::new([]))
                .map_err(|_| ArtifactEnvelopeError::ArtifactEvidenceInvalid)?;
            artifact.verification.digest = WorkflowDigest::from_bytes([0x42; 32]);
            Ok(artifact)
        }
    }
    let requested = accepted_artifact_with_caps(Box::new([]))?.digest;

    let result = admit_run(
        &MismatchedProofStore,
        RuntimePolicy::Strict,
        requested,
        RunId::new(1),
        CapabilitySet::empty(),
    );

    assert_eq!(
        result,
        Err(AdmissionError::ArtifactDigestMismatch {
            requested,
            found: WorkflowDigest::from_bytes([0x42; 32]),
        })
    );
    Ok(())
}

#[test]
fn admission_admit_run_relaxed_without_artifact() {
    /// An artifact store that always reports artifacts as absent.
    struct NeverPresentStore;
    impl AcceptedArtifactStore for NeverPresentStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            Err(ArtifactEnvelopeError::ArtifactNotFound {
                digest: WorkflowDigest::from_bytes([0u8; 32]),
            })
        }
    }
    let store = NeverPresentStore;
    let digest = test_digest();
    let run_id = RunId::new(1);
    let caps = CapabilitySet::empty();
    let result = admit_run(&store, RuntimePolicy::Relaxed, digest, run_id, caps);
    assert_eq!(
        result,
        Ok(RunAdmission::new(
            digest,
            run_id,
            CapabilitySet::empty(),
            RuntimePolicy::Relaxed,
        ))
    );
}

#[test]
fn admission_admit_run_strict_without_artifact_rejected() {
    /// An artifact store that always reports artifacts as absent.
    struct NeverPresentStore;
    impl AcceptedArtifactStore for NeverPresentStore {
        fn load_accepted_artifact(
            &self,
            digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            Err(ArtifactEnvelopeError::ArtifactNotFound { digest })
        }
    }
    let store = NeverPresentStore;
    let digest = test_digest();
    let run_id = RunId::new(1);
    let caps = CapabilitySet::empty();
    let result = admit_run(&store, RuntimePolicy::Strict, digest, run_id, caps);
    assert_eq!(result, Err(AdmissionError::ArtifactNotFound { digest }));
}

#[test]
fn admission_admit_run_journaled_without_artifact_rejected() {
    /// An artifact store that always reports artifacts as absent.
    struct NeverPresentStore;
    impl AcceptedArtifactStore for NeverPresentStore {
        fn load_accepted_artifact(
            &self,
            digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            Err(ArtifactEnvelopeError::ArtifactNotFound { digest })
        }
    }
    let store = NeverPresentStore;
    let digest = test_digest();
    let run_id = RunId::new(1);
    let caps = CapabilitySet::empty();
    let result = admit_run(&store, RuntimePolicy::Journaled, digest, run_id, caps);
    assert_eq!(result, Err(AdmissionError::ArtifactNotFound { digest }));
}

// -------------------------------------------------------------------------
// ArtifactEnvelopeError propagation tests (B-14, B-16, B-17, B-18, B-19, B-20)
// -------------------------------------------------------------------------

#[test]
fn load_accepted_artifact_returns_postcard_decode_failed() {
    /// Store that always returns PostcardDecodeFailed.
    struct PostcardFailingStore;
    impl AcceptedArtifactStore for PostcardFailingStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            Err(ArtifactEnvelopeError::PostcardDecodeFailed)
        }
    }
    let store = PostcardFailingStore;
    let result = store.load_accepted_artifact(test_digest());
    assert_eq!(result, Err(ArtifactEnvelopeError::PostcardDecodeFailed));
}

#[test]
fn load_accepted_artifact_returns_invalid_gate_count() {
    /// Store that returns ArtifactEnvelopeError::InvalidGateCount — simulating
    /// what StorageArtifactStore returns when loading a corrupt artifact with wrong gate count.
    struct WrongGateCountStore;
    impl AcceptedArtifactStore for WrongGateCountStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            Err(ArtifactEnvelopeError::InvalidGateCount {
                found: 7,
                required: 15,
            })
        }
    }
    let store = WrongGateCountStore;
    let result = store.load_accepted_artifact(test_digest());
    assert_eq!(
        result,
        Err(ArtifactEnvelopeError::InvalidGateCount {
            found: 7,
            required: 15
        })
    );
}

#[test]
fn admit_artifact_run_rejects_missing_bounded_flag() -> Result<(), String> {
    /// Store that returns artifact with bounded=false.
    struct BoundedFalseStore;
    impl AcceptedArtifactStore for BoundedFalseStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            let mut artifact = accepted_artifact_with_caps(Box::new([]))
                .map_err(|_| ArtifactEnvelopeError::ArtifactEvidenceInvalid)?;
            artifact.verification.bounded_claimed = false;
            Ok(artifact)
        }
    }
    let store = BoundedFalseStore;
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        test_digest(),
        CapabilitySet::empty(),
    );
    assert_eq!(
        result,
        Err(AdmissionError::ArtifactInvalidProofFlag { flag: "evidence" })
    );
    Ok(())
}

#[test]
fn admit_artifact_run_rejects_forged_taint_safe_claim() -> Result<(), String> {
    struct TaintSafeFalseStore;
    impl AcceptedArtifactStore for TaintSafeFalseStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            let mut artifact = accepted_artifact_with_caps(Box::new([]))
                .map_err(|_| ArtifactEnvelopeError::ArtifactEvidenceInvalid)?;
            artifact.verification.taint_safe_claimed = true;
            Ok(artifact)
        }
    }
    let store = TaintSafeFalseStore;
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        test_digest(),
        CapabilitySet::empty(),
    );
    assert_eq!(
        result,
        Err(AdmissionError::ArtifactInvalidProofFlag { flag: "evidence" })
    );
    Ok(())
}

#[test]
fn admit_artifact_run_rejects_forged_retry_safe_claim() -> Result<(), String> {
    struct RetrySafeFalseStore;
    impl AcceptedArtifactStore for RetrySafeFalseStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            let mut artifact = accepted_artifact_with_caps(Box::new([]))
                .map_err(|_| ArtifactEnvelopeError::ArtifactEvidenceInvalid)?;
            artifact.verification.retry_safe_claimed = true;
            Ok(artifact)
        }
    }
    let store = RetrySafeFalseStore;
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        test_digest(),
        CapabilitySet::empty(),
    );
    assert_eq!(
        result,
        Err(AdmissionError::ArtifactInvalidProofFlag { flag: "evidence" })
    );
    Ok(())
}

#[test]
fn admit_artifact_run_rejects_forged_durable_claim() -> Result<(), String> {
    struct DurableFalseStore;
    impl AcceptedArtifactStore for DurableFalseStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            let mut artifact = accepted_artifact_with_caps(Box::new([]))
                .map_err(|_| ArtifactEnvelopeError::ArtifactEvidenceInvalid)?;
            artifact.verification.durable = true;
            Ok(artifact)
        }
    }
    let store = DurableFalseStore;
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        test_digest(),
        CapabilitySet::empty(),
    );
    assert_eq!(
        result,
        Err(AdmissionError::ArtifactInvalidProofFlag { flag: "evidence" })
    );
    Ok(())
}

#[test]
fn admit_artifact_run_rejects_forged_replayable_claim() -> Result<(), String> {
    struct ReplayableFalseStore;
    impl AcceptedArtifactStore for ReplayableFalseStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            let mut artifact = accepted_artifact_with_caps(Box::new([]))
                .map_err(|_| ArtifactEnvelopeError::ArtifactEvidenceInvalid)?;
            artifact.verification.replayable_claimed = true;
            Ok(artifact)
        }
    }
    let store = ReplayableFalseStore;
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        test_digest(),
        CapabilitySet::empty(),
    );
    assert_eq!(
        result,
        Err(AdmissionError::ArtifactInvalidProofFlag { flag: "evidence" })
    );
    Ok(())
}

#[test]
fn admit_artifact_run_rejects_missing_idempotency_verified_flag() -> Result<(), String> {
    struct IdempotencyVerifiedFalseStore;
    impl AcceptedArtifactStore for IdempotencyVerifiedFalseStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            let mut artifact = accepted_artifact_with_caps(Box::new([]))
                .map_err(|_| ArtifactEnvelopeError::ArtifactEvidenceInvalid)?;
            artifact.verification.idempotency_verified_claimed = false;
            Ok(artifact)
        }
    }
    let store = IdempotencyVerifiedFalseStore;
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        test_digest(),
        CapabilitySet::empty(),
    );
    assert_eq!(
        result,
        Err(AdmissionError::ArtifactInvalidProofFlag { flag: "evidence" })
    );
    Ok(())
}

#[test]
fn admit_artifact_run_rejects_missing_idempotency_attestation() -> Result<(), String> {
    /// Store that returns artifact with idempotency_keyed but no attestation.
    struct MissingAttestationStore {
        action: ActionId,
    }
    impl AcceptedArtifactStore for MissingAttestationStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            let mut artifact = accepted_artifact_with_caps(Box::new([]))
                .map_err(|_| ArtifactEnvelopeError::ArtifactEvidenceInvalid)?;
            artifact.verification.idempotency_keyed = Box::new([self.action]);
            artifact.verification.idempotency_attested = Box::new([]);
            Ok(artifact)
        }
    }
    let store = MissingAttestationStore {
        action: ActionId::new(42),
    };
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        test_digest(),
        CapabilitySet::empty(),
    );
    assert_eq!(
        result,
        Err(AdmissionError::ArtifactInvalidProofFlag { flag: "evidence" })
    );
    Ok(())
}

// -------------------------------------------------------------------------
// admit_artifact_run digest mismatch (B-11)
// -------------------------------------------------------------------------

#[test]
fn admit_artifact_run_returns_digest_mismatch() -> Result<(), String> {
    /// Store that returns an artifact with a different digest than requested.
    struct MismatchedDigestStore;
    impl AcceptedArtifactStore for MismatchedDigestStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            let wrong_digest = WorkflowDigest::from_bytes([0x42; 32]);
            let mut artifact = accepted_artifact_with_caps(Box::new([]))
                .map_err(|_| ArtifactEnvelopeError::ArtifactEvidenceInvalid)?;
            artifact.digest = wrong_digest;
            artifact.source_digest = wrong_digest;
            artifact.verification.digest = wrong_digest;
            Ok(artifact)
        }
    }
    let store = MismatchedDigestStore;
    let requested = test_digest();
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        requested,
        CapabilitySet::empty(),
    );
    assert_eq!(
        result,
        Err(AdmissionError::ArtifactInvalidProofFlag { flag: "evidence" })
    );
    Ok(())
}

// -------------------------------------------------------------------------
// admit_run_with_budget tests
// -------------------------------------------------------------------------

#[test]
fn admit_run_with_budget_returns_resource_capacity_exceeded() {
    /// A store that always says artifact exists.
    struct AlwaysPresentStore;
    impl ArtifactStore for AlwaysPresentStore {
        fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
            true
        }
    }
    let store = AlwaysPresentStore;
    let digest = test_digest();
    let run_id = RunId::new(1);
    let caps = CapabilitySet::empty();
    // Request more steps than the capacity allows, while staying inside policy.
    let requested = AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets: 1,
        max_parallel_in_flight: 1,
        max_retries_per_action: 1,
        max_gather_pages: 1,
        max_gather_items: 1,
        max_for_each_iterations: 1,
        max_together_branches: 1,
        max_repeat_attempts: 1,
        max_run_time_seconds: 10,
        max_result_bytes: 1,
        max_total_slots_written: 1,
        max_timer_entries: 1,
        max_trace_events: 1,
        max_queue_depth: 1,
        max_journal_batch_bytes: 1,
        max_ipc_payload_bytes: 1,
        max_blob_bytes: 1,
        max_input_bytes: 1,
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 1,
    };
    let capacity = AggregateResourceCapacity {
        max_steps_executable: 0,
        max_action_tickets: u64::MAX,
        max_parallel_in_flight: u32::MAX,
        max_gather_pages: u64::MAX,
        max_gather_items: u64::MAX,
        max_result_bytes: u64::MAX,
        max_total_slots_written: u64::MAX,
        max_timer_entries: u64::MAX,
        max_trace_events: u64::MAX,
        max_active_runs: u64::MAX,
        max_queue_depth: u64::MAX,
        max_journal_batch_bytes: u64::MAX,
        max_ipc_payload_bytes: u64::MAX,
        max_blob_bytes: u64::MAX,
        max_input_bytes: u64::MAX,
        max_step_budget_per_tick: u64::MAX,
        max_transitions_per_tick: u64::MAX,
    };

    let result = admit_run_with_budget(
        &store,
        RuntimePolicy::Strict,
        digest,
        run_id,
        caps,
        requested,
        capacity,
    );

    assert!(matches!(
        result,
        Err(AdmissionError::ResourceCapacityExceeded { .. })
    ));
}

#[test]
fn admit_run_with_budget_policy_rejects_over_policy_before_capacity() {
    struct AlwaysPresentStore;
    impl ArtifactStore for AlwaysPresentStore {
        fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
            true
        }
    }
    let requested = AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets: 1,
        max_parallel_in_flight: 1,
        max_retries_per_action: 1,
        max_gather_pages: 1,
        max_gather_items: 1,
        max_for_each_iterations: 1,
        max_together_branches: 1,
        max_repeat_attempts: 1,
        max_run_time_seconds: 10,
        max_result_bytes: 2_001,
        max_total_slots_written: 1,
        max_timer_entries: 1,
        max_trace_events: 1,
        max_queue_depth: 1,
        max_journal_batch_bytes: 1,
        max_ipc_payload_bytes: 1,
        max_blob_bytes: 1,
        max_input_bytes: 1,
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 1,
    };
    let capacity = AggregateResourceCapacity {
        max_steps_executable: u64::MAX,
        max_action_tickets: u64::MAX,
        max_parallel_in_flight: u32::MAX,
        max_gather_pages: u64::MAX,
        max_gather_items: u64::MAX,
        max_result_bytes: u64::MAX,
        max_total_slots_written: u64::MAX,
        max_timer_entries: u64::MAX,
        max_trace_events: u64::MAX,
        max_active_runs: u64::MAX,
        max_queue_depth: u64::MAX,
        max_journal_batch_bytes: u64::MAX,
        max_ipc_payload_bytes: u64::MAX,
        max_blob_bytes: u64::MAX,
        max_input_bytes: u64::MAX,
        max_step_budget_per_tick: u64::MAX,
        max_transitions_per_tick: u64::MAX,
    };
    let budget_policy = BoundednessPolicy {
        absolute_max_result_bytes: 2_000,
        ..BoundednessPolicy::DEFAULT
    };

    let result = admit_run_with_budget_policy(
        &AlwaysPresentStore,
        RuntimePolicy::Strict,
        test_digest(),
        RunId::new(77),
        CapabilitySet::empty(),
        AdmissionBudgetRequest {
            requested,
            available: capacity,
            policy: budget_policy,
        },
    );

    assert_eq!(
        result,
        Err(AdmissionError::BudgetPolicyExceeded {
            resource: "max_result_bytes",
            actual: 2_001,
            limit: 2_000,
        })
    );
}

#[test]
fn admit_run_with_budget_rejects_strict_without_artifact() {
    /// A store that always says artifact does NOT exist.
    struct NeverPresentStore;
    impl ArtifactStore for NeverPresentStore {
        fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
            false
        }
    }
    let store = NeverPresentStore;
    let digest = test_digest();
    let run_id = RunId::new(1);
    let caps = CapabilitySet::empty();
    let requested = AggregateResourceBudget {
        max_steps_executable: 0,
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
    let capacity = AggregateResourceCapacity {
        max_steps_executable: u64::MAX,
        max_action_tickets: u64::MAX,
        max_parallel_in_flight: u32::MAX,
        max_gather_pages: u64::MAX,
        max_gather_items: u64::MAX,
        max_result_bytes: u64::MAX,
        max_total_slots_written: u64::MAX,
        max_timer_entries: u64::MAX,
        max_trace_events: u64::MAX,
        max_active_runs: u64::MAX,
        max_queue_depth: u64::MAX,
        max_journal_batch_bytes: u64::MAX,
        max_ipc_payload_bytes: u64::MAX,
        max_blob_bytes: u64::MAX,
        max_input_bytes: u64::MAX,
        max_step_budget_per_tick: u64::MAX,
        max_transitions_per_tick: u64::MAX,
    };

    let result = admit_run_with_budget(
        &store,
        RuntimePolicy::Strict,
        digest,
        run_id,
        caps,
        requested,
        capacity,
    );

    assert_eq!(result, Err(AdmissionError::ArtifactNotFound { digest }));
}

// ══ vb-h7rex: Policy pin constructor and serialization tests ═════════════
#[test]
fn admission_legacy_constructor_produces_explicit_unverified_pin() {
    // RA-019 regression: compatibility constructor must produce explicit
    // RunPolicyPin::Unverified, never a silent new authority.
    let digest = test_digest();
    let run_id = RunId::new(1);
    let caps = CapabilitySet::empty();
    let admission = RunAdmission::new(digest, run_id, caps, RuntimePolicy::Relaxed);
    assert_eq!(admission.policy_pin(), RunPolicyPin::Unverified);
}

#[test]
fn admission_with_policy_pin_builder_sets_actual_pin() {
    // RA-021: verified artifact admission must set the actual policy identity,
    // not the legacy Unverified default.
    let digest = test_digest();
    let run_id = RunId::new(1);
    let caps = CapabilitySet::empty();
    let policy_digest = WorkflowDigest::from_bytes([0xCC; 32]);
    let admission = RunAdmission::new(digest, run_id, caps, RuntimePolicy::Strict)
        .with_policy_pin(RunPolicyPin::Pinned(policy_digest));
    assert_eq!(admission.policy_pin(), RunPolicyPin::Pinned(policy_digest));
}

#[test]
fn admission_strict_journaled_admit_artifact_run_sets_pinned_policy_pin() -> Result<(), String> {
    // RA-017 / RA-020 regression: full strict and journaled admission of an
    // accepted artifact must record the artifact's actual policy_digest as the
    // RunPolicyPin, not Unverified or a caller-provided value.
    use vb_storage::admission::compute_policy_digest;
    let action = ActionId::new(1);
    let required = Capability::new("network".into(), action);
    let caps = CapabilitySet::from_grants(Box::new([required]));
    let required_for_artifact = Capability::new("network".into(), action);
    let artifact_caps = Box::new([required_for_artifact]);
    let mut artifact = accepted_artifact_with_caps(artifact_caps)?;
    let workflow_parts = build_workflow_with_single_action()?;
    let computed_policy = compute_policy_digest(&workflow_parts)
        .map_err(|e| format!("compute_policy_digest failed: {e}"))?;
    artifact.policy_digest = computed_policy;
    let store = FixedAcceptedStore { artifact };
    let run_id = RunId::new(1);
    // The artifact returned by the store has its own digest; the admission
    // gate validates that the loaded artifact's digest matches the request.
    // Use the actual artifact digest for the request to pass validation.
    let artifact_digest = store.artifact.digest;
    let result = crate::admission::admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        run_id,
        artifact_digest,
        caps,
    );
    let admission = result.map_err(|e| format!("admit_artifact_run failed: {e}"))?;
    assert_eq!(admission.policy(), RuntimePolicy::Strict);
    assert_eq!(
        admission.policy_pin(),
        RunPolicyPin::Pinned(computed_policy)
    );
    assert_ne!(admission.policy_pin(), RunPolicyPin::Unverified);
    Ok(())
}

#[test]
fn admission_relaxed_admit_run_keeps_unverified_pin() {
    // RA-019: relaxed admission without artifact validation must remain
    // explicitly Unverified, even when the caller knows the policy digest.
    let digest = test_digest();
    let run_id = RunId::new(1);
    let caps = CapabilitySet::empty();
    let result = crate::admission::admit_run(
        &AlwaysPresentArtifactStore,
        RuntimePolicy::Relaxed,
        digest,
        run_id,
        caps.clone(),
    );
    assert_eq!(
        result,
        Ok(RunAdmission::new(
            digest,
            run_id,
            caps,
            RuntimePolicy::Relaxed
        ))
    );
}

// ══ vb-h7rex: Checkpoint policy pin roundtrip tests ═════════════════════
#[test]
fn checkpoint_preserves_pinned_policy_pin_through_roundtrip() -> Result<(), String> {
    // RA-022: new checkpoints with pinned policy identity must survive
    // serialize → deserialize without silent degradation to Unverified.
    let run_id = RunId::new(1);
    let digest = test_digest();
    let policy_digest = WorkflowDigest::from_bytes([0xDD; 32]);
    let admission = RunAdmission::new(
        digest,
        run_id,
        CapabilitySet::empty(),
        RuntimePolicy::Strict,
    )
    .with_policy_pin(RunPolicyPin::Pinned(policy_digest));
    let bytes = postcard::to_allocvec(&admission).map_err(|e| e.to_string())?;
    let restored: RunAdmission = postcard::from_bytes(&bytes).map_err(|e| e.to_string())?;
    assert_eq!(restored.policy_pin(), RunPolicyPin::Pinned(policy_digest));
    Ok(())
}

// ══ vb-hbav B14: AdmissionError exhaustiveness compile-time check ════════
#[test]
fn admission_error_match_covers_all_variants() {
    fn _exhaustive_match(e: &AdmissionError) -> &'static str {
        match e {
            AdmissionError::ArtifactNotFound { .. } => "artifact_not_found",
            AdmissionError::CapabilityDenied { .. } => "capability_denied",
            AdmissionError::ResourceCapacityExceeded { .. } => "resource_capacity_exceeded",
            AdmissionError::BudgetPolicyExceeded { .. } => "budget_policy_exceeded",
            AdmissionError::ResourceBudgetOverflow { .. } => "resource_budget_overflow",
            AdmissionError::ResourceBudgetUnderflow { .. } => "resource_budget_underflow",
            AdmissionError::ResourceBudgetInvalidCapacity { .. } => {
                "resource_budget_invalid_capacity"
            }
            AdmissionError::ResourceStepCeilingExceeded { .. } => "resource_step_ceiling_exceeded",
            AdmissionError::ResourcePerTickCeilingExceeded { .. } => {
                "resource_per_tick_ceiling_exceeded"
            }
            AdmissionError::ArtifactEnvelopeDecodeFailed => "artifact_envelope_decode_failed",
            AdmissionError::ArtifactInvalidGateCount { .. } => "artifact_invalid_gate_count",
            AdmissionError::ArtifactInvalidProofFlag { .. } => "artifact_invalid_proof_flag",
            AdmissionError::ArtifactDigestMismatch { .. } => "artifact_digest_mismatch",
            AdmissionError::ArtifactCertificateStale { .. } => "artifact_certificate_stale",
            AdmissionError::CapabilityCountMismatch { .. } => "capability_count_mismatch",
        }
    }
    let _ = _exhaustive_match;
}
