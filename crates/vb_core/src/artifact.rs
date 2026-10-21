#![forbid(unsafe_code)]
//! Wire evidence is a claim until the runtime artifact verifier rechecks it.

use crate::{RuntimePolicy, WorkflowDigest};
use serde::{Deserialize, Serialize};

/// Minimum obligation vocabulary from master section 77.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObligationKind {
    /// IR excludes ambient/nondeterministic expression operations.
    Determinism,
    /// Action effects obey the supplied external declarations.
    EffectSafety,
    /// Required capability set comes only from the checked action contracts.
    CapabilityNonMinting,
    /// IR structure and computed budget fit enforced resource limits.
    Boundedness,
    /// Replay reconstructs effects without dispatching them.
    ReplayEffectSeparation,
    /// Canonical IR, table references and identities validate.
    ArtifactValidity,
    /// Action declarations have acceptable static retry/idempotency classes.
    IdempotencyClassification,
    /// Runtime recovery follows the declared implementation contract.
    RecoveryClassification,
    /// Runtime enforces dynamic secret/taint restrictions.
    TaintHandling,
    /// Resource policy identity is bound to the checked payload.
    PolicyBinding,
}

/// Explicit scope of an external assumption; these are not executed proofs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssumptionId {
    /// Executors implement the supplied action declarations, including effects.
    ExecutorHonorsActionContracts,
    /// Pinned runtime obeys master 77.3 replay/effect separation.
    RuntimeReplayEffectSeparation,
    /// Pinned runtime obeys master 77.6 recovery semantics.
    RuntimeRecoverySemantics,
    /// Pinned runtime enforces its taint policy on actual values.
    RuntimeTaintEnforcement,
}

/// What closes an obligation, including explicit unproved external assumptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObligationClosure {
    /// A production-bound static proof, never emitted by the IR validator.
    StaticProof { proof_digest: WorkflowDigest },
    /// A named runtime-side artifact verifier ran its scoped IR checks.
    RuntimeValidated { verifier_digest: WorkflowDigest },
    /// Executed runtime/journal evidence, never inferred from compilation.
    RuntimeEvidenced {
        journal_schema_digest: WorkflowDigest,
    },
    /// Explicit operator attestation, never inferred from a declaration.
    OperatorAttested {
        approval_policy_digest: WorkflowDigest,
    },
    /// A declared assumption. The digest identifies its declaration, not proof.
    ExternalAssumption {
        assumption_id: AssumptionId,
        evidence_digest: WorkflowDigest,
    },
}

/// A scoped obligation with the resource policy under which it was checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationLedgerEntry {
    /// The obligation addressed by this entry.
    pub obligation: ObligationKind,
    /// Precise evidence class; assumptions remain visible.
    pub closure: ObligationClosure,
    /// Resource policy bound to this entry.
    pub policy_digest: WorkflowDigest,
}

/// Evidence reconstructed independently by the runtime-side artifact verifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactVerificationEvidence {
    /// Explicit evidence schema. Legacy claims require regeneration.
    pub schema_version: u16,
    /// Exact postcard payload bytes, including its embedded workflow identity.
    pub payload_digest: WorkflowDigest,
    /// Canonical self-zeroed workflow identity; used by recovery.
    pub workflow_digest: WorkflowDigest,
    /// Exact serialized action declarations.
    pub action_contract_digest: WorkflowDigest,
    /// Exact serialized resource policy.
    pub policy_digest: WorkflowDigest,
    /// Policy requested during artifact preparation; no fsync acknowledgement.
    pub admission_policy: RuntimePolicy,
    /// Fingerprint of the validator implementation and its checked sources.
    pub verifier_digest: WorkflowDigest,
    /// All ten obligations in canonical order. No omitted or duplicate entries.
    pub ledger: [VerificationLedgerEntry; 10],
}

/// Legacy proof claims and checked evidence are different wire states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactEvidence {
    /// No verifier ran. This state cannot pass accepted-artifact verification.
    Unverified,
    /// Serialized claims that still require independent verifier acceptance.
    Verified(Box<ArtifactVerificationEvidence>),
    /// External packs require a trusted registered catalog and cannot verify as legacy claims.
    Packed(Box<crate::action_pack::PackedArtifactEvidence>),
}
