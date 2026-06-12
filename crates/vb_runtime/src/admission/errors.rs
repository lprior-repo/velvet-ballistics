#![forbid(unsafe_code)]
//! Error types for runtime admission control.

use thiserror::Error;
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, WorkflowDigest};
use vb_storage::EventSeq;

/// Artifact envelope validation errors for runtime admission.
///
/// These errors are raised when a stored compiled artifact fails semantic
/// validation before a run can be admitted under Strict or Journaled policy.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ArtifactEnvelopeError {
    /// Artifact was not found in the store.
    #[error("artifact not found: {digest:?}")]
    ArtifactNotFound {
        /// Digest that was looked up.
        digest: WorkflowDigest,
    },
    /// Artifact failed envelope deserialization.
    #[error("artifact envelope decode failed")]
    PostcardDecodeFailed,
    /// Verification gate count is not 15.
    #[error("invalid gate count: found {found}, required {required}")]
    InvalidGateCount {
        /// Found gate count.
        found: u8,
        /// Required gate count.
        required: u8,
    },
    /// A required proof flag is false.
    #[error("missing required proof flag: bounded")]
    MissingRequiredProofFlagBounded,
    /// A required proof flag is false.
    #[error("missing required proof flag: taint_safe")]
    MissingRequiredProofFlagTaintSafe,
    /// A required proof flag is false.
    #[error("missing required proof flag: retry_safe")]
    MissingRequiredProofFlagRetrySafe,
    /// A required proof flag is false.
    #[error("missing required proof flag: durable")]
    MissingRequiredProofFlagDurable,
    /// A required proof flag is false.
    #[error("missing required proof flag: replayable")]
    MissingRequiredProofFlagReplayable,
    /// A required proof flag is false.
    #[error("missing required proof flag: idempotency_verified")]
    MissingRequiredProofFlagIdempotencyVerified,
    /// A keyed action was not present in the attested idempotency evidence.
    #[error("missing idempotency attestation for action {action:?}")]
    MissingIdempotencyAttestation {
        /// Action requiring idempotency attestation.
        action: ActionId,
    },
    /// The verification proof digest does not match the accepted artifact digest.
    #[error("artifact verification digest mismatch: requested {requested:?}, found {found:?}")]
    ArtifactDigestMismatch {
        /// Digest found in the accepted artifact envelope.
        requested: WorkflowDigest,
        /// Digest found in the verification proof.
        found: WorkflowDigest,
    },
}

/// Errors that can occur during run admission.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum AdmissionError {
    /// The required compiled artifact was not found in the journal.
    #[error("admission rejected: compiled artifact not found for digest {digest:?}")]
    ArtifactNotFound {
        /// Digest of the artifact that was expected.
        digest: WorkflowDigest,
    },
    /// The run requires a capability that was not granted.
    #[error("admission rejected: capability denied for action {action:?}")]
    CapabilityDenied {
        /// Action that required the capability.
        action: ActionId,
        /// Capability that was required but not granted.
        required: Capability,
        /// Capabilities that were granted at admission time.
        granted: CapabilitySet,
    },
    /// The requested aggregate budget exceeds shard capacity.
    #[error(
        "admission rejected: resource capacity exceeded for {resource}: {requested} > {available}"
    )]
    ResourceCapacityExceeded {
        /// Resource dimension that failed comparison.
        resource: &'static str,
        /// Requested aggregate amount.
        requested: u64,
        /// Available aggregate amount.
        available: u64,
    },
    /// The requested aggregate budget exceeds admission policy.
    #[error("admission rejected: budget policy exceeded for {resource}: {actual} > {limit}")]
    BudgetPolicyExceeded {
        /// Resource dimension that failed comparison.
        resource: &'static str,
        /// Actual aggregate amount.
        actual: u64,
        /// Policy limit.
        limit: u64,
    },
    /// Aggregate budget arithmetic overflowed before admission could reserve capacity.
    #[error("admission rejected: aggregate budget overflow for {resource}")]
    ResourceBudgetOverflow {
        /// Resource dimension that overflowed.
        resource: &'static str,
    },
    /// Aggregate budget arithmetic underflowed before admission could release capacity.
    #[error("admission rejected: aggregate budget underflow for {resource}")]
    ResourceBudgetUnderflow {
        /// Resource dimension that underflowed.
        resource: &'static str,
    },
    /// Aggregate budget capacity configuration is invalid.
    #[error("admission rejected: invalid aggregate capacity for {resource}")]
    ResourceBudgetInvalidCapacity {
        /// Resource dimension with invalid capacity.
        resource: &'static str,
    },
    /// Per-tick step ceiling is invalid or exceeded.
    #[error("admission rejected: step ceiling exceeded: {requested} > {limit}")]
    ResourceStepCeilingExceeded {
        /// Requested steps per tick.
        requested: u64,
        /// Ceiling limit.
        limit: u64,
    },
    /// Per-tick transition ceiling is invalid or exceeded.
    #[error("admission rejected: transition ceiling exceeded: {requested} > {limit}")]
    ResourcePerTickCeilingExceeded {
        /// Requested transitions per tick.
        requested: u64,
        /// Ceiling limit.
        limit: u64,
    },
    /// Artifact envelope failed to decode as a valid accepted artifact.
    #[error("admission rejected: artifact envelope decode failed")]
    ArtifactEnvelopeDecodeFailed,
    /// Artifact has an invalid gate count for v1 admission.
    #[error("admission rejected: artifact gate count {found} != {required}")]
    ArtifactInvalidGateCount {
        /// Found gate count.
        found: u8,
        /// Required gate count.
        required: u8,
    },
    /// Artifact has a proof flag that is false.
    #[error("admission rejected: artifact proof flag {flag} is false")]
    ArtifactInvalidProofFlag {
        /// Name of the false flag.
        flag: &'static str,
    },
    /// The loaded artifact digest does not match the requested digest.
    #[error(
        "admission rejected: artifact digest mismatch: requested {requested:?}, found {found:?}"
    )]
    ArtifactDigestMismatch {
        /// Digest that was requested at admission.
        requested: WorkflowDigest,
        /// Digest found inside the loaded artifact envelope.
        found: WorkflowDigest,
    },
    /// The loaded artifact certificate is older than the caller's freshness floor.
    #[error(
        "admission rejected: artifact certificate stale for digest {digest:?}: accepted_at_seq {accepted_at_seq:?} < required_at_least {required_at_least:?}"
    )]
    ArtifactCertificateStale {
        /// Digest whose certificate was too old.
        digest: WorkflowDigest,
        /// Sequence at which the artifact was accepted.
        accepted_at_seq: EventSeq,
        /// Minimum accepted sequence required by the caller.
        required_at_least: EventSeq,
    },
    /// Workflow step count exceeds the master contract per-workflow ceiling
    /// (`vb_core::limits::MAX_STEPS_PER_WORKFLOW = 1_000`).
    ///
    /// Returned by `preflight_step_budget` when the compiled workflow declares
    /// a `ResourceContract::max_steps` above the limit. This is the typed,
    /// step-count-specific failure that the production admission preflight
    /// surfaces in place of a generic `BudgetPolicyExceeded` so the runtime
    /// can fail closed before any persistence.
    #[error("admission rejected: workflow step count {actual} exceeds per-workflow ceiling {limit}")]
    BudgetExceeded {
        /// Step count declared by the workflow's `ResourceContract::max_steps`.
        actual: u32,
        /// Per-workflow ceiling from `vb_core::limits::MAX_STEPS_PER_WORKFLOW`.
        limit: u32,
    },
}

/// Maps an `ArtifactEnvelopeError` to an `AdmissionError`.
pub fn map_artifact_envelope_error(source: ArtifactEnvelopeError) -> AdmissionError {
    match source {
        ArtifactEnvelopeError::ArtifactNotFound { digest } => {
            AdmissionError::ArtifactNotFound { digest }
        }
        ArtifactEnvelopeError::PostcardDecodeFailed => AdmissionError::ArtifactEnvelopeDecodeFailed,
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
