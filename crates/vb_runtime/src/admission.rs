#![forbid(unsafe_code)]
//! Runtime admission control for workflow runs.
//!
//! `RunAdmission` records the artifact digest, granted capabilities,
//! and admission policy for each accepted run. `AdmissionError` enumerates
//! the reasons a submit may be rejected at the admission gate.

use std::sync::Arc;
use thiserror::Error;
use vb_core::budget::{
    AggregateBudgetError, AggregateResourceBudget, AggregateResourceCapacity,
    AggregateResourceUsage, BoundednessPolicy, validate_aggregate_budget,
};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, RunId, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_storage::EventSeq;

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

/// Aggregate resource request plus policy used by runtime budget admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmissionBudgetRequest {
    /// Aggregate resources requested by the run.
    pub requested: AggregateResourceBudget,
    /// Shard-local aggregate resource capacity available for admission.
    pub available: AggregateResourceCapacity,
    /// Policy ceiling that the requested budget must satisfy before capacity is reserved.
    pub policy: BoundednessPolicy,
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

            if artifact.accepted_at_seq < required_at_least {
                return Err(AdmissionError::ArtifactCertificateStale {
                    digest: artifact_digest,
                    accepted_at_seq: artifact.accepted_at_seq,
                    required_at_least,
                });
            }

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
    admit_run_with_budget_policy(
        store,
        policy,
        digest,
        run_id,
        caps,
        AdmissionBudgetRequest {
            requested,
            available,
            policy: BoundednessPolicy::DEFAULT,
        },
    )
}

/// Performs artifact admission plus policy and aggregate capacity admission.
pub fn admit_run_with_budget_policy(
    store: &dyn ArtifactStore,
    policy: RuntimePolicy,
    digest: WorkflowDigest,
    run_id: RunId,
    caps: CapabilitySet,
    budget: AdmissionBudgetRequest,
) -> Result<RunAdmission, AdmissionError> {
    validate_aggregate_budget(&budget.requested, &budget.policy).map_err(map_budget_error)?;
    let requested_usage = AggregateResourceUsage::default()
        .try_add_budget(&budget.requested)
        .map_err(map_budget_error)?;
    requested_usage
        .check_policy(&budget.policy)
        .map_err(map_budget_error)?;
    requested_usage
        .fits_within(&budget.available)
        .map_err(map_budget_error)?;
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
        digest,
        run_id,
        caps,
        policy,
        budget.requested,
    ))
}

fn map_budget_error(error: AggregateBudgetError) -> AdmissionError {
    // NOTE: AggregateBudgetError is #[non_exhaustive]. This catch-all ensures
    // new error variants don't break existing code, but they lose specific semantics.
    // Consider adding explicit arms for new variants as they are added.
    match error {
        AggregateBudgetError::PolicyExceeded {
            resource,
            actual,
            limit,
        } => AdmissionError::BudgetPolicyExceeded {
            resource,
            actual,
            limit,
        },
        AggregateBudgetError::CapacityExceeded {
            resource,
            requested,
            available,
        } => AdmissionError::ResourceCapacityExceeded {
            resource,
            requested,
            available,
        },
        AggregateBudgetError::Overflow { resource } => {
            AdmissionError::ResourceBudgetOverflow { resource }
        }
        AggregateBudgetError::Underflow { resource } => {
            AdmissionError::ResourceBudgetUnderflow { resource }
        }
        AggregateBudgetError::InvalidCapacity { resource } => {
            AdmissionError::ResourceBudgetInvalidCapacity { resource }
        }
        AggregateBudgetError::StepCeilingExceeded { requested, limit } => {
            AdmissionError::ResourceStepCeilingExceeded { requested, limit }
        }
        AggregateBudgetError::PerTickCeilingExceeded { requested, limit } => {
            AdmissionError::ResourcePerTickCeilingExceeded { requested, limit }
        }
        #[cfg(not(kani))]
        AggregateBudgetError::WorkflowBudget(_) => AdmissionError::BudgetPolicyExceeded {
            resource: "workflow_budget",
            actual: u64::MAX,
            limit: 0,
        },
        #[cfg(kani)]
        AggregateBudgetError::WorkflowBudget => AdmissionError::BudgetPolicyExceeded {
            resource: "workflow_budget",
            actual: u64::MAX,
            limit: 0,
        },
        AggregateBudgetError::ReservationNotFound { .. } => AdmissionError::BudgetPolicyExceeded {
            resource: "reservation_not_found",
            actual: u64::MAX,
            limit: 0,
        },
        _ => AdmissionError::BudgetPolicyExceeded {
            resource: "unknown_aggregate_budget_error", // DEAD: #[non_exhaustive] catch-all
            actual: u64::MAX,
            limit: 0,
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
#[path = "admission/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "admission/artifact_envelope_tests.rs"]
mod artifact_envelope_tests;
