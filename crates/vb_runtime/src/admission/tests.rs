use super::*;
use vb_core::ids::WorkflowDigest;

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

fn accepted_artifact_with_caps(
    required_capabilities: Box<[Capability]>,
) -> vb_storage::admission::AcceptedArtifact {
    let digest = test_digest();
    vb_storage::admission::AcceptedArtifact {
        digest,
        source_digest: digest,
        policy_digest: digest,
        ir: Vec::new(),
        verification: vb_storage::admission::VerificationProof {
            digest,
            gate_count: REQUIRED_GATE_COUNT,
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
        accepted_at_seq: vb_storage::EventSeq::new(0),
        required_capabilities,
    }
}

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
fn admission_capability_denied_error_fields() {
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
fn admit_artifact_run_rejects_capability_superset() {
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
        artifact: accepted_artifact_with_caps(Box::new([required])),
    };
    let granted = CapabilitySet::from_grants(Box::new([
        Capability::new("network".into(), action),
        extra,
    ]));

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        test_digest(),
        granted,
    );

    assert_eq!(
        result,
        Err(AdmissionError::CapabilityCountMismatch {
            required_count: 1,
            granted_count: 2,
        })
    );
}

#[test]
fn admit_artifact_run_rejects_capability_duplicate() {
    // F-001 fix: strict admission rejects duplicate grants (cardinality mismatch).
    // RA-023 fix: surface as typed `CapabilityCountMismatch`, not fabricated denial.
    let action = ActionId::new(7);
    let required = Capability::new("network".into(), action);
    let store = FixedAcceptedStore {
        artifact: accepted_artifact_with_caps(Box::new([required.clone()])),
    };
    let granted =
        CapabilitySet::from_grants(Box::new([required.clone(), required.clone()]));

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        test_digest(),
        granted,
    );

    assert_eq!(
        result,
        Err(AdmissionError::CapabilityCountMismatch {
            required_count: 1,
            granted_count: 2,
        })
    );
}

#[test]
fn admit_artifact_run_count_mismatch_returns_typed_error_not_capability_denied() {
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
        artifact: accepted_artifact_with_caps(Box::new([required.clone()])),
    };
    let granted = CapabilitySet::from_grants(Box::new([required.clone(), extra]));

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        test_digest(),
        granted,
    );

    assert_eq!(
        result,
        Err(AdmissionError::CapabilityCountMismatch {
            required_count: 1,
            granted_count: 2,
        })
    );
}

#[test]
fn admit_artifact_run_count_mismatch_under_grant_returns_typed_error_not_per_cap_denial() {
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
        artifact: accepted_artifact_with_caps(Box::new([
            network.clone(),
            filesystem.clone(),
        ])),
    };
    let granted = CapabilitySet::from_grants(Box::new([network, filesystem, extra]));

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        test_digest(),
        granted,
    );

    assert_eq!(
        result,
        Err(AdmissionError::CapabilityCountMismatch {
            required_count: 2,
            granted_count: 3,
        })
    );
}

#[test]
fn admit_artifact_run_count_match_with_missing_capability_still_returns_capability_denied() {
    // RA-023 contract preservation: when cardinality is equal but membership
    // differs, the per-capability loop must fire and surface the actual
    // missing capability, NOT a count-mismatch error.
    let network = Capability::new("network.github".into(), ActionId::new(7));
    let filesystem = Capability::new("filesystem.read".into(), ActionId::new(8));
    let wrong = Capability::new("network.other".into(), ActionId::new(11));
    let store = FixedAcceptedStore {
        artifact: accepted_artifact_with_caps(Box::new([
            network.clone(),
            filesystem.clone(),
        ])),
    };
    // Granted has the right count (2) but the second cap is wrong.
    let granted = CapabilitySet::from_grants(Box::new([network, wrong]));

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        test_digest(),
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
}

#[test]
fn admit_artifact_run_accepts_capability_exact_match() {
    // F-001 fix: strict admission accepts exactly-equal capability sets.
    let action = ActionId::new(7);
    let required = Capability::new("network".into(), action);
    let store = FixedAcceptedStore {
        artifact: accepted_artifact_with_caps(Box::new([required.clone()])),
    };
    let granted = CapabilitySet::from_grants(Box::new([required.clone()]));

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        test_digest(),
        granted.clone(),
    );

    let admitted = result.expect("exact-match capability set must admit");
    assert_eq!(admitted.granted_capabilities(), &granted);
}

#[test]
fn admit_artifact_run_rejects_missing_grants_without_allocation() {
    let network = Capability::new("network.github".into(), ActionId::new(7));
    let filesystem = Capability::new("filesystem.read".into(), ActionId::new(8));
    let store = FixedAcceptedStore {
        artifact: accepted_artifact_with_caps(Box::new([network.clone(), filesystem.clone()])),
    };
    let granted = CapabilitySet::from_grants(Box::new([network.clone()]));

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        test_digest(),
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
}

#[test]
fn admit_artifact_run_rejects_non_exact_grant_without_allocation() {
    let action = ActionId::new(7);
    let required = Capability::new("network.github".into(), action);
    let store = FixedAcceptedStore {
        artifact: accepted_artifact_with_caps(Box::new([required.clone()])),
    };
    let granted = CapabilitySet::from_grants(Box::new([Capability::new("network".into(), action)]));

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        test_digest(),
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
}

#[test]
fn admit_artifact_run_preserves_non_empty_required_capabilities() {
    let action = ActionId::new(7);
    let required = Capability::new("network".into(), action);
    let store = FixedAcceptedStore {
        artifact: accepted_artifact_with_caps(Box::new([required.clone()])),
    };
    let granted = CapabilitySet::from_grants(Box::new([required]));

    let admission = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        test_digest(),
        granted.clone(),
    );

    assert!(matches!(admission, Ok(run) if run.granted_capabilities() == &granted));
}

#[test]
fn admit_artifact_run_rejects_missing_idempotency_gate() {
    let mut artifact = accepted_artifact_with_caps(Box::new([]));
    artifact.verification.idempotency_verified_claimed = false;
    let store = FixedAcceptedStore { artifact };

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        test_digest(),
        CapabilitySet::empty(),
    );

    assert_eq!(
        result,
        Err(AdmissionError::ArtifactInvalidProofFlag {
            flag: "idempotency_verified",
        })
    );
}

#[test]
fn admit_artifact_run_rejects_keyed_action_without_attestation() {
    let action = ActionId::new(9);
    let mut artifact = accepted_artifact_with_caps(Box::new([]));
    artifact.verification.idempotency_keyed = Box::new([action]);
    artifact.verification.idempotency_attested = Box::new([]);
    let store = FixedAcceptedStore { artifact };

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        test_digest(),
        CapabilitySet::empty(),
    );

    assert_eq!(
        result,
        Err(AdmissionError::ArtifactInvalidProofFlag {
            flag: "idempotency_attested",
        })
    );
}

#[test]
fn admit_artifact_run_carries_idempotency_evidence_to_dispatch() {
    let action = ActionId::new(9);
    let mut artifact = accepted_artifact_with_caps(Box::new([]));
    artifact.verification.idempotency_keyed = Box::new([action]);
    artifact.verification.idempotency_attested = Box::new([action]);
    let store = FixedAcceptedStore { artifact };

    let admission = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        test_digest(),
        CapabilitySet::empty(),
    );

    assert!(matches!(admission, Ok(run) if run.idempotency_attested() == [action]));
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
fn admission_admit_run_strict_with_present_artifact() {
    let store = AlwaysPresentArtifactStore::shared();
    let digest = test_digest();
    let run_id = RunId::new(1);
    let caps = CapabilitySet::from_grants(Box::new([Capability::new("".into(), ActionId::new(0))]));
    let result = admit_run(
        store.as_ref(),
        RuntimePolicy::Strict,
        digest,
        run_id,
        caps.clone(),
    );
    let admission = RunAdmission::new(digest, run_id, caps, RuntimePolicy::Strict);
    assert_eq!(result, Ok(admission.clone()));
    assert_eq!(admission.artifact_digest(), digest);
    assert_eq!(admission.run_id(), run_id);
    assert_eq!(admission.policy(), RuntimePolicy::Strict);
}

#[test]
fn admission_admit_run_strict_rejects_loaded_digest_mismatch() {
    struct MismatchedAcceptedStore;
    impl AcceptedArtifactStore for MismatchedAcceptedStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            let found = WorkflowDigest::from_bytes([0x42; 32]);
            let mut artifact = accepted_artifact_with_caps(Box::new([]));
            artifact.digest = found;
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
        Err(AdmissionError::ArtifactDigestMismatch {
            requested,
            found: WorkflowDigest::from_bytes([0x42; 32]),
        })
    );
}

#[test]
fn admission_admit_run_strict_rejects_loaded_proof_digest_mismatch() {
    struct MismatchedProofStore;
    impl AcceptedArtifactStore for MismatchedProofStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            let mut artifact = accepted_artifact_with_caps(Box::new([]));
            artifact.verification.digest = WorkflowDigest::from_bytes([0x42; 32]);
            Ok(artifact)
        }
    }
    let requested = test_digest();

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
fn admit_artifact_run_rejects_missing_bounded_flag() {
    /// Store that returns artifact with bounded=false.
    struct BoundedFalseStore;
    impl AcceptedArtifactStore for BoundedFalseStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            let mut artifact = accepted_artifact_with_caps(Box::new([]));
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
        Err(AdmissionError::ArtifactInvalidProofFlag { flag: "bounded" })
    );
}

#[test]
fn admit_artifact_run_rejects_missing_taint_safe_flag() {
    struct TaintSafeFalseStore;
    impl AcceptedArtifactStore for TaintSafeFalseStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            let mut artifact = accepted_artifact_with_caps(Box::new([]));
            artifact.verification.taint_safe_claimed = false;
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
        Err(AdmissionError::ArtifactInvalidProofFlag { flag: "taint_safe" })
    );
}

#[test]
fn admit_artifact_run_rejects_missing_retry_safe_flag() {
    struct RetrySafeFalseStore;
    impl AcceptedArtifactStore for RetrySafeFalseStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            let mut artifact = accepted_artifact_with_caps(Box::new([]));
            artifact.verification.retry_safe_claimed = false;
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
        Err(AdmissionError::ArtifactInvalidProofFlag { flag: "retry_safe" })
    );
}

#[test]
fn admit_artifact_run_rejects_missing_durable_flag() {
    struct DurableFalseStore;
    impl AcceptedArtifactStore for DurableFalseStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            let mut artifact = accepted_artifact_with_caps(Box::new([]));
            artifact.verification.durable = false;
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
        Err(AdmissionError::ArtifactInvalidProofFlag { flag: "durable" })
    );
}

#[test]
fn admit_artifact_run_rejects_missing_replayable_flag() {
    struct ReplayableFalseStore;
    impl AcceptedArtifactStore for ReplayableFalseStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            let mut artifact = accepted_artifact_with_caps(Box::new([]));
            artifact.verification.replayable_claimed = false;
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
        Err(AdmissionError::ArtifactInvalidProofFlag { flag: "replayable" })
    );
}

#[test]
fn admit_artifact_run_rejects_missing_idempotency_verified_flag() {
    struct IdempotencyVerifiedFalseStore;
    impl AcceptedArtifactStore for IdempotencyVerifiedFalseStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            let mut artifact = accepted_artifact_with_caps(Box::new([]));
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
        Err(AdmissionError::ArtifactInvalidProofFlag {
            flag: "idempotency_verified"
        })
    );
}

#[test]
fn admit_artifact_run_rejects_missing_idempotency_attestation() {
    /// Store that returns artifact with idempotency_keyed but no attestation.
    struct MissingAttestationStore {
        action: ActionId,
    }
    impl AcceptedArtifactStore for MissingAttestationStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            let mut artifact = accepted_artifact_with_caps(Box::new([]));
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
        Err(AdmissionError::ArtifactInvalidProofFlag {
            flag: "idempotency_attested"
        })
    );
}

// -------------------------------------------------------------------------
// admit_artifact_run digest mismatch (B-11)
// -------------------------------------------------------------------------

#[test]
fn admit_artifact_run_returns_digest_mismatch() {
    /// Store that returns an artifact with a different digest than requested.
    struct MismatchedDigestStore;
    impl AcceptedArtifactStore for MismatchedDigestStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            let wrong_digest = WorkflowDigest::from_bytes([0x42; 32]);
            let mut artifact = accepted_artifact_with_caps(Box::new([]));
            artifact.digest = wrong_digest;
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
        Err(AdmissionError::ArtifactDigestMismatch {
            requested,
            found: WorkflowDigest::from_bytes([0x42; 32])
        })
    );
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
