use std::fmt;

/// A soft verification failure that does not block admission but should be reported.
///
/// Each warning is associated with a specific verification gate (1-2 range per
/// contract §4.2) and carries a numeric code and human-readable message.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VerificationWarning {
    /// Numeric code identifying the specific warning condition.
    pub code: u32,
    /// Human-readable description of the warning.
    pub message: Box<str>,
    /// Which verification gate produced this warning (1-2 range per contract).
    pub gate: u8,
}

impl VerificationWarning {
    /// Minimum valid gate value (inclusive).
    pub const MIN_GATE: u8 = 1;
    /// Maximum valid gate value (inclusive). Contract §4.2 specifies gate_count = 15.
    pub const MAX_GATE: u8 = 15;

    /// Returns `true` if the `gate` field falls within the valid 1-2 range.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.gate >= Self::MIN_GATE && self.gate <= Self::MAX_GATE
    }
}

impl fmt::Display for VerificationWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "gate {}: [{}] {}", self.gate, self.code, self.message)
    }
}

/// Proof flag that must be true for an accepted artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum ProofFlag {
    /// Artifact IR is size-bounded.
    Bounded,
    /// Artifact does not propagate taint.
    TaintSafe,
    /// Artifact actions are safe to retry.
    RetrySafe,
    /// Artifact can be replayed.
    Replayable,
}

/// Legacy display flags plus typed, payload-bound artifact evidence.
///
/// Flags and counts are not admission authority. Consumers must independently
/// run `verify_accepted_artifact`; legacy constructors are unverified.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VerificationProof {
    /// Confirmed digest of the verified artifact.
    pub digest: vb_core::WorkflowDigest,
    /// Number of verification gates that passed.
    pub gate_count: u8,
    /// Whether the proof was durably persisted (SyncAll).
    pub durable: bool,
    /// Artifact IR is size-bounded (CLAIMED - actual verification not yet implemented).
    pub bounded_claimed: bool,
    /// Artifact does not propagate taint (CLAIMED - actual verification not yet implemented).
    pub taint_safe_claimed: bool,
    /// Artifact actions are safe to retry (CLAIMED - actual verification not yet implemented).
    pub retry_safe_claimed: bool,
    /// Artifact idempotency evidence was verified by the acceptance gate (CLAIMED).
    pub idempotency_verified_claimed: bool,
    /// Artifact can be replayed (CLAIMED - actual verification not yet implemented).
    pub replayable_claimed: bool,
    /// Actions keyed by idempotency key.
    pub idempotency_keyed: Box<[vb_core::ActionId]>,
    /// Actions with idempotency attested.
    pub idempotency_attested: Box<[vb_core::ActionId]>,
    /// Soft verification failures encountered during admission.
    pub warnings: Vec<VerificationWarning>,
    /// Typed evidence bound by the accepted envelope digest.
    pub evidence: vb_core::artifact::ArtifactEvidence,
}

/// Allocation-free core of [`VerificationProof`] construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VerificationProofCore {
    /// Confirmed digest of the verified artifact.
    pub(crate) digest: vb_core::WorkflowDigest,
    /// Number of verification gates that passed.
    pub(crate) gate_count: u8,
    /// Whether the proof was durably persisted.
    pub(crate) durable: bool,
    /// Artifact IR is size-bounded claim flag.
    pub(crate) bounded_claimed: bool,
    /// Artifact taint-safety claim flag.
    pub(crate) taint_safe_claimed: bool,
    /// Artifact retry-safety claim flag.
    pub(crate) retry_safe_claimed: bool,
    /// Artifact idempotency claim flag.
    pub(crate) idempotency_verified_claimed: bool,
    /// Artifact replayability claim flag.
    pub(crate) replayable_claimed: bool,
}

pub(crate) const fn verification_proof_core(
    digest: vb_core::WorkflowDigest,
    _gate_count: u8,
    _durable: bool,
) -> VerificationProofCore {
    VerificationProofCore {
        digest,
        gate_count: 0,
        durable: false,
        bounded_claimed: false,
        taint_safe_claimed: false,
        retry_safe_claimed: false,
        idempotency_verified_claimed: false,
        replayable_claimed: false,
    }
}

impl VerificationProof {
    /// Creates unverified legacy metadata. Supplied counts and durability are
    /// intentionally ignored: caller assertions cannot mint verification.
    /// Use `prepare_compiled_artifact` for checked artifact construction.
    #[must_use]
    pub fn new(digest: vb_core::WorkflowDigest, gate_count: u8, durable: bool) -> Self {
        let core = verification_proof_core(digest, gate_count, durable);
        Self {
            digest: core.digest,
            gate_count: core.gate_count,
            durable: core.durable,
            bounded_claimed: core.bounded_claimed,
            taint_safe_claimed: core.taint_safe_claimed,
            retry_safe_claimed: core.retry_safe_claimed,
            idempotency_verified_claimed: core.idempotency_verified_claimed,
            replayable_claimed: core.replayable_claimed,
            idempotency_keyed: Box::new([]),
            idempotency_attested: Box::new([]),
            warnings: Vec::new(),
            evidence: vb_core::artifact::ArtifactEvidence::Unverified,
        }
    }
}
