//! Private human-readable Serde view for RunAdmission.
//!
//! For bound admissions (input_digest is a Digest) serialize all ten fields.
//! For unbound admissions omit input_digest to preserve the old nine-field
//! named-map shape. Deserialization defaults a missing input_digest to
//! LegacyUnbound, missing tenant to None, and missing ir_compat_decision
//! to None; idempotency_attested is required.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use vb_core::budget::AggregateResourceBudget;
use vb_core::capability::CapabilitySet;
use vb_core::ids::{ActionId, RunId, TenantId, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_storage::RunInputBinding;
use vb_storage::RunPolicyPin;
use vb_storage::compat::CompatDecision;

use super::RunAdmission;

/// Borrowed serialization view for bound admissions with input digest.
/// All ten fields are present; references avoid cloning.
#[derive(Serialize)]
struct HumanBound<'a> {
    artifact_digest: &'a WorkflowDigest,
    run_id: &'a RunId,
    granted_capabilities: &'a CapabilitySet,
    policy: &'a RuntimePolicy,
    policy_pin: &'a RunPolicyPin,
    input_digest: &'a RunInputBinding,
    budget: &'a Option<AggregateResourceBudget>,
    tenant: &'a Option<TenantId>,
    idempotency_attested: &'a [ActionId],
    ir_compat_decision: &'a Option<Box<CompatDecision>>,
}

impl<'a> HumanBound<'a> {
    fn from_admission(admission: &'a RunAdmission) -> Self {
        Self {
            artifact_digest: &admission.artifact_digest,
            run_id: &admission.run_id,
            granted_capabilities: &admission.granted_capabilities,
            policy: &admission.policy,
            policy_pin: &admission.policy_pin,
            input_digest: &admission.input_digest,
            budget: &admission.budget,
            tenant: &admission.tenant,
            idempotency_attested: &admission.idempotency_attested,
            ir_compat_decision: &admission.ir_compat_decision,
        }
    }
}

/// Borrowed serialization view for unbound admissions without input digest.
/// Nine fields preserve the historical named-map shape.
#[derive(Serialize)]
struct HumanUnbound<'a> {
    artifact_digest: &'a WorkflowDigest,
    run_id: &'a RunId,
    granted_capabilities: &'a CapabilitySet,
    policy: &'a RuntimePolicy,
    policy_pin: &'a RunPolicyPin,
    budget: &'a Option<AggregateResourceBudget>,
    tenant: &'a Option<TenantId>,
    idempotency_attested: &'a [ActionId],
    ir_compat_decision: &'a Option<Box<CompatDecision>>,
}

impl<'a> HumanUnbound<'a> {
    fn from_admission(admission: &'a RunAdmission) -> Self {
        Self {
            artifact_digest: &admission.artifact_digest,
            run_id: &admission.run_id,
            granted_capabilities: &admission.granted_capabilities,
            policy: &admission.policy,
            policy_pin: &admission.policy_pin,
            budget: &admission.budget,
            tenant: &admission.tenant,
            idempotency_attested: &admission.idempotency_attested,
            ir_compat_decision: &admission.ir_compat_decision,
        }
    }
}

/// Owned deserialization view accepting both nine-field legacy and
/// ten-field bound input. Missing optional fields use serde defaults.
#[derive(Deserialize)]
struct Human {
    artifact_digest: WorkflowDigest,
    run_id: RunId,
    granted_capabilities: CapabilitySet,
    policy: RuntimePolicy,
    policy_pin: RunPolicyPin,
    #[serde(default)]
    input_digest: RunInputBinding,
    budget: Option<AggregateResourceBudget>,
    #[serde(default)]
    tenant: Option<TenantId>,
    idempotency_attested: Box<[ActionId]>,
    #[serde(default)]
    ir_compat_decision: Option<Box<CompatDecision>>,
}

impl Human {
    fn into_admission(self) -> RunAdmission {
        RunAdmission {
            artifact_digest: self.artifact_digest,
            run_id: self.run_id,
            granted_capabilities: self.granted_capabilities,
            policy: self.policy,
            policy_pin: self.policy_pin,
            input_digest: self.input_digest,
            budget: self.budget,
            tenant: self.tenant,
            idempotency_attested: self.idempotency_attested,
            ir_compat_decision: self.ir_compat_decision,
        }
    }
}

pub(super) fn serialize_human<S>(admission: &RunAdmission, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if matches!(admission.input_digest, RunInputBinding::Digest(_)) {
        HumanBound::from_admission(admission).serialize(serializer)
    } else {
        HumanUnbound::from_admission(admission).serialize(serializer)
    }
}

pub(super) fn deserialize_human<'de, D>(deserializer: D) -> Result<RunAdmission, D::Error>
where
    D: Deserializer<'de>,
{
    let human = Human::deserialize(deserializer)?;
    Ok(human.into_admission())
}
