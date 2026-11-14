/// Performs the admission gate check for a submit.
///
/// - Strict / Journaled: artifact must exist in the store.
/// - Relaxed: always succeeds.
///
/// Returns a `RunAdmission` on success or an `AdmissionError` on rejection.
pub fn admit_run(
    store: &dyn AcceptedArtifactStore,
    policy: RuntimePolicy,
    digest: WorkflowDigest,
    run_id: RunId,
    caps: CapabilitySet,
) -> Result<RunAdmission, AdmissionError> {
    let (admitted_digest, decision) = match policy {
        RuntimePolicy::Strict | RuntimePolicy::Journaled => {
            admit_run_artifact(store, policy, digest)?
        }
        RuntimePolicy::Relaxed => (digest, None),
        _ => {
            return Err(AdmissionError::ArtifactInvalidProofFlag {
                flag: "runtime_policy",
            });
        }
    };
    let mut admission = RunAdmission::new(admitted_digest, run_id, caps, policy);
    if let Some(decision) = decision {
        admission = admission.with_ir_compat_decision(decision);
    }
    Ok(admission)
}

fn admit_run_artifact(
    store: &dyn AcceptedArtifactStore,
    policy: RuntimePolicy,
    digest: WorkflowDigest,
) -> Result<(WorkflowDigest, Option<vb_storage::compat::CompatDecision>), AdmissionError> {
    let artifact = store
        .load_accepted_artifact(digest)
        .map_err(map_artifact_envelope_error)?;
    validate_accepted_artifact_envelope(store, &artifact).map_err(map_artifact_envelope_error)?;
    verify_stored_artifact(store, &artifact, Some(policy)).map_err(|_| {
        AdmissionError::ArtifactInvalidProofFlag {
            flag: "admission_policy",
        }
    })?;
    let decision = check_artifact_ir_version(&artifact)?;
    if artifact.digest != digest && artifact.source_digest != digest {
        return Err(AdmissionError::ArtifactDigestMismatch {
            requested: digest,
            found: artifact.digest,
        });
    }
    Ok((artifact.digest, Some(decision)))
}

/// Performs full admission gate check with artifact validation before run creation.
///
/// For `RuntimePolicy::Strict` and `RuntimePolicy::Journaled`:
///   - Loads and validates the accepted artifact from storage
///   - Checks that the artifact has all 15 gates passing and proof flags set
///   - Validates that granted capabilities cover the artifact's required capabilities
///
/// For `RuntimePolicy::Relaxed`:
///   - Skips artifact loading and capability checking
///   - Returns a lightweight RunAdmission with no budget
///
/// Returns `Ok(RunAdmission)` on success, or an `AdmissionError` on rejection.
/// On error, no run frame is allocated, no run state is inserted, and no
/// `RunAccepted` journal event is recorded.
pub fn admit_artifact_run(
    store: &dyn AcceptedArtifactStore,
    policy: RuntimePolicy,
    run_id: RunId,
    artifact_digest: WorkflowDigest,
    caps: CapabilitySet,
) -> Result<RunAdmission, AdmissionError> {
    admit_artifact_run_with_certificate_floor(
        store,
        policy,
        run_id,
        artifact_digest,
        caps,
        EventSeq::ZERO,
    )
}

/// Performs full artifact admission with a caller-supplied certificate freshness floor.
///
/// This preserves relaxed-mode behavior and rejects Strict/Journaled artifacts whose
/// `accepted_at_seq` is below `required_at_least` after envelope validation.
pub fn admit_artifact_run_with_certificate_floor(
    store: &dyn AcceptedArtifactStore,
    policy: RuntimePolicy,
    run_id: RunId,
    artifact_digest: WorkflowDigest,
    caps: CapabilitySet,
    required_at_least: EventSeq,
) -> Result<RunAdmission, AdmissionError> {
    match policy {
        RuntimePolicy::Strict | RuntimePolicy::Journaled => admit_strict_artifact_run(
            store,
            run_id,
            artifact_digest,
            caps,
            policy,
            required_at_least,
        ),
        RuntimePolicy::Relaxed => Ok(RunAdmission::new(artifact_digest, run_id, caps, policy)),
        _ => Err(AdmissionError::ArtifactInvalidProofFlag {
            flag: "runtime_policy",
        }),
    }
}

fn admit_strict_artifact_run(
    store: &dyn AcceptedArtifactStore,
    run_id: RunId,
    artifact_digest: WorkflowDigest,
    caps: CapabilitySet,
    policy: RuntimePolicy,
    required_at_least: EventSeq,
) -> Result<RunAdmission, AdmissionError> {
    let (artifact, decision) = load_admission_artifact(store, artifact_digest, policy)?;
    verify_admission_digest(&artifact, artifact_digest)?;
    verify_certificate_floor(&artifact, artifact_digest, required_at_least)?;
    verify_admission_capabilities(&artifact, &caps)?;
    Ok(admission_from_artifact(
        artifact, run_id, caps, policy, decision,
    ))
}

fn load_admission_artifact(
    store: &dyn AcceptedArtifactStore,
    artifact_digest: WorkflowDigest,
    policy: RuntimePolicy,
) -> Result<
    (
        vb_storage::admission::AcceptedArtifact,
        vb_storage::compat::CompatDecision,
    ),
    AdmissionError,
> {
    let artifact = store
        .load_accepted_artifact(artifact_digest)
        .map_err(map_artifact_envelope_error)?;
    validate_accepted_artifact_envelope(store, &artifact).map_err(map_artifact_envelope_error)?;
    verify_stored_artifact(store, &artifact, Some(policy)).map_err(|_| {
        AdmissionError::ArtifactInvalidProofFlag {
            flag: "admission_policy",
        }
    })?;
    let decision = check_artifact_ir_version(&artifact)?;
    Ok((artifact, decision))
}

fn verify_admission_digest(
    artifact: &vb_storage::admission::AcceptedArtifact,
    artifact_digest: WorkflowDigest,
) -> Result<(), AdmissionError> {
    if artifact.digest != artifact_digest && artifact.source_digest != artifact_digest {
        return Err(AdmissionError::ArtifactDigestMismatch {
            requested: artifact_digest,
            found: artifact.digest,
        });
    }
    if artifact.verification.digest != artifact.digest {
        return Err(AdmissionError::ArtifactDigestMismatch {
            requested: artifact_digest,
            found: artifact.verification.digest,
        });
    }
    Ok(())
}

fn verify_certificate_floor(
    artifact: &vb_storage::admission::AcceptedArtifact,
    artifact_digest: WorkflowDigest,
    required_at_least: EventSeq,
) -> Result<(), AdmissionError> {
    if artifact.accepted_at_seq < required_at_least {
        return Err(AdmissionError::ArtifactCertificateStale {
            digest: artifact_digest,
            accepted_at_seq: artifact.accepted_at_seq,
            required_at_least,
        });
    }
    Ok(())
}

fn verify_admission_capabilities(
    artifact: &vb_storage::admission::AcceptedArtifact,
    caps: &CapabilitySet,
) -> Result<(), AdmissionError> {
    for required_cap in artifact.required_capabilities.iter() {
        check_capability(required_cap.action_id(), required_cap, caps)?;
    }
    let required_count = artifact.required_capabilities.len();
    let granted_count = caps.len();
    if required_count != granted_count {
        return Err(AdmissionError::CapabilityCountMismatch {
            required_count,
            granted_count,
        });
    }
    Ok(())
}

fn admission_from_artifact(
    artifact: vb_storage::admission::AcceptedArtifact,
    run_id: RunId,
    caps: CapabilitySet,
    policy: RuntimePolicy,
    decision: vb_storage::compat::CompatDecision,
) -> RunAdmission {
    RunAdmission::with_idempotency_evidence(
        artifact.digest,
        run_id,
        caps,
        policy,
        artifact.verification.idempotency_attested,
    )
    .with_policy_pin(RunPolicyPin::Pinned(artifact.policy_digest))
    .with_ir_compat_decision(decision)
}
