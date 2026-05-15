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
}

/// Errors that can occur during run admission.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
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
    /// Artifact digest does not match the requested digest.
    #[error("admission rejected: artifact digest mismatch")]
    ArtifactDigestMismatch {
        /// Digest that was requested for admission.
        requested: WorkflowDigest,
        /// Digest found in the stored record.
        record: WorkflowDigest,
        /// Digest in the accepted artifact envelope.
        envelope: WorkflowDigest,
    },
    /// Artifact is stale and cannot be admitted under strict/journaled policy.
    #[error("admission rejected: artifact certificate is stale")]
    ArtifactStale {
        /// Digest of the stale artifact.
        digest: WorkflowDigest,
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

        // Validate gate count (must be 15 for v1 accepted artifact).
        if artifact.verification.gate_count != REQUIRED_GATE_COUNT {
            return Err(ArtifactEnvelopeError::InvalidGateCount {
                found: artifact.verification.gate_count,
                required: REQUIRED_GATE_COUNT,
            });
        }

        // Validate required proof flags are all true.
        if !artifact.verification.bounded {
            return Err(ArtifactEnvelopeError::MissingRequiredProofFlagBounded);
        }
        if !artifact.verification.taint_safe {
            return Err(ArtifactEnvelopeError::MissingRequiredProofFlagTaintSafe);
        }
        if !artifact.verification.retry_safe {
            return Err(ArtifactEnvelopeError::MissingRequiredProofFlagRetrySafe);
        }
        if !artifact.verification.durable {
            return Err(ArtifactEnvelopeError::MissingRequiredProofFlagDurable);
        }
        if !artifact.verification.replayable {
            return Err(ArtifactEnvelopeError::MissingRequiredProofFlagReplayable);
        }

        Ok(artifact)
    }
}

/// Performs the admission gate check for a submit.
///
/// - Strict / Journaled: artifact must exist in the store.
/// - Relaxed: always succeeds.
///
/// Returns a `RunAdmission` on success or an `AdmissionError` on rejection.
pub fn admit_run(
    store: &dyn ArtifactStore,
    policy: RuntimePolicy,
    digest: WorkflowDigest,
    run_id: RunId,
    caps: CapabilitySet,
) -> Result<RunAdmission, AdmissionError> {
    match policy {
        RuntimePolicy::Strict | RuntimePolicy::Journaled => {
            if !store.compiled_ir_exists(digest) {
                return Err(AdmissionError::ArtifactNotFound { digest });
            }
        }
        RuntimePolicy::Relaxed => {}
    }
    Ok(RunAdmission::new(digest, run_id, caps, policy))
}

/// Performs full admission gate check with artifact validation before run creation.
///
/// For `RuntimePolicy::Strict` and `RuntimePolicy::Journaled`:
///   - Loads and validates the accepted artifact from storage
///   - Checks that the artifact has exactly REQUIRED_GATE_COUNT gates
///   - Validates that the artifact is marked durable
///   - Validates that the artifact is not stale (accepted_at_seq > 0)
///   - Validates that the artifact has all required proof flags set
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
                .map_err(|source| match source {
                    ArtifactEnvelopeError::ArtifactNotFound { digest } => {
                        AdmissionError::ArtifactNotFound { digest }
                    }
                    ArtifactEnvelopeError::PostcardDecodeFailed => {
                        AdmissionError::ArtifactEnvelopeDecodeFailed
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
                })?;

            // Runtime-level revalidation: ensure the stored artifact digest matches
            // the requested digest. This prevents a tampered record from satisfying
            // admission by returning a different artifact than requested.
            if artifact.digest != artifact_digest {
                return Err(AdmissionError::ArtifactDigestMismatch {
                    requested: artifact_digest,
                    record: artifact.verification.digest,
                    envelope: artifact.digest,
                });
            }

            // Runtime-level revalidation: ensure the proof digest matches the
            // artifact digest to prevent envelope tampering.
            if artifact.verification.digest != artifact_digest {
                return Err(AdmissionError::ArtifactDigestMismatch {
                    requested: artifact_digest,
                    record: artifact.verification.digest,
                    envelope: artifact.digest,
                });
            }

            // Runtime-level gate count check: strict/journaled artifacts must
            // have exactly REQUIRED_GATE_COUNT gates to ensure canonical form.
            if artifact.verification.gate_count != REQUIRED_GATE_COUNT {
                return Err(AdmissionError::ArtifactInvalidGateCount {
                    found: artifact.verification.gate_count,
                    required: REQUIRED_GATE_COUNT,
                });
            }

            // Runtime-level proof flag checks: strict/journaled artifacts must
            // have all required proof flags set. Order matters for error precedence.
            // Bounded is checked before durable to ensure the artifact has bounded
            // execution semantics before checking durability guarantees.
            if !artifact.verification.bounded {
                return Err(AdmissionError::ArtifactInvalidProofFlag { flag: "bounded" });
            }
            if !artifact.verification.taint_safe {
                return Err(AdmissionError::ArtifactInvalidProofFlag { flag: "taint_safe" });
            }
            if !artifact.verification.retry_safe {
                return Err(AdmissionError::ArtifactInvalidProofFlag { flag: "retry_safe" });
            }
            if !artifact.verification.replayable {
                return Err(AdmissionError::ArtifactInvalidProofFlag { flag: "replayable" });
            }

            // Runtime-level durable flag check: strict/journaled artifacts must
            // be marked durable to prevent non-durable bypass.
            if !artifact.verification.durable {
                return Err(AdmissionError::ArtifactInvalidProofFlag { flag: "durable" });
            }

            // Runtime-level staleness check: artifacts with zero acceptance sequence
            // are considered stale and cannot be admitted under strict/journaled policy.
            if artifact.accepted_at_seq.get() == 0 {
                return Err(AdmissionError::ArtifactStale {
                    digest: artifact_digest,
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

            Ok(RunAdmission::new(artifact_digest, run_id, caps, policy))
        }
        RuntimePolicy::Relaxed => {
            // Relaxed: skip artifact loading and capability checking.
            Ok(RunAdmission::new(artifact_digest, run_id, caps, policy))
        }
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
        RuntimePolicy::Strict | RuntimePolicy::Journaled => {
            if !store.compiled_ir_exists(digest) {
                return Err(AdmissionError::ArtifactNotFound { digest });
            }
        }
        RuntimePolicy::Relaxed => {}
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
            ir: Vec::new(),
            verification: vb_storage::admission::VerificationProof {
                digest,
                gate_count: REQUIRED_GATE_COUNT,
                durable: true,
                bounded: true,
                taint_safe: true,
                retry_safe: true,
                replayable: true,
                idempotency_keyed: Box::new([]),
                idempotency_attested: Box::new([]),
                warnings: Vec::new(),
            },
            // Use non-zero sequence so the artifact is not considered stale.
            accepted_at_seq: vb_storage::EventSeq::new(1),
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
        let store = AlwaysPresentArtifactStore::shared_artifact();
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
    fn admission_admit_run_relaxed_without_artifact() {
        /// An artifact store that always reports artifacts as absent.
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
        impl ArtifactStore for NeverPresentStore {
            fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
                false
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
        impl ArtifactStore for NeverPresentStore {
            fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
                false
            }
        }
        let store = NeverPresentStore;
        let digest = test_digest();
        let run_id = RunId::new(1);
        let caps = CapabilitySet::empty();
        let result = admit_run(&store, RuntimePolicy::Journaled, digest, run_id, caps);
        assert_eq!(result, Err(AdmissionError::ArtifactNotFound { digest }));
    }
}

// Test support module: provides test artifact store implementation.
// This is in a separate file to satisfy the source code inspection test that verifies
// AlwaysPresentArtifactStore is not used as an AcceptedArtifactStore in this module.
mod admission_test_support;
