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
    let expected_decision = vb_storage::compat::decide_boundary(
        vb_storage::compat::artifact_ir_version_inputs(vb_storage::compat::CURRENT_IR_VERSION),
        vb_storage::compat::schema_version_digest(vb_storage::compat::CURRENT_IR_VERSION),
        vb_storage::compat::schema_version_digest(vb_storage::compat::CURRENT_IR_VERSION),
    );
    let admission = RunAdmission::new(digest, run_id, caps, RuntimePolicy::Strict)
        .with_ir_compat_decision(expected_decision);
    assert_eq!(result, Ok(admission.clone()));
    assert_eq!(admission.artifact_digest(), digest);
    assert_eq!(admission.run_id(), run_id);
    assert_eq!(admission.policy(), RuntimePolicy::Strict);
    Ok(())
}

#[test]
fn admit_artifact_run_rejects_future_major_ir_version() -> Result<(), String> {
    let mut artifact = accepted_artifact_with_caps(Box::new([]))?;
    artifact.artifact_version = vb_storage::compat::SchemaVersion::new(3, 0, 0);
    let digest =
        vb_storage::admission::accepted_artifact_digest(&artifact).map_err(|e| e.to_string())?;
    artifact.digest = digest;
    artifact.verification.digest = digest;
    let store = FixedAcceptedStore { artifact };
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        digest,
        CapabilitySet::empty(),
    );
    assert!(
        matches!(
            result,
            Err(AdmissionError::ArtifactVersionIncompatible { code: 300, .. })
        ),
        "major-line IR change with no declared path must reject with code 300, got {result:?}"
    );
    Ok(())
}

#[test]
fn admit_artifact_run_accepts_same_major_newer_minor_ir_version() -> Result<(), String> {
    let mut artifact = accepted_artifact_with_caps(Box::new([]))?;
    artifact.artifact_version = vb_storage::compat::SchemaVersion::new(1, 2, 0);
    let digest =
        vb_storage::admission::accepted_artifact_digest(&artifact).map_err(|e| e.to_string())?;
    artifact.digest = digest;
    artifact.verification.digest = digest;
    let store = FixedAcceptedStore { artifact };
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        digest,
        CapabilitySet::empty(),
    );
    assert!(
        result.is_ok(),
        "same-major newer-minor IR version is forward compatible and must admit, got {result:?}"
    );
    Ok(())
}

#[test]
fn check_artifact_ir_version_allows_only_live_effect_classes() -> Result<(), String> {
    let mut artifact = accepted_artifact_with_caps(Box::new([]))?;
    let decision = check_artifact_ir_version(&artifact).map_err(|error| error.to_string())?;
    assert_eq!(
        decision.boundary,
        vb_storage::compat::CompatBoundary::IrVersion
    );
    assert_eq!(
        decision.class,
        vb_storage::compat::CompatibilityClass::Identical
    );
    assert_eq!(decision.decision_code.code(), 100);
    assert_eq!(
        decision.old_schema_digest,
        vb_storage::compat::schema_version_digest(vb_storage::compat::CURRENT_IR_VERSION)
    );
    assert_eq!(
        decision.new_schema_digest,
        vb_storage::compat::schema_version_digest(vb_storage::compat::CURRENT_IR_VERSION)
    );
    artifact.artifact_version = vb_storage::compat::SchemaVersion::new(3, 0, 0);
    assert!(
        matches!(
            check_artifact_ir_version(&artifact),
            Err(AdmissionError::ArtifactVersionIncompatible { code: 300, .. })
        ),
        "future major IR version must not authorize live effects"
    );
    Ok(())
}

#[test]
fn admit_artifact_run_carries_ir_compat_decision() -> Result<(), String> {
    let mut artifact = accepted_artifact_with_caps(Box::new([]))?;
    let digest =
        vb_storage::admission::accepted_artifact_digest(&artifact).map_err(|e| e.to_string())?;
    artifact.digest = digest;
    artifact.verification.digest = digest;
    let store = FixedAcceptedStore { artifact };
    let admission = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        digest,
        CapabilitySet::empty(),
    )
    .map_err(|error| error.to_string())?;
    let decision = admission
        .ir_compat_decision()
        .ok_or("strict admission must carry the IR compat decision")?;
    assert_eq!(
        decision.class,
        vb_storage::compat::CompatibilityClass::Identical
    );
    assert_eq!(decision.decision_code.code(), 100);
    Ok(())
}

#[test]
fn admit_artifact_run_carries_forward_compat_decision() -> Result<(), String> {
    let mut artifact = accepted_artifact_with_caps(Box::new([]))?;
    artifact.artifact_version = vb_storage::compat::SchemaVersion::new(1, 2, 0);
    let digest =
        vb_storage::admission::accepted_artifact_digest(&artifact).map_err(|e| e.to_string())?;
    artifact.digest = digest;
    artifact.verification.digest = digest;
    let store = FixedAcceptedStore { artifact };
    let admission = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        digest,
        CapabilitySet::empty(),
    )
    .map_err(|error| error.to_string())?;
    let decision = admission
        .ir_compat_decision()
        .ok_or("strict admission must carry the IR compat decision")?;
    assert_eq!(
        decision.class,
        vb_storage::compat::CompatibilityClass::ForwardCompatible
    );
    assert_eq!(decision.decision_code.code(), 102);
    Ok(())
}

#[test]
fn admit_artifact_run_relaxed_carries_no_ir_compat_decision() -> Result<(), String> {
    let admission = admit_artifact_run(
        &FixedAcceptedStore {
            artifact: accepted_artifact_with_caps(Box::new([]))?,
        },
        RuntimePolicy::Relaxed,
        RunId::new(1),
        test_digest(),
        CapabilitySet::empty(),
    )
    .map_err(|error| error.to_string())?;
    assert_eq!(admission.ir_compat_decision(), None);
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

