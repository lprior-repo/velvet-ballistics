fn validate_accepted_artifact_envelope(
    store: &dyn AcceptedArtifactStore,
    artifact: &vb_storage::admission::AcceptedArtifact,
) -> Result<(), ArtifactEnvelopeError> {
    if artifact.verification.digest != artifact.digest {
        return Err(ArtifactEnvelopeError::ArtifactDigestMismatch {
            requested: artifact.digest,
            found: artifact.verification.digest,
        });
    }
    verify_stored_artifact(store, artifact, None).map(|_| ())
}

/// Shared §77.17 live-effect gate for the admission call sites in
/// `chunk_005` and the submit/dispatch binding in
/// `shard::lifecycle::execution_binding`.
pub(crate) fn check_artifact_ir_version(
    artifact: &vb_storage::admission::AcceptedArtifact,
) -> Result<(), AdmissionError> {
    let classification = vb_core::compat::classify(vb_storage::compat::artifact_ir_version_inputs(
        artifact.artifact_version,
    ));
    if classification.class.authorizes_live_effect() {
        Ok(())
    } else {
        Err(AdmissionError::ArtifactVersionIncompatible {
            held: artifact.artifact_version,
            required: vb_core::compat::CURRENT_IR_VERSION,
            code: classification.code.code(),
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
