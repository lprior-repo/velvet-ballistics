/// Private 9-position Serde compatibility view for RunAdmission.
///
/// Historical 9-field unbound records (no input_digest) must decode and
/// re-encode byte-for-byte. New bound admissions carry an input digest
/// in a closed carrier discriminant 2 at the budget position. The
/// carrier's three variants map to Postcard bytes that exactly preserve
/// old Option::None (0x00) and Option::Some (0x01) records.
///
/// Carrier discriminants:
///   0 -> LegacyUnbound, budget=None   (old 0x00)
///   1 -> LegacyUnbound, budget=Some   (old 0x01 + budget)
///   2 -> Digest, budget=option        (new bound)
///
/// Position 6 in the old layout was Option<AggregateResourceBudget>.
/// This compatibility view replaces that Option with the carrier, which
/// encodes legacy Option variants identically while adding discriminant 2
/// for new bound admissions that retain their input digest.

use serde::{Deserialize, Serialize, Deserializer, Serializer};

/// The compatibility wire layout: 9 positions in historical order.
#[derive(Debug, Serialize, Deserialize)]
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

/// Closed carrier at the budget position with three states:
/// historical unbound (no budget), historical with budget, and new bound
/// with digest. Discriminant 2 was not used in the old 9-field layout.
#[derive(Debug, Serialize, Deserialize)]
enum Carrier {
    LegacyUnbound,
    LegacyWithBudget(AggregateResourceBudget),
    Bound { digest: WorkflowDigest, budget: Option<AggregateResourceBudget> },
}

impl CompatibilityView {
    fn from_admission(admission: &RunAdmission) -> Self {
        let carrier = match (&admission.input_digest, admission.budget) {
            (RunInputBinding::LegacyUnbound, None) => Carrier::LegacyUnbound,
            (RunInputBinding::LegacyUnbound, Some(budget)) => {
                Carrier::LegacyWithBudget(budget)
            }
            (RunInputBinding::Digest(digest), budget) => Carrier::Bound {
                digest: *digest,
                budget,
            },
        };
        Self {
            artifact_digest: admission.artifact_digest(),
            run_id: admission.run_id(),
            granted_capabilities: admission.granted_capabilities().clone(),
            policy: admission.policy(),
            policy_pin: admission.policy_pin(),
            carrier,
            tenant: admission.tenant(),
            idempotency_attested: admission.idempotency_attested().to_vec().into_boxed_slice(),
            ir_compat_decision: admission.ir_compat_decision().map(|d| Box::new(d)),
        }
    }

    fn into_admission(self) -> RunAdmission {
        let (input_digest, budget) = match self.carrier {
            Carrier::LegacyUnbound => (RunInputBinding::LegacyUnbound, None),
            Carrier::LegacyWithBudget(b) => (RunInputBinding::LegacyUnbound, Some(b)),
            Carrier::Bound { digest, budget } => (RunInputBinding::Digest(digest), budget),
        };
        let mut admission = RunAdmission::new(
            self.artifact_digest,
            self.run_id,
            self.granted_capabilities,
            self.policy,
        );
        admission = admission.with_policy_pin(self.policy_pin);
        admission = admission.with_input_digest(input_digest);
        if let Some(b) = budget {
            admission = admission.with_attached_budget(b);
        }
        if let Some(t) = self.tenant {
            admission = admission.with_tenant(t);
        }
        if !self.idempotency_attested.is_empty() {
            admission = admission.with_idempotency_evidence(
                self.artifact_digest,
                self.run_id,
                self.granted_capabilities,
                self.policy,
                self.idempotency_attested,
            );
            admission = admission.with_policy_pin(self.policy_pin);
            admission = admission.with_input_digest(input_digest);
            if let Some(b) = budget {
                admission = admission.with_attached_budget(b);
            }
            if let Some(t) = self.tenant {
                admission = admission.with_tenant(t);
            }
        }
        if let Some(d) = self.ir_compat_decision {
            admission = admission.with_ir_compat_decision(*d);
        }
        admission
    }
}

pub fn serialize_admission(admission: &RunAdmission) -> Result<Vec<u8>, postcard::to_allocvec::Error> {
    let view = CompatibilityView::from_admission(admission);
    postcard::to_allocvec(&view)
}

pub fn deserialize_admission(bytes: &[u8]) -> Result<RunAdmission, postcard::de::Error> {
    let view: CompatibilityView = postcard::from_bytes(bytes)?;
    Ok(view.into_admission())
}