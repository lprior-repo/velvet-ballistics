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
    let admitted_digest = match policy {
        RuntimePolicy::Strict | RuntimePolicy::Journaled => {
            let artifact = store
                .load_accepted_artifact(digest)
                .map_err(map_artifact_envelope_error)?;
            validate_accepted_artifact_envelope(&artifact).map_err(map_artifact_envelope_error)?;
            if artifact.digest != digest && artifact.source_digest != digest {
                return Err(AdmissionError::ArtifactDigestMismatch {
                    requested: digest,
                    found: artifact.digest,
                });
            }
            artifact.digest
        }
        RuntimePolicy::Relaxed => digest,
        _ => {
            return Err(AdmissionError::ArtifactInvalidProofFlag {
                flag: "runtime_policy",
            });
        }
    };
    Ok(RunAdmission::new(admitted_digest, run_id, caps, policy))
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
        RuntimePolicy::Strict | RuntimePolicy::Journaled => {
            // Load and validate the full artifact.
            let artifact = store
                .load_accepted_artifact(artifact_digest)
                .map_err(map_artifact_envelope_error)?;
            validate_accepted_artifact_envelope(&artifact).map_err(map_artifact_envelope_error)?;

            // INV-002: digest binding must be total. The loaded artifact's digest
            // must match the requested digest exactly — a crafted artifact with
            // valid gates but wrong identity must not be admitted.
            if artifact.digest != artifact_digest && artifact.source_digest != artifact_digest {
                return Err(AdmissionError::ArtifactDigestMismatch {
                    requested: artifact_digest,
                    found: artifact.digest,
                });
            }

            // INV-003: proof digest must match artifact digest. The verification
            // proof's digest field must bind to the artifact content exactly.
            if artifact.verification.digest != artifact.digest {
                return Err(AdmissionError::ArtifactDigestMismatch {
                    requested: artifact_digest,
                    found: artifact.verification.digest,
                });
            }

            if artifact.accepted_at_seq < required_at_least {
                return Err(AdmissionError::ArtifactCertificateStale {
                    digest: artifact_digest,
                    accepted_at_seq: artifact.accepted_at_seq,
                    required_at_least,
                });
            }

            // spec: VERUS-CARD-003 strict equality (cardinality-exact + membership-exact admission).
            // F-001 fix: restore strict capability equality (VERUS-CARD-003).
            //
            // Strict admission requires that granted capabilities exactly match the
            // artifact's declared required capabilities — same cardinality AND same
            // membership. Subset grants with extras are rejected because the runtime
            // re-checks `RunAdmission::granted_capabilities` per action in
            // `engine::action::execute_do`; preserving extras would let an over-granted
            // capability authorize actions the artifact never declared, violating
            // least-privilege and contradicting the cardinality-exact Verus proof
            // model (`verification/verus/capability_artifact_model.rs::exact_profile`).
            //
            // Order: per-required subset check runs first so under-grant reports the
            // specific missing capability; only when every required is covered do we
            // gate on cardinality to reject over-grants (extras / duplicates).
            //
            // RA-023 fix: when cardinality differs, surface a typed
            // `CapabilityCountMismatch { required_count, granted_count }` instead of
            // fabricating a `CapabilityDenied` that names a granted capability as the
            // missing one. Honesty-preserving error variants matter for the operator
            // diagnostic surface (admission_result / RuntimeError mapping).
            for required_cap in artifact.required_capabilities.iter() {
                check_capability(required_cap.action_id(), required_cap, &caps)?;
            }
            let required_count = artifact.required_capabilities.len();
            let granted_count = caps.len();
            if required_count != granted_count {
                return Err(AdmissionError::CapabilityCountMismatch {
                    required_count,
                    granted_count,
                });
            }

            let admitted_digest = artifact.digest;
            Ok(RunAdmission::with_idempotency_evidence(
                admitted_digest,
                run_id,
                caps,
                policy,
                artifact.verification.idempotency_attested,
            ))
        }
        RuntimePolicy::Relaxed => {
            // Relaxed: skip artifact loading and capability checking.
            Ok(RunAdmission::new(artifact_digest, run_id, caps, policy))
        }
        _ => Err(AdmissionError::ArtifactInvalidProofFlag {
            flag: "runtime_policy",
        }),
    }
}
