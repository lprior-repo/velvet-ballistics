//! Runtime admission control for workflow runs.
//!
//! `RunAdmission` records the artifact digest, granted capabilities,
//! and admission policy for each accepted run. `AdmissionError` enumerates
//! the reasons a submit may be rejected at the admission gate.

use std::sync::Arc;
use thiserror::Error;
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, RunId, WorkflowDigest};
use vb_core::policy::RuntimePolicy;

/// Accepted run admission record, attached to a run frame after passing the admission gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunAdmission {
    /// Digest of the accepted compiled artifact.
    artifact_digest: WorkflowDigest,
    /// Run identifier assigned at admission.
    run_id: RunId,
    /// Capabilities granted for this run.
    granted_capabilities: CapabilitySet,
    /// Admission policy that governed this admission decision.
    policy: RuntimePolicy,
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

/// Artifact store that always reports artifacts as present.
/// Used in tests and when policy is Relaxed.
#[derive(Debug, Default)]
pub struct AlwaysPresentArtifactStore;

impl AlwaysPresentArtifactStore {
    /// Creates a new shared always-present store.
    #[must_use]
    pub fn shared() -> SharedArtifactStore {
        Arc::new(Self)
    }
}

impl ArtifactStore for AlwaysPresentArtifactStore {
    fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
        true
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

    /// Creates a new shared storage-backed artifact store.
    #[must_use]
    pub fn shared(journal: Arc<vb_storage::FjallJournal>) -> SharedArtifactStore {
        Arc::new(Self::new(journal))
    }
}

impl ArtifactStore for StorageArtifactStore {
    fn compiled_ir_exists(&self, digest: WorkflowDigest) -> bool {
        match self.journal.compiled_ir(digest) {
            Ok(Some(_)) => true,
            _ => false,
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

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::WorkflowDigest;

    fn test_digest() -> WorkflowDigest {
        WorkflowDigest::from_bytes([0xAB; 32])
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
        let granted = CapabilitySet::from_grants(Box::new([Capability::new("network".into(), ActionId::new(1))]));
        assert_eq!(check_capability(action, &required, &granted), Ok(()));
    }

    #[test]
    fn admission_check_capability_denied() {
        let action = ActionId::new(1);
        let required = Capability::new("network".into(), ActionId::new(1));
        let granted = CapabilitySet::empty();
        let result = check_capability(action, &required, &granted);
        assert!(matches!(
            result,
            Err(AdmissionError::CapabilityDenied { .. })
        ));
    }

    #[test]
    fn admission_check_capability_hierarchical_grants_subname() {
        let action = ActionId::new(99);
        let required = Capability::new("network.http".into(), action);
        let granted = CapabilitySet::from_grants(Box::new([Capability::new("network".into(), action)]));
        assert_eq!(check_capability(action, &required, &granted), Ok(()));
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
        assert!(result.is_ok());
        let admission = match result {
            Ok(a) => a,
            Err(_) => return,
        };
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
        assert!(result.is_ok());
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
