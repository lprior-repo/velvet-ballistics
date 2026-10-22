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
            idempotency_attested: Box::new([]),
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
            idempotency_attested,
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
            idempotency_attested: Box::new([]),
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

    /// Returns the idempotency-attested action IDs available to dispatch.
    #[must_use]
    pub fn idempotency_attested(&self) -> &[ActionId] {
        &self.idempotency_attested
    }
}
