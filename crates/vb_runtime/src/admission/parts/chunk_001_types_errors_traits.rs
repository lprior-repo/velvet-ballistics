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
    /// Granted capability count does not match the artifact's required count.
    ///
    /// RA-023 fix: a cardinality mismatch (extras, duplicates, or under-grants
    /// whose per-capability membership check would otherwise fabricate a
    /// `CapabilityDenied` on a granted capability) is reported via this typed
    /// error instead. Carries the raw counts so callers and the operator
    /// diagnostic surface can render an honest "set size mismatch" message
    /// without inventing capability data.
    #[error(
        "admission rejected: capability count mismatch: required {required_count}, granted {granted_count}"
    )]
    CapabilityCountMismatch {
        /// Number of capabilities the artifact requires.
        required_count: usize,
        /// Number of capabilities the caller granted.
        granted_count: usize,
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
