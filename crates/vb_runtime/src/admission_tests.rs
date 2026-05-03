//! Tests for runtime admission control.
#![forbid(unsafe_code)]

use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, RunId, WorkflowDigest};
use vb_core::policy::RuntimePolicy;

use crate::admission::{
    admit_run, check_capability, AlwaysPresentArtifactStore, ArtifactStore, RunAdmission,
    AdmissionError,
};

fn test_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0xAB; 32])
}

#[test]
fn admission_new_stores_all_fields() {
    let digest = test_digest();
    let run_id = RunId::new(42);
    let caps = CapabilitySet::from_grants(Box::new([Capability::AnyWorkflow]));
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
    let required = Capability::Action(ActionId::new(5));
    let granted = CapabilitySet::empty();
    let err = AdmissionError::CapabilityDenied {
        action,
        required,
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
    let required = Capability::Action(ActionId::new(1));
    let granted = CapabilitySet::from_grants(Box::new([Capability::Action(ActionId::new(1))]));
    assert_eq!(check_capability(action, &required, &granted), Ok(()));
}

#[test]
fn admission_check_capability_denied() {
    let action = ActionId::new(1);
    let required = Capability::Action(ActionId::new(1));
    let granted = CapabilitySet::empty();
    let result = check_capability(action, &required, &granted);
    assert!(matches!(result, Err(AdmissionError::CapabilityDenied { .. })));
}

#[test]
fn admission_check_capability_any_workflow_grants_action() {
    let action = ActionId::new(99);
    let required = Capability::Action(ActionId::new(99));
    let granted = CapabilitySet::from_grants(Box::new([Capability::AnyWorkflow]));
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
    let caps = CapabilitySet::from_grants(Box::new([Capability::AnyWorkflow]));
    let original = RunAdmission::new(digest, run_id, caps, RuntimePolicy::Strict);
    let cloned = original.clone();
    assert_eq!(cloned, original);
}

#[test]
fn admission_admit_run_strict_with_present_artifact() {
    let store = AlwaysPresentArtifactStore::shared();
    let digest = test_digest();
    let run_id = RunId::new(1);
    let caps = CapabilitySet::from_grants(Box::new([Capability::AnyWorkflow]));
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
    assert_eq!(
        result,
        Err(AdmissionError::ArtifactNotFound { digest })
    );
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
    assert_eq!(
        result,
        Err(AdmissionError::ArtifactNotFound { digest })
    );
}
