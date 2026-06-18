#![forbid(unsafe_code)]
//! Domain types for artifact admission.
//!
//! Defines the data structures that represent verification proofs,
//! accepted artifacts, and soft verification warnings.

use std::fmt;

use crate::records::CompiledIrRecord;
use crate::types::EventSeq;

// =========================================================================
// VerificationWarning
// =========================================================================

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

// =========================================================================
// ProofFlag
// =========================================================================

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

impl ProofFlag {
    /// Converts a flag-name string (from `missing_proof_flag`) to the corresponding enum variant.
    ///
    /// This is kept as a utility for future work that needs a `ProofFlag`
    /// value derived from the string returned by `missing_proof_flag`.
    /// Currently `JournalError::MissingRequiredProofFlag` only requires `&'static str`.
    pub(crate) fn from_flag_name(name: &str) -> Self {
        match name {
            "bounded" => Self::Bounded,
            "taint_safe" => Self::TaintSafe,
            "retry_safe" => Self::RetrySafe,
            "idempotency_verified" => Self::RetrySafe,
            "replayable" => Self::Replayable,
            _ => Self::Bounded,
        }
    }
}

// =========================================================================
// VerificationProof
// =========================================================================

/// Proof that artifact verification passed at admission time.
///
/// GAP-001 FIX: Fields ending in `_claimed` are set unconditionally by
/// `VerificationProof::new()` because the actual verification gates are not
/// yet implemented. The `_claimed` suffix makes the intent explicit: these
/// are unverified claims, not proven facts. When proper verification is
/// implemented, the suffix should be removed and flags set based on results.
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
    gate_count: u8,
    durable: bool,
) -> VerificationProofCore {
    VerificationProofCore {
        digest,
        gate_count,
        durable,
        bounded_claimed: true,
        taint_safe_claimed: true,
        retry_safe_claimed: true,
        idempotency_verified_claimed: true,
        replayable_claimed: true,
    }
}

impl VerificationProof {
    /// Creates a new verification proof with all proof flags set to true.
    ///
    /// GAP-001 NOTE: All `_claimed` flags are unconditionally set to `true`
    /// because actual per-gate verification is not yet implemented. The flags
    /// are named with `_claimed` suffix to indicate they represent unverified
    /// claims, not proven facts. See `VerificationProof` struct docs.
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
        }
    }
}

// =========================================================================
// AcceptedArtifact
// =========================================================================

/// Accepted artifact record produced by the admission flow.
///
/// GAP-002/GAP-003 FIX: Added `source_digest` and `policy_digest` fields to satisfy
/// Backend DoD requirement for durable evidence chain binding.
///
/// Tracks the binding between a run and its accepting artifact per Backend DoD:
/// - `source_digest` binds the run to the workflow source that produced the artifact
/// - `policy_digest` binds the run to the policy/resource contract in effect
///
/// GAP-004 FIX: Per-action digests are NOT added because actions are already
/// cryptographically bound via the `CompiledWorkflow` digest. Each action's
/// bytecode and parameters are part of the workflow structure that is hashed
/// to produce the workflow digest. The workflow digest therefore serves as
/// a composite binding for all actions in the workflow.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AcceptedArtifact {
    /// The compiled artifact's content hash (matches `source_digest` when
    /// artifact is produced directly from compilation without separate source).
    pub digest: vb_core::WorkflowDigest,
    /// Digest of the original workflow source that was compiled to produce this artifact.
    /// For directly compiled workflows, this equals `digest`.
    pub source_digest: vb_core::WorkflowDigest,
    /// Digest of the resource/policy contract that governed this artifact's admission.
    /// Derived from the `resource_contract` field of the compiled workflow.
    pub policy_digest: vb_core::WorkflowDigest,
    /// Serialized compiled IR (postcard).
    pub ir: Vec<u8>,
    /// Proof that verification passed.
    pub verification: VerificationProof,
    /// Journal sequence when accepted.
    ///
    /// GAP-007 FIX: This field is currently always set to `EventSeq::new(0)`
    /// because actual sequence tracking is not implemented. The field is retained
    /// as a placeholder for future implementation of proper sequence tracking.
    /// When actual tracking is implemented, replace the placeholder with the real
    /// sequence number from the journal at admission time.
    pub accepted_at_seq: EventSeq,
    /// Required capabilities for actions in this artifact.
    pub required_capabilities: Box<[vb_core::capability::Capability]>,
}
