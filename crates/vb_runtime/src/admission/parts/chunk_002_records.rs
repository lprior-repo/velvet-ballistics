use serde::{Deserialize, Serialize};

/// Accepted run admission record, attached to a run frame after passing the admission gate.
///
/// Serde behavior is manual, not derived. The derived ten-position layout
/// broke decoding of historical nine-position records that predate
/// `input_digest`. Manual implementations route through a private
/// compatibility view (`compat_codec`) that preserves the nine-position
/// byte layout while supporting bound admissions with input digests via
/// a closed carrier at the budget position.
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
    /// Actual policy identity from the accepted artifact, or explicit absence.
    policy_pin: RunPolicyPin,
    /// Canonical typed-input identity, or explicit historical absence.
    input_digest: RunInputBinding,
    /// Aggregate budget admitted for this run, when budget admission is used.
    budget: Option<AggregateResourceBudget>,
    /// Caller-supplied tenant the run is charged to, when tenant-scoped
    /// admission was used. Absent for admissions that predate tenant
    /// scoping or did not use it.
    tenant: Option<TenantId>,
    /// Actions whose idempotency evidence passed artifact admission.
    idempotency_attested: Box<[ActionId]>,
    /// §77.17 compatibility decision for the admitted artifact's IR
    /// version, when the strict/journaled admission path classified it.
    /// Absent for relaxed admissions, which skip the IR version check,
    /// and for admissions that predate compatibility journaling. Boxed:
    /// the three digests on the decision would otherwise push
    /// `RunAdmission` past the large-variant lint threshold.
    ir_compat_decision: Option<Box<CompatDecision>>,
}

// compat_codec is included into this same module via admission.rs;
// reference it directly by name.

impl serde::Serialize for RunAdmission {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            // Human-readable: for bound admissions serialize all ten fields;
            // for unbound admissions omit input_digest to preserve the old
            // nine-field named-map shape. Deserialization defaults a missing
            // input_digest to LegacyUnbound.
            if matches!(self.input_digest, RunInputBinding::Digest(_)) {
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
                HumanBound {
                    artifact_digest: &self.artifact_digest,
                    run_id: &self.run_id,
                    granted_capabilities: &self.granted_capabilities,
                    policy: &self.policy,
                    policy_pin: &self.policy_pin,
                    input_digest: &self.input_digest,
                    budget: &self.budget,
                    tenant: &self.tenant,
                    idempotency_attested: &self.idempotency_attested,
                    ir_compat_decision: &self.ir_compat_decision,
                }
                .serialize(serializer)
            } else {
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
                HumanUnbound {
                    artifact_digest: &self.artifact_digest,
                    run_id: &self.run_id,
                    granted_capabilities: &self.granted_capabilities,
                    policy: &self.policy,
                    policy_pin: &self.policy_pin,
                    budget: &self.budget,
                    tenant: &self.tenant,
                    idempotency_attested: &self.idempotency_attested,
                    ir_compat_decision: &self.ir_compat_decision,
                }
                .serialize(serializer)
            }
        } else {
            // Binary: use the 9-position compatibility layout that
            // preserves old Postcard bytes for historical records.
            compat_codec::serialize_admission(self, serializer)
        }
    }
}

impl<'de> serde::Deserialize<'de> for RunAdmission {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
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
            let human = Human::deserialize(deserializer)?;
            Ok(RunAdmission {
                artifact_digest: human.artifact_digest,
                run_id: human.run_id,
                granted_capabilities: human.granted_capabilities,
                policy: human.policy,
                policy_pin: human.policy_pin,
                input_digest: human.input_digest,
                budget: human.budget,
                tenant: human.tenant,
                idempotency_attested: human.idempotency_attested,
                ir_compat_decision: human.ir_compat_decision,
            })
        } else {
            // Binary: decode the 9-position compatibility layout, which
            // handles both old historical records and new bound
            // admissions via the carrier discriminant.
            compat_codec::deserialize_admission(deserializer)
        }
    }
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
    /// Creates a new admission record with the default unverified policy pin.
    ///
    /// This is the compatibility constructor used by callers that do not have
    /// an accepted artifact's actual policy identity available.
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
            policy_pin: RunPolicyPin::Unverified,
            input_digest: RunInputBinding::LegacyUnbound,
            budget: None,
            tenant: None,
            idempotency_attested: Box::new([]),
            ir_compat_decision: None,
        }
    }

    /// Creates a new admission record carrying accepted idempotency evidence.
    ///
    /// This is the compatibility constructor used by callers that do not have
    /// an accepted artifact's actual policy identity available.
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
            policy_pin: RunPolicyPin::Unverified,
            input_digest: RunInputBinding::LegacyUnbound,
            budget: None,
            tenant: None,
            idempotency_attested,
            ir_compat_decision: None,
        }
    }

    /// Creates a new admission record carrying an aggregate resource budget.
    ///
    /// This is the compatibility constructor used by callers that do not have
    /// an accepted artifact's actual policy identity available.
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
            policy_pin: RunPolicyPin::Unverified,
            input_digest: RunInputBinding::LegacyUnbound,
            budget: Some(budget),
            tenant: None,
            idempotency_attested: Box::new([]),
            ir_compat_decision: None,
        }
    }

    /// Overrides the policy pin with the actual policy identity from an
    /// accepted artifact.
    ///
    /// Use this after constructing an admission with the compatibility
    /// constructors to attach a real policy pin when the artifact's actual
    /// policy identity is available.
    pub(crate) fn with_policy_pin(mut self, policy_pin: RunPolicyPin) -> Self {
        self.policy_pin = policy_pin;
        self
    }

    /// Attaches the canonical typed-input identity from the submission.
    pub(crate) fn with_input_digest(
        mut self,
        input_digest: RunInputBinding,
    ) -> Self {
        self.input_digest = input_digest;
        self
    }

    /// Returns the canonical typed-input identity, or explicit historical absence.
    #[must_use]
    pub fn input_digest(&self) -> RunInputBinding {
        self.input_digest
    }

    /// Attaches the workflow-derived aggregate budget to this admission.
    pub(crate) fn with_attached_budget(mut self, budget: AggregateResourceBudget) -> Self {
        self.budget = Some(budget);
        self
    }

    /// Records the caller-supplied tenant this run is charged to.
    pub(crate) fn with_tenant(mut self, tenant: TenantId) -> Self {
        self.tenant = Some(tenant);
        self
    }

    /// Records the §77.17 compatibility decision for the admitted
    /// artifact's IR version, so the submit path can journal it with the
    /// admission header.
    pub(crate) fn with_ir_compat_decision(mut self, decision: CompatDecision) -> Self {
        self.ir_compat_decision = Some(Box::new(decision));
        self
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

    /// Returns the actual policy identity from the accepted artifact, or explicit absence.
    #[must_use]
    pub fn policy_pin(&self) -> RunPolicyPin {
        self.policy_pin
    }

    /// Returns the admitted aggregate budget when budget admission was used.
    #[must_use]
    pub const fn budget(&self) -> Option<AggregateResourceBudget> {
        self.budget
    }

    /// Returns the tenant this run is charged to, when tenant-scoped
    /// admission was used.
    #[must_use]
    pub const fn tenant(&self) -> Option<TenantId> {
        self.tenant
    }

    /// Returns the §77.17 compatibility decision for the admitted
    /// artifact's IR version, when the admission path classified it.
    #[must_use]
    pub fn ir_compat_decision(&self) -> Option<CompatDecision> {
        self.ir_compat_decision.as_deref().copied()
    }

    /// Returns fresh remaining-budget counters for the admitted budget,
    /// when budget admission was used. Runtime enforcement points consume
    /// from these counters; exhaustion is a typed
    /// `RemainingBudgetError`, never a silent truncation.
    #[must_use]
    pub fn remaining_budget(&self) -> Option<RemainingBudget> {
        self.budget.as_ref().map(RemainingBudget::from_budget)
    }

    /// Returns the idempotency-attested action IDs available to dispatch.
    #[must_use]
    pub fn idempotency_attested(&self) -> &[ActionId] {
        &self.idempotency_attested
    }
}
