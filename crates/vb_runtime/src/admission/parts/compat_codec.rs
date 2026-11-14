//! Private 9-position Serde compatibility view for RunAdmission.
//!
//! Historical 9-field unbound records (no input_digest) must decode and
//! re-encode byte-for-byte. New bound admissions carry an input digest
//! in a closed carrier discriminant 2 at the budget position. The
//! carrier's three variants map to Postcard bytes that exactly preserve
//! old Option::None (0x00) and Option::Some (0x01) records.
//!
//! Carrier discriminants:
//!   0 -> LegacyUnbound, budget=None   (old 0x00)
//!   1 -> LegacyUnbound, budget=Some   (old 0x01 + budget)
//!   2 -> Digest, budget=option        (new bound)
//!
//! Position 6 in the old layout was Option<AggregateResourceBudget>.
//! This compatibility view replaces that Option with the carrier, which
//! encodes legacy Option variants identically while adding discriminant 2
//! for new bound admissions that retain their input digest.

use vb_core::budget::AggregateResourceBudget;
use vb_core::capability::CapabilitySet;
use vb_core::ids::{ActionId, RunId, TenantId, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_storage::RunInputBinding;
use vb_storage::RunPolicyPin;
use vb_storage::compat::CompatDecision;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::RunAdmission;

/// The compatibility wire layout: 9 positions in historical order.
/// Used for deserialization (owned values).
#[derive(Debug, Deserialize)]
struct CompatibilityView {
    artifact_digest: WorkflowDigest,
    run_id: RunId,
    granted_capabilities: CapabilitySet,
    policy: RuntimePolicy,
    policy_pin: RunPolicyPin,
    carrier: Carrier,
    tenant: Option<TenantId>,
    idempotency_attested: Box<[ActionId]>,
    ir_compat_decision: Option<Box<CompatDecision>>,
}

/// Borrowed serialization view that references RunAdmission fields directly
/// to avoid cloning capabilities, idempotency evidence, or boxed compat
/// context during serialization. The carrier is a Copy value (all fields
/// are Copy), so it is passed by value rather than by reference.
#[derive(Debug, Serialize)]
struct CompatibilityViewBorrowed<'a> {
    artifact_digest: &'a WorkflowDigest,
    run_id: &'a RunId,
    granted_capabilities: &'a CapabilitySet,
    policy: &'a RuntimePolicy,
    policy_pin: &'a RunPolicyPin,
    carrier: Carrier,
    tenant: &'a Option<TenantId>,
    idempotency_attested: &'a [ActionId],
    ir_compat_decision: &'a Option<Box<CompatDecision>>,
}

/// Closed carrier at the budget position with three states:
/// historical unbound (no budget), historical with budget, and new bound
/// with digest. Discriminant 2 was not used in the old 9-field layout.
/// All fields are Copy types, so the entire carrier is Copy and can be
/// passed by value to borrowed serialization views.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum Carrier {
    LegacyUnbound,
    LegacyWithBudget(AggregateResourceBudget),
    Bound {
        digest: WorkflowDigest,
        budget: Option<AggregateResourceBudget>,
    },
}

impl<'a> CompatibilityViewBorrowed<'a> {
    fn from_admission(admission: &'a RunAdmission) -> Self {
        let carrier = match (&admission.input_digest, admission.budget) {
            (RunInputBinding::LegacyUnbound, None) => Carrier::LegacyUnbound,
            (RunInputBinding::LegacyUnbound, Some(budget)) => Carrier::LegacyWithBudget(budget),
            (RunInputBinding::Digest(digest), budget) => Carrier::Bound {
                digest: *digest,
                budget,
            },
        };
        Self {
            artifact_digest: &admission.artifact_digest,
            run_id: &admission.run_id,
            granted_capabilities: &admission.granted_capabilities,
            policy: &admission.policy,
            policy_pin: &admission.policy_pin,
            carrier,
            tenant: &admission.tenant,
            idempotency_attested: admission.idempotency_attested.as_ref(),
            ir_compat_decision: &admission.ir_compat_decision,
        }
    }
}

impl CompatibilityView {
    fn into_admission(self) -> RunAdmission {
        let (input_digest, budget) = match self.carrier {
            Carrier::LegacyUnbound => (RunInputBinding::LegacyUnbound, None),
            Carrier::LegacyWithBudget(b) => (RunInputBinding::LegacyUnbound, Some(b)),
            Carrier::Bound { digest, budget } => (RunInputBinding::Digest(digest), budget),
        };
        RunAdmission {
            artifact_digest: self.artifact_digest,
            run_id: self.run_id,
            granted_capabilities: self.granted_capabilities,
            policy: self.policy,
            policy_pin: self.policy_pin,
            input_digest,
            budget,
            tenant: self.tenant,
            idempotency_attested: self.idempotency_attested,
            ir_compat_decision: self.ir_compat_decision,
        }
    }
}

pub(super) fn serialize_admission<S>(
    admission: &RunAdmission,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let view = CompatibilityViewBorrowed::from_admission(admission);
    view.serialize(serializer)
}

pub(super) fn deserialize_admission<'de, D>(deserializer: D) -> Result<RunAdmission, D::Error>
where
    D: Deserializer<'de>,
{
    let view = CompatibilityView::deserialize(deserializer)?;
    Ok(view.into_admission())
}
