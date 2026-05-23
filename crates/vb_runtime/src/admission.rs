#![forbid(unsafe_code)]
//! Runtime admission control for workflow runs.
//!
//! `RunAdmission` records the artifact digest, granted capabilities,
//! and admission policy for each accepted run. `AdmissionError` enumerates
//! the reasons a submit may be rejected at the admission gate.

use std::sync::Arc;
use thiserror::Error;
use vb_core::budget::{AggregateResourceBudget, AggregateResourceCapacity, AggregateResourceUsage};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, RunId, WorkflowDigest};
use vb_core::policy::RuntimePolicy;

/// Number of verification gates required in a v1 accepted artifact for Strict/Journaled admission.
pub const REQUIRED_GATE_COUNT: u8 = 15;

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

/// Accepted run admission record, attached to a run frame after passing the admission gate.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RunAdmission {
    /// Digest of the accepted compiled artifact.
    artifact_digest: WorkflowDigest,
    /// Run identifier assigned at admission.
    run_id: RunId,
    /// Capabilities granted for this run.
    granted_capabilities: CapabilitySet,
    /// Admission policy that governed this admission decision.
    policy: RuntimePolicy,
    /// Aggregate budget admitted for this run, when budget admission is used.
    budget: Option<AggregateResourceBudget>,
    /// Actions whose idempotency evidence passed artifact admission.
    idempotency_attested: Box<[ActionId]>,
}

impl RunAdmission {
    /// Creates a new admission record.
    pub fn new(
        digest: WorkflowDigest,
        run_id: RunId,
        caps: CapabilitySet,
        policy: RuntimePolicy,
    ) -> Self {
        Self {
            artifact_digest: digest,
            run_id,
            granted_capabilities: caps,
            policy,
            budget: None,
            idempotency_attested: Box::new([]),
        }
    }

    /// Creates a new admission record carrying accepted idempotency evidence.
    pub fn with_idempotency_evidence(
        digest: WorkflowDigest,
        run_id: RunId,
        caps: CapabilitySet,
        policy: RuntimePolicy,
        idempotency_attested: Box<[ActionId]>,
    ) -> Self {
        Self {
            artifact_digest: digest,
            run_id,
            granted_capabilities: caps,
            policy,
            budget: None,
            idempotency_attested,
        }
    }

    /// Creates a new admission record carrying an aggregate resource budget.
    pub fn with_budget(
        digest: WorkflowDigest,
        run_id: RunId,
        caps: CapabilitySet,
        policy: RuntimePolicy,
        budget: AggregateResourceBudget,
    ) -> Self {
        Self {
            artifact_digest: digest,
            run_id,
            granted_capabilities: caps,
            policy,
            budget: Some(budget),
            idempotency_attested: Box::new([]),
        }
    }

    /// Returns the artifact digest for this admission.
    #[must_use]
    pub fn artifact_digest(&self) -> WorkflowDigest {
        self.artifact_digest
    }

    /// Returns the run identifier for this admission.
    #[must_use]
    pub fn run_id(&self) -> RunId {
        self.run_id
    }

    /// Returns a reference to the granted capabilities.
    #[must_use]
    pub fn granted_capabilities(&self) -> &CapabilitySet {
        &self.granted_capabilities
    }

    /// Returns the admission policy used.
    #[must_use]
    pub fn policy(&self) -> RuntimePolicy {
        self.policy
    }

    /// Returns the admitted aggregate budget when budget admission was used.
    #[must_use]
    pub const fn budget(&self) -> Option<AggregateResourceBudget> {
        self.budget
    }

    /// Returns the idempotency-attested action IDs available to dispatch.
    #[must_use]
    pub fn idempotency_attested(&self) -> &[ActionId] {
        &self.idempotency_attested
    }
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
}

/// Trait for checking whether a compiled artifact exists in storage.
///
/// Implemented by storage backends that can verify artifact presence.
/// The shard uses this to enforce admission policy.
pub trait ArtifactStore: Send + Sync {
    /// Returns `true` if a compiled artifact with the given digest exists.
    fn compiled_ir_exists(&self, digest: WorkflowDigest) -> bool;
}

/// Shared artifact store trait object.
pub type SharedArtifactStore = Arc<dyn ArtifactStore>;

/// Shared accepted artifact store for full validation at admission gate.
pub type SharedAcceptedArtifactStore = Arc<dyn AcceptedArtifactStore>;

/// Artifact store that always reports artifacts as present.
/// Used in tests and when policy is Relaxed.
#[derive(Debug, Default)]
pub struct AlwaysPresentArtifactStore;

/// Artifact store for non-durable strict admission where no accepted artifact
/// source exists.
#[derive(Debug, Default)]
pub struct MissingAcceptedArtifactStore;

impl AlwaysPresentArtifactStore {
    /// Creates a new shared always-present store (legacy artifact-only view).
    #[must_use]
    pub fn shared_artifact() -> SharedArtifactStore {
        Arc::new(Self)
    }

    /// Creates a new shared always-present store as an accepted artifact store.
    #[must_use]
    pub fn shared() -> SharedAcceptedArtifactStore {
        Arc::new(Self)
    }
}

impl ArtifactStore for AlwaysPresentArtifactStore {
    fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
        true
    }
}

/// Loads and validates accepted artifacts from storage.
///
/// This trait enables the runtime admission gate to perform full artifact
/// validation — not just existence — before admitting a run.
pub trait AcceptedArtifactStore: Send + Sync {
    /// Loads and validates an accepted artifact by digest.
    ///
    /// Returns the validated artifact on success, or an error if the artifact
    /// is missing or fails semantic validation (gate count, proof flags).
    fn load_accepted_artifact(
        &self,
        artifact_digest: WorkflowDigest,
    ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError>;
}

impl AcceptedArtifactStore for AlwaysPresentArtifactStore {
    fn load_accepted_artifact(
        &self,
        artifact_digest: WorkflowDigest,
    ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
        Ok(always_present_accepted_artifact(artifact_digest))
    }
}

fn always_present_accepted_artifact(
    artifact_digest: WorkflowDigest,
) -> vb_storage::admission::AcceptedArtifact {
    vb_storage::admission::AcceptedArtifact {
        digest: artifact_digest,
        source_digest: artifact_digest,
        policy_digest: artifact_digest,
        ir: Vec::new(),
        verification: always_present_verification_proof(artifact_digest),
        accepted_at_seq: vb_storage::types::EventSeq::new(0),
        required_capabilities: Box::new([]),
    }
}

fn always_present_verification_proof(
    artifact_digest: WorkflowDigest,
) -> vb_storage::admission::VerificationProof {
    vb_storage::admission::VerificationProof {
        digest: artifact_digest,
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
    }
}

impl MissingAcceptedArtifactStore {
    /// Creates a new shared missing-artifact store.
    #[must_use]
    pub fn shared() -> SharedAcceptedArtifactStore {
        Arc::new(Self)
    }
}

impl AcceptedArtifactStore for MissingAcceptedArtifactStore {
    fn load_accepted_artifact(
        &self,
        artifact_digest: WorkflowDigest,
    ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
        Err(ArtifactEnvelopeError::ArtifactNotFound {
            digest: artifact_digest,
        })
    }
}

/// Artifact store backed by FjallJournal.
pub struct StorageArtifactStore {
    journal: Arc<vb_storage::FjallJournal>,
}

impl StorageArtifactStore {
    /// Creates a new storage-backed artifact store.
    #[must_use]
    pub fn new(journal: Arc<vb_storage::FjallJournal>) -> Self {
        Self { journal }
    }

    /// Creates a new shared storage-backed artifact store (legacy artifact-only view).
    #[must_use]
    pub fn shared_artifact(journal: Arc<vb_storage::FjallJournal>) -> SharedArtifactStore {
        Arc::new(Self::new(journal))
    }

    /// Creates a new shared storage-backed accepted artifact store.
    #[must_use]
    pub fn shared(journal: Arc<vb_storage::FjallJournal>) -> SharedAcceptedArtifactStore {
        Arc::new(Self::new(journal))
    }
}

impl ArtifactStore for StorageArtifactStore {
    fn compiled_ir_exists(&self, digest: WorkflowDigest) -> bool {
        matches!(self.journal.compiled_ir(digest), Ok(Some(_)))
    }
}

impl AcceptedArtifactStore for StorageArtifactStore {
    fn load_accepted_artifact(
        &self,
        artifact_digest: WorkflowDigest,
    ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
        // Load the compiled IR record from the journal.
        let record = self
            .journal
            .compiled_ir(artifact_digest)
            .map_err(|_jb_err| ArtifactEnvelopeError::ArtifactNotFound {
                digest: artifact_digest,
            })?
            .ok_or(ArtifactEnvelopeError::ArtifactNotFound {
                digest: artifact_digest,
            })?;

        // Decode the postcard payload as AcceptedArtifact.
        let artifact: vb_storage::admission::AcceptedArtifact = postcard::from_bytes(&record.ir)
            .map_err(|_decode_err| ArtifactEnvelopeError::PostcardDecodeFailed)?;

        if artifact.digest != artifact_digest {
            return Err(ArtifactEnvelopeError::ArtifactDigestMismatch {
                requested: artifact_digest,
                found: artifact.digest,
            });
        }
        validate_accepted_artifact_envelope(&artifact)?;

        Ok(artifact)
    }
}

fn validate_accepted_artifact_envelope(
    artifact: &vb_storage::admission::AcceptedArtifact,
) -> Result<(), ArtifactEnvelopeError> {
    if artifact.verification.digest != artifact.digest {
        return Err(ArtifactEnvelopeError::ArtifactDigestMismatch {
            requested: artifact.digest,
            found: artifact.verification.digest,
        });
    }
    if artifact.verification.gate_count != REQUIRED_GATE_COUNT {
        return Err(ArtifactEnvelopeError::InvalidGateCount {
            found: artifact.verification.gate_count,
            required: REQUIRED_GATE_COUNT,
        });
    }
    if !artifact.verification.bounded_claimed {
        return Err(ArtifactEnvelopeError::MissingRequiredProofFlagBounded);
    }
    if !artifact.verification.taint_safe_claimed {
        return Err(ArtifactEnvelopeError::MissingRequiredProofFlagTaintSafe);
    }
    if !artifact.verification.retry_safe_claimed {
        return Err(ArtifactEnvelopeError::MissingRequiredProofFlagRetrySafe);
    }
    if !artifact.verification.durable {
        return Err(ArtifactEnvelopeError::MissingRequiredProofFlagDurable);
    }
    if !artifact.verification.replayable_claimed {
        return Err(ArtifactEnvelopeError::MissingRequiredProofFlagReplayable);
    }
    if !artifact.verification.idempotency_verified_claimed {
        return Err(ArtifactEnvelopeError::MissingRequiredProofFlagIdempotencyVerified);
    }
    first_missing_idempotency_attestation(artifact).map_or(Ok(()), |action| {
        Err(ArtifactEnvelopeError::MissingIdempotencyAttestation { action })
    })
}

fn first_missing_idempotency_attestation(
    artifact: &vb_storage::admission::AcceptedArtifact,
) -> Option<ActionId> {
    artifact
        .verification
        .idempotency_keyed
        .iter()
        .copied()
        .find(|action| !artifact.verification.idempotency_attested.contains(action))
}

fn map_artifact_envelope_error(source: ArtifactEnvelopeError) -> AdmissionError {
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
    match policy {
        RuntimePolicy::Strict | RuntimePolicy::Journaled => {
            let artifact = store
                .load_accepted_artifact(digest)
                .map_err(map_artifact_envelope_error)?;
            validate_accepted_artifact_envelope(&artifact).map_err(map_artifact_envelope_error)?;
            if artifact.digest != digest {
                return Err(AdmissionError::ArtifactDigestMismatch {
                    requested: digest,
                    found: artifact.digest,
                });
            }
        }
        RuntimePolicy::Relaxed => {}
        _ => {
            return Err(AdmissionError::ArtifactInvalidProofFlag {
                flag: "runtime_policy",
            });
        }
    }
    Ok(RunAdmission::new(digest, run_id, caps, policy))
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
    match policy {
        RuntimePolicy::Strict | RuntimePolicy::Journaled => {
            // Load and validate the full artifact.
            let artifact = store
                .load_accepted_artifact(artifact_digest)
                .map_err(map_artifact_envelope_error)?;
            validate_accepted_artifact_envelope(&artifact).map_err(map_artifact_envelope_error)?;

            // Check that granted capabilities cover the artifact's required capabilities.
            if caps.len() != artifact.required_capabilities.len() {
                return Err(capability_count_mismatch_error(
                    &artifact.required_capabilities,
                    &caps,
                ));
            }
            for required_cap in artifact.required_capabilities.iter() {
                check_capability(required_cap.action_id(), required_cap, &caps)?;
            }

            // INV-002: digest binding must be total. The loaded artifact's digest
            // must match the requested digest exactly — a crafted artifact with
            // valid gates but wrong identity must not be admitted.
            if artifact.digest != artifact_digest {
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

            Ok(RunAdmission::with_idempotency_evidence(
                artifact_digest,
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

/// Performs artifact admission plus aggregate resource capacity admission.
pub fn admit_run_with_budget(
    store: &dyn ArtifactStore,
    policy: RuntimePolicy,
    digest: WorkflowDigest,
    run_id: RunId,
    caps: CapabilitySet,
    requested: AggregateResourceBudget,
    available: AggregateResourceCapacity,
) -> Result<RunAdmission, AdmissionError> {
    let requested_usage = AggregateResourceUsage::default()
        .try_add_budget(&requested)
        .map_err(|error| map_budget_error(error, requested, available))?;
    requested_usage
        .fits_within(&available)
        .map_err(|error| map_budget_error(error, requested, available))?;
    match policy {
        RuntimePolicy::Strict | RuntimePolicy::Journaled if !store.compiled_ir_exists(digest) => {
            return Err(AdmissionError::ArtifactNotFound { digest });
        }
        RuntimePolicy::Strict | RuntimePolicy::Journaled => {}
        RuntimePolicy::Relaxed => {}
        _ => {
            return Err(AdmissionError::ArtifactInvalidProofFlag {
                flag: "runtime_policy",
            });
        }
    }
    Ok(RunAdmission::with_budget(
        digest, run_id, caps, policy, requested,
    ))
}

fn map_budget_error(
    error: vb_core::budget::AggregateBudgetError,
    _requested_budget: AggregateResourceBudget,
    _available_capacity: AggregateResourceCapacity,
) -> AdmissionError {
    match error {
        vb_core::budget::AggregateBudgetError::CapacityExceeded {
            resource,
            requested,
            available,
        } => AdmissionError::ResourceCapacityExceeded {
            resource,
            requested,
            available,
        },
        vb_core::budget::AggregateBudgetError::Overflow { resource } => {
            AdmissionError::ResourceCapacityExceeded {
                resource,
                requested: u64::MAX,
                available: u64::MAX,
            }
        }
        _ => AdmissionError::ResourceCapacityExceeded {
            resource: "aggregate_resource_budget",
            requested: u64::MAX,
            available: 0,
        },
    }
}

/// Checks whether a capability is granted for an action.
///
/// Returns `Ok(())` if the action's capability is covered by the granted set,
/// or `Err(AdmissionError::CapabilityDenied)` otherwise.
pub fn check_capability(
    action: ActionId,
    required: &Capability,
    granted: &CapabilitySet,
) -> Result<(), AdmissionError> {
    if granted.grants(required) {
        Ok(())
    } else {
        Err(AdmissionError::CapabilityDenied {
            action,
            required: required.clone(),
            granted: granted.clone(),
        })
    }
}

fn capability_count_mismatch_error(
    required: &[Capability],
    granted: &CapabilitySet,
) -> AdmissionError {
    let fallback = Capability::new("__capability_count_mismatch__".into(), ActionId::new(0));
    let required_capability = required.first().cloned().unwrap_or(fallback);
    AdmissionError::CapabilityDenied {
        action: required_capability.action_id(),
        required: required_capability,
        granted: granted.clone(),
    }
}

#[cfg(test)]
mod tests {
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
        let caps =
            CapabilitySet::from_grants(Box::new([Capability::new("".into(), ActionId::new(0))]));
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
        let granted =
            CapabilitySet::from_grants(Box::new([Capability::new("network".into(), action)]));
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
    fn admit_artifact_run_rejects_excess_grants() {
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
                action: network.action_id(),
                required: network,
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
        let granted =
            CapabilitySet::from_grants(Box::new([Capability::new("network".into(), action)]));

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
        let caps =
            CapabilitySet::from_grants(Box::new([Capability::new("".into(), ActionId::new(0))]));
        let original = RunAdmission::new(digest, run_id, caps, RuntimePolicy::Strict);
        let cloned = original.clone();
        assert_eq!(cloned, original);
    }

    #[test]
    fn admission_admit_run_strict_with_present_artifact() {
        let store = AlwaysPresentArtifactStore::shared();
        let digest = test_digest();
        let run_id = RunId::new(1);
        let caps =
            CapabilitySet::from_grants(Box::new([Capability::new("".into(), ActionId::new(0))]));
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
            ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError>
            {
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
            ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError>
            {
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
            ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError>
            {
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
            ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError>
            {
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
            ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError>
            {
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
            ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError>
            {
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
            ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError>
            {
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
            ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError>
            {
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
            ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError>
            {
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
            ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError>
            {
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
            ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError>
            {
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
            ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError>
            {
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
            ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError>
            {
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
            ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError>
            {
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
            ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError>
            {
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
        // Request more steps than the capacity allows.
        let requested = AggregateResourceBudget {
            max_steps_executable: u32::MAX,
            max_action_tickets: u32::MAX,
            max_parallel_in_flight: u16::MAX,
            max_retries_per_action: u16::MAX,
            max_gather_pages: u32::MAX,
            max_gather_items: u32::MAX,
            max_for_each_iterations: u32::MAX,
            max_together_branches: u16::MAX,
            max_repeat_attempts: u16::MAX,
            max_run_time_seconds: u64::MAX,
            max_result_bytes: u32::MAX,
            max_total_slots_written: u32::MAX,
            max_queue_depth: u32::MAX,
            max_journal_batch_bytes: u32::MAX,
            max_step_budget_per_tick: u64::MAX,
            max_transitions_per_tick: u64::MAX,
        };
        let capacity = AggregateResourceCapacity {
            max_steps_executable: 0,
            max_action_tickets: u64::MAX,
            max_parallel_in_flight: u32::MAX,
            max_gather_pages: u64::MAX,
            max_gather_items: u64::MAX,
            max_result_bytes: u64::MAX,
            max_total_slots_written: u64::MAX,
            max_active_runs: u64::MAX,
            max_queue_depth: u64::MAX,
            max_journal_batch_bytes: u64::MAX,
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
            max_queue_depth: 0,
            max_journal_batch_bytes: 0,
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
            max_active_runs: u64::MAX,
            max_queue_depth: u64::MAX,
            max_journal_batch_bytes: u64::MAX,
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
}

#[cfg(test)]
mod artifact_envelope_tests {
    // Tests are in artifact_envelope_tests.rs
    // but we include them here via the module system.
    include!("admission/artifact_envelope_tests.rs");
}
