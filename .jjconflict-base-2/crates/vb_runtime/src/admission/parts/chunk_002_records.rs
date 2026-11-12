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
    /// Actual policy identity from the accepted artifact, or explicit absence.
    policy_pin: RunPolicyPin,
    /// Aggregate budget admitted for this run, when budget admission is used.
    budget: Option<AggregateResourceBudget>,
    /// Caller-supplied tenant the run is charged to, when tenant-scoped
    /// admission was used. Absent for admissions that predate tenant
    /// scoping or did not use it.
    #[serde(default)]
    tenant: Option<TenantId>,
    /// Actions whose idempotency evidence passed artifact admission.
    idempotency_attested: Box<[ActionId]>,
    /// §77.17 compatibility decision for the admitted artifact's IR
    /// version, when the strict/journaled admission path classified it.
    /// Absent for relaxed admissions, which skip the IR version check,
    /// and for admissions that predate compatibility journaling. Boxed:
    /// the three digests on the decision would otherwise push
    /// `RunAdmission` past the large-variant lint threshold.
    #[serde(default)]
    ir_compat_decision: Option<Box<CompatDecision>>,
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
