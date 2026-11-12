fn validate_accepted_artifact_envelope(
    store: &dyn AcceptedArtifactStore,
    artifact: &vb_storage::admission::AcceptedArtifact,
) -> Result<(), ArtifactEnvelopeError> {
    validate_artifact_proof_digest(artifact)?;
    verify_stored_artifact(store, artifact, None).map(|_| ())
}

fn validate_artifact_proof_digest(
    artifact: &vb_storage::admission::AcceptedArtifact,
) -> Result<(), ArtifactEnvelopeError> {
    if artifact.verification.digest != artifact.digest {
        return Err(ArtifactEnvelopeError::ArtifactDigestMismatch {
            requested: artifact.digest,
            found: artifact.verification.digest,
        });
    }
    Ok(())
}

fn validate_artifact_outer_digest(
    artifact: &vb_storage::admission::AcceptedArtifact,
) -> Result<(), ArtifactEnvelopeError> {
    let len = u32::try_from(artifact.ir.len())
        .map_err(|_| ArtifactEnvelopeError::ArtifactEvidenceInvalid)?;
    if len > vb_storage::constants::MAX_COMPILED_IR_BYTES {
        return Err(ArtifactEnvelopeError::ArtifactEvidenceInvalid);
    }
    let digest = vb_storage::admission::accepted_artifact_digest(artifact)
        .map_err(|_| ArtifactEnvelopeError::ArtifactEvidenceInvalid)?;
    if digest != artifact.digest {
        return Err(ArtifactEnvelopeError::ArtifactEvidenceInvalid);
    }
    Ok(())
}

fn checked_admission_ir_version(
    artifact: &vb_storage::admission::AcceptedArtifact,
) -> Result<vb_storage::compat::CompatDecision, AdmissionError> {
    validate_artifact_proof_digest(artifact).map_err(map_artifact_envelope_error)?;
    validate_artifact_outer_digest(artifact).map_err(map_artifact_envelope_error)?;
    check_artifact_ir_version(artifact)
}

fn required_artifact_ir_version(
    held: vb_storage::compat::SchemaVersion,
) -> vb_storage::compat::SchemaVersion {
    if held.major == 1 {
        vb_storage::compat::CURRENT_IR_VERSION
    } else {
        vb_storage::compat::SchemaVersion::new(2, 0, 0)
    }
}

/// Shared §77.17 live-effect gate for the admission call sites in
/// `chunk_005` and the submit/dispatch binding in
/// `shard::lifecycle::execution_binding`.
///
/// Returns the journaled-shape decision record on success so callers on
/// the submit path can persist it; the decision carries the class,
/// deciding-rule digest, old/new schema digests, and decision code the
/// spec requires on the durable record.
pub(crate) fn check_artifact_ir_version(
    artifact: &vb_storage::admission::AcceptedArtifact,
) -> Result<vb_storage::compat::CompatDecision, AdmissionError> {
    let required = required_artifact_ir_version(artifact.artifact_version);
    let decision = vb_storage::compat::decide_boundary(
        vb_storage::compat::artifact_ir_version_inputs_for(artifact.artifact_version, required),
        vb_storage::compat::schema_version_digest(artifact.artifact_version),
        vb_storage::compat::schema_version_digest(required),
    );
    if decision.authorize_live_effect().is_ok() {
        Ok(decision)
    } else {
        Err(AdmissionError::ArtifactVersionIncompatible {
            held: artifact.artifact_version,
            required,
            code: decision.decision_code.code(),
        })
    }
}

fn map_artifact_envelope_error(source: ArtifactEnvelopeError) -> AdmissionError {
    match source {
        ArtifactEnvelopeError::ArtifactNotFound { digest } => {
            AdmissionError::ArtifactNotFound { digest }
        }
        ArtifactEnvelopeError::PostcardDecodeFailed => AdmissionError::ArtifactEnvelopeDecodeFailed,
        ArtifactEnvelopeError::ArtifactEvidenceInvalid => {
            AdmissionError::ArtifactInvalidProofFlag { flag: "evidence" }
        }
        ArtifactEnvelopeError::InvalidGateCount { found, required } => {
            AdmissionError::ArtifactInvalidGateCount { found, required }
        }
        ArtifactEnvelopeError::MissingRequiredProofFlagBounded => {
            AdmissionError::ArtifactInvalidProofFlag { flag: "bounded" }
        }
        ArtifactEnvelopeError::MissingRequiredProofFlagTaintSafe => {
            AdmissionError::ArtifactInvalidProofFlag { flag: "taint_safe" }
        }
        ArtifactEnvelopeError::MissingRequiredProofFlagRetrySafe => {
            AdmissionError::ArtifactInvalidProofFlag { flag: "retry_safe" }
        }
        ArtifactEnvelopeError::MissingRequiredProofFlagDurable => {
            AdmissionError::ArtifactInvalidProofFlag { flag: "durable" }
        }
        ArtifactEnvelopeError::MissingRequiredProofFlagReplayable => {
            AdmissionError::ArtifactInvalidProofFlag { flag: "replayable" }
        }
        ArtifactEnvelopeError::MissingRequiredProofFlagIdempotencyVerified => {
            AdmissionError::ArtifactInvalidProofFlag {
                flag: "idempotency_verified",
            }
        }
        ArtifactEnvelopeError::MissingIdempotencyAttestation { .. } => {
            AdmissionError::ArtifactInvalidProofFlag {
                flag: "idempotency_attested",
            }
        }
        ArtifactEnvelopeError::ArtifactDigestMismatch { requested, found } => {
            AdmissionError::ArtifactDigestMismatch { requested, found }
        }
    }
}
