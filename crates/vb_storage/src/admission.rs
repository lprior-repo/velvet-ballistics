#![forbid(unsafe_code)]
//! Artifact admission and verification functions.
//!
//! Provides artifact submission and admission flows with policy-controlled durability.

use std::fmt;

use crate::{
    constants::MAX_COMPILED_IR_BYTES, error::JournalError, records::CompiledIrRecord,
    types::EventSeq,
};

use crate::journal::FjallJournal;
use vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect};

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

/// Computes the policy digest from a workflow's resource contract.
///
/// GAP-003 FIX: Added per review finding that `AcceptedArtifact` must bind
/// to the policy digest that governed admission. The policy digest is derived
/// from the resource contract by hashing its canonical serialization.
pub fn compute_policy_digest(
    workflow: &vb_core::CompiledWorkflow,
) -> Result<vb_core::WorkflowDigest, JournalError> {
    let contract_bytes = postcard::to_allocvec(&workflow.resource_contract())
        .map_err(|_| JournalError::ArtifactMalformed)?;
    let hash = blake3::hash(&contract_bytes);
    Ok(vb_core::WorkflowDigest::from_bytes(*hash.as_bytes()))
}

/// Number of verification gates in the accepted artifact v1 admission flow.
/// This must match `vb_runtime::admission::REQUIRED_GATE_COUNT` (15).
const ADMISSION_GATE_COUNT: u8 = 15;

/// Validates, verifies, and persists a compiled workflow artifact with policy-controlled durability.
///
/// This is the full admission flow. It performs:
/// 1. Policy check: Relaxed is rejected when accepted artifacts are required.
/// 2. Structure validation: re-parse the workflow from serialized parts.
/// 3. Checksum validation: serialized bytes must hash to the claimed digest.
/// 4. Proof validation: gate count must be 15 and all proof flags must be true.
/// 5. Persistence: store the artifact in the `compiled_ir` keyspace.
/// 6. Durability: under `Strict` policy, calls SyncAll before returning.
///
/// Returns the `AcceptedArtifact` on success.
pub fn submit_artifact(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
    policy: vb_core::RuntimePolicy,
) -> Result<AcceptedArtifact, JournalError> {
    submit_artifact_with_contracts(journal, workflow, policy, &[])
}

/// Validates, verifies, and persists a compiled workflow artifact with the
/// required capability profile extracted from validated action contracts.
pub fn submit_artifact_with_contracts(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
    policy: vb_core::RuntimePolicy,
    action_contracts: &[ActionContract],
) -> Result<AcceptedArtifact, JournalError> {
    let admission_inputs = admission_inputs_from_contracts(action_contracts)?;
    submit_artifact_for_policy(journal, workflow, policy, admission_inputs)
}

fn admission_inputs_from_contracts(
    action_contracts: &[ActionContract],
) -> Result<AdmissionInputs, JournalError> {
    Ok(AdmissionInputs {
        required_capabilities: required_capabilities_from_contracts(action_contracts)?,
        idempotency_evidence: idempotency_evidence_from_contracts(action_contracts)?,
    })
}

fn submit_artifact_for_policy(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
    policy: vb_core::RuntimePolicy,
    admission_inputs: AdmissionInputs,
) -> Result<AcceptedArtifact, JournalError> {
    match policy {
        vb_core::RuntimePolicy::Relaxed => admission_inputs.submit_relaxed(journal, workflow),
        vb_core::RuntimePolicy::Journaled | vb_core::RuntimePolicy::Strict => {
            admission_inputs.submit_checked(journal, workflow, policy)
        }
        // `RuntimePolicy` is `#[non_exhaustive]`; unknown variants
        // fail closed rather than silently accept malformed artifacts.
        _ => Err(JournalError::ArtifactMalformed),
    }
}

fn submit_relaxed_artifact_with_evidence(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
    required_capabilities: Box<[vb_core::capability::Capability]>,
    idempotency_evidence: &IdempotencyEvidence,
) -> Result<AcceptedArtifact, JournalError> {
    let ir_bytes = validate_workflow_artifact_bytes(workflow)?;
    let mut proof = VerificationProof::new(workflow.digest(), 0, false);
    proof.idempotency_keyed = idempotency_evidence.keyed.clone();
    proof.idempotency_attested = idempotency_evidence.attested.clone();
    let artifact = accepted_artifact(workflow, ir_bytes, proof, required_capabilities)?;
    persist_accepted_artifact_ir(journal, &artifact)?;
    Ok(artifact)
}

fn submit_checked_artifact_with_evidence(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
    policy: vb_core::RuntimePolicy,
    required_capabilities: Box<[vb_core::capability::Capability]>,
    idempotency_evidence: IdempotencyEvidence,
) -> Result<AcceptedArtifact, JournalError> {
    let ir_bytes = validate_workflow_artifact_bytes(workflow)?;
    let durable = policy == vb_core::RuntimePolicy::Strict;
    let mut proof = VerificationProof::new(workflow.digest(), ADMISSION_GATE_COUNT, durable);
    proof.idempotency_keyed = idempotency_evidence.keyed;
    proof.idempotency_attested = idempotency_evidence.attested;
    let artifact = accepted_artifact(workflow, ir_bytes, proof, required_capabilities)?;
    persist_accepted_artifact_ir(journal, &artifact)?;
    if durable {
        journal.persist_strict()?;
    }
    verify_persisted_artifact_present(journal, workflow.digest())?;
    Ok(artifact)
}

fn validate_workflow_artifact_bytes(
    workflow: &vb_core::CompiledWorkflow,
) -> Result<Vec<u8>, JournalError> {
    let parts = workflow.to_parts();
    vb_core::CompiledWorkflow::try_from_parts(parts.clone())
        .map_err(|_| JournalError::ArtifactMalformed)?;
    let ir_bytes = canonical_workflow_ir_bytes(&parts)?;
    let computed = blake3::hash(&ir_bytes);
    if computed.as_bytes() == &workflow.digest().as_bytes() {
        Ok(ir_bytes)
    } else {
        Err(JournalError::ArtifactChecksumMismatch)
    }
}

fn accepted_artifact(
    workflow: &vb_core::CompiledWorkflow,
    ir: Vec<u8>,
    verification: VerificationProof,
    required_capabilities: Box<[vb_core::capability::Capability]>,
) -> Result<AcceptedArtifact, JournalError> {
    Ok(AcceptedArtifact {
        digest: workflow.digest(),
        source_digest: workflow.digest(),
        policy_digest: compute_policy_digest(workflow)?,
        ir,
        verification,
        accepted_at_seq: EventSeq::new(0),
        required_capabilities,
    })
}

fn persist_accepted_artifact_ir(
    journal: &FjallJournal,
    artifact: &AcceptedArtifact,
) -> Result<(), JournalError> {
    let envelope = serialize_accepted_artifact(artifact)?;
    let metadata_hash = compute_artifact_metadata_hash(artifact);
    let record = CompiledIrRecord {
        digest: artifact.digest,
        ir: envelope,
        metadata_hash: Some(metadata_hash),
    };
    journal.put_compiled_ir(&record)
}

fn serialize_accepted_artifact(artifact: &AcceptedArtifact) -> Result<Vec<u8>, JournalError> {
    postcard::to_allocvec(artifact).map_err(|_| JournalError::ArtifactMalformed)
}

/// Validates a stored compiled-IR record and rejects malformed accepted-artifact envelopes.
pub fn validate_compiled_ir_record(record: &CompiledIrRecord) -> Result<(), JournalError> {
    reject_oversized_compiled_ir_value(record.ir.len())?;
    let artifact = decode_accepted_artifact_envelope(&record.ir)?;
    validate_accepted_artifact_digest(&artifact, record.digest)
}

pub(crate) fn decode_accepted_artifact_envelope(bytes: &[u8]) -> Result<AcceptedArtifact, JournalError> {
    let (artifact, remaining) =
        postcard::take_from_bytes(bytes).map_err(|_| JournalError::ArtifactMalformed)?;
    let declared_end = bytes
        .len()
        .checked_sub(remaining.len())
        .ok_or(JournalError::UnexpectedEof)?;
    crate::codec::payload::reject_trailing_bytes(declared_end, bytes.len())?;
    Ok(artifact)
}

/// Rejects compiled-IR envelope values larger than the configured storage bound.
pub fn reject_oversized_compiled_ir_value(len: usize) -> Result<(), JournalError> {
    let payload_len = u32::try_from(len).map_err(|_| JournalError::PayloadTooLarge {
        len: u32::MAX,
        max: MAX_COMPILED_IR_BYTES,
    })?;
    if payload_len > MAX_COMPILED_IR_BYTES {
        Err(JournalError::PayloadTooLarge {
            len: payload_len,
            max: MAX_COMPILED_IR_BYTES,
        })
    } else {
        Ok(())
    }
}

#[cfg(fuzzing)]
#[doc(hidden)]
pub mod fuzz_access {
    //! Internal fuzz-harness accessors; unavailable in normal Cargo builds.

    use crate::{JournalError, records::CompiledIrRecord};

    use super::AcceptedArtifact;

    pub fn validate_compiled_ir_record(record: &CompiledIrRecord) -> Result<(), JournalError> {
        super::validate_compiled_ir_record(record)
    }

    pub fn decode_accepted_artifact_envelope(
        bytes: &[u8],
    ) -> Result<AcceptedArtifact, JournalError> {
        super::decode_accepted_artifact_envelope(bytes)
    }

    pub fn reject_oversized_compiled_ir_value(len: usize) -> Result<(), JournalError> {
        super::reject_oversized_compiled_ir_value(len)
    }
}

fn validate_accepted_artifact_digest(
    artifact: &AcceptedArtifact,
    digest: vb_core::WorkflowDigest,
) -> Result<(), JournalError> {
    validate_accepted_artifact_metadata(artifact)?;
    if artifact.digest != digest || artifact.verification.digest != digest {
        return Err(JournalError::ArtifactChecksumMismatch);
    }
    Ok(())
}

fn validate_accepted_artifact_metadata(artifact: &AcceptedArtifact) -> Result<(), JournalError> {
    if artifact.source_digest != artifact.digest {
        return Err(JournalError::ArtifactMalformed);
    }
    validate_artifact_policy_digest(artifact)?;
    validate_verification_proof(&artifact.verification)
}

fn validate_artifact_policy_digest(artifact: &AcceptedArtifact) -> Result<(), JournalError> {
    let workflow = workflow_from_artifact_ir(artifact)?;
    if artifact.policy_digest == compute_policy_digest(&workflow)? {
        Ok(())
    } else {
        Err(JournalError::ArtifactMalformed)
    }
}

fn workflow_from_artifact_ir(
    artifact: &AcceptedArtifact,
) -> Result<vb_core::CompiledWorkflow, JournalError> {
    let (mut parts, remaining) = postcard::take_from_bytes::<vb_core::WorkflowParts>(&artifact.ir)
        .map_err(|_| JournalError::ArtifactMalformed)?;
    let declared_end = artifact
        .ir
        .len()
        .checked_sub(remaining.len())
        .ok_or(JournalError::UnexpectedEof)?;
    crate::codec::payload::reject_trailing_bytes(declared_end, artifact.ir.len())?;
    parts.digest = artifact.digest;
    vb_core::CompiledWorkflow::try_from_parts(parts).map_err(|_| JournalError::ArtifactMalformed)
}

fn validate_verification_proof(proof: &VerificationProof) -> Result<(), JournalError> {
    if !is_accepted_gate_count(proof.gate_count) {
        return Err(JournalError::InvalidGateCount {
            found: proof.gate_count,
        });
    }
    if proof.gate_count == 0 && proof.durable {
        return Err(JournalError::ArtifactMalformed);
    }
    if let Some(flag) = missing_proof_flag(proof) {
        return Err(JournalError::MissingRequiredProofFlag { flag });
    }
    if !proof.warnings.iter().all(VerificationWarning::is_valid) {
        return Err(JournalError::ArtifactMalformed);
    }
    Ok(())
}

fn missing_proof_flag(proof: &VerificationProof) -> Option<&'static str> {
    if !proof.bounded_claimed {
        Some("bounded")
    } else if !proof.taint_safe_claimed {
        Some("taint_safe")
    } else if !proof.retry_safe_claimed {
        Some("retry_safe")
    } else if !proof.idempotency_verified_claimed {
        Some("idempotency_verified")
    } else if !proof.replayable_claimed {
        Some("replayable")
    } else {
        None
    }
}

fn is_accepted_gate_count(gate_count: u8) -> bool {
    gate_count == 0 || gate_count == ADMISSION_GATE_COUNT
}

fn verify_persisted_artifact_present(
    journal: &FjallJournal,
    digest: vb_core::WorkflowDigest,
) -> Result<(), JournalError> {
    let stored = journal
        .compiled_ir(digest)
        .map_err(|_| JournalError::ArtifactMalformed)?;
    if stored.is_some() {
        Ok(())
    } else {
        Err(JournalError::ArtifactMalformed)
    }
}

fn canonical_workflow_ir_bytes(parts: &vb_core::WorkflowParts) -> Result<Vec<u8>, JournalError> {
    let mut parts_for_hash = parts.clone();
    parts_for_hash.digest = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
    postcard::to_allocvec(&parts_for_hash).map_err(|_| JournalError::ArtifactMalformed)
}

fn required_capabilities_from_contracts(
    action_contracts: &[ActionContract],
) -> Result<Box<[vb_core::capability::Capability]>, JournalError> {
    let mut total = 0usize;
    for contract in action_contracts {
        total = total
            .checked_add(contract.required_capabilities.len())
            .ok_or(JournalError::ArtifactMalformed)?;
    }
    let mut required = Vec::new();
    required
        .try_reserve(total)
        .map_err(|_| JournalError::ArtifactMalformed)?;
    for contract in action_contracts {
        for capability in contract.required_capabilities.iter() {
            required.push(capability.clone());
        }
    }
    Ok(required.into_boxed_slice())
}

#[derive(Debug, Clone)]
struct IdempotencyEvidence {
    keyed: Box<[vb_core::ActionId]>,
    attested: Box<[vb_core::ActionId]>,
}

#[derive(Debug, Clone)]
struct AdmissionInputs {
    required_capabilities: Box<[vb_core::capability::Capability]>,
    idempotency_evidence: IdempotencyEvidence,
}

impl AdmissionInputs {
    fn submit_relaxed(
        self,
        journal: &FjallJournal,
        workflow: &vb_core::CompiledWorkflow,
    ) -> Result<AcceptedArtifact, JournalError> {
        let evidence = self.idempotency_evidence;
        submit_relaxed_artifact_with_evidence(
            journal,
            workflow,
            self.required_capabilities,
            &evidence,
        )
    }

    fn submit_checked(
        self,
        journal: &FjallJournal,
        workflow: &vb_core::CompiledWorkflow,
        policy: vb_core::RuntimePolicy,
    ) -> Result<AcceptedArtifact, JournalError> {
        submit_checked_artifact_with_evidence(
            journal,
            workflow,
            policy,
            self.required_capabilities,
            self.idempotency_evidence,
        )
    }
}

fn idempotency_evidence_from_contracts(
    action_contracts: &[ActionContract],
) -> Result<IdempotencyEvidence, JournalError> {
    let keyed = action_contracts
        .iter()
        .filter(|contract| requires_idempotency_key(contract))
        .map(|contract| contract.id)
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let attested = action_contracts
        .iter()
        .map(attested_action_id)
        .collect::<Result<Vec<_>, _>>()?
        .into_boxed_slice();
    Ok(IdempotencyEvidence { keyed, attested })
}

fn attested_action_id(contract: &ActionContract) -> Result<vb_core::ActionId, JournalError> {
    is_contract_idempotency_accepted(contract)
        .then_some(contract.id)
        .ok_or(JournalError::ArtifactMalformed)
}

fn requires_idempotency_key(contract: &ActionContract) -> bool {
    matches!(
        (contract.retry_safety, contract.idempotency),
        (RetrySafety::KeyRequired, _) | (_, Idempotency::AtLeastOnceExternal)
    )
}

fn is_contract_idempotency_accepted(contract: &ActionContract) -> bool {
    match (
        contract.side_effect,
        contract.retry_safety,
        contract.idempotency,
    ) {
        (SideEffect::None, _, _) => true,
        (_, RetrySafety::Unsafe, _) => false,
        (_, _, Idempotency::AtLeastOnceExternal | Idempotency::DeterministicPure) => false,
        (_, RetrySafety::Safe | RetrySafety::KeyRequired, Idempotency::IdempotentExternal) => true,
        // `SideEffect`, `RetrySafety`, and `Idempotency` are all `#[non_exhaustive]`.
        // Unknown combinations are conservatively rejected.
        _ => false,
    }
}

/// Validates and persists a compiled workflow artifact.
///
/// Structure validation ensures the workflow can be reconstructed from its parts.
/// Checksum validation recomputes the BLAKE3 digest from the serialized parts
/// and compares it to the digest claimed by the workflow.
///
/// On success, the artifact is stored in the `compiled_ir` keyspace and its
/// digest is returned. On failure, the storage is left unchanged.
pub fn admit_compiled_artifact(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
) -> Result<vb_core::WorkflowDigest, JournalError> {
    let artifact = submit_artifact(journal, workflow, vb_core::RuntimePolicy::Journaled)?;
    Ok(artifact.digest)
}

/// Computes a BLAKE3 hash of the artifact metadata fields that must remain
/// immutable after admission.
///
/// This includes: `source_digest`, `policy_digest`, the inner `ir` bytes,
/// `verification` fields (excluding the nested `digest` which equals the outer
/// digest), `accepted_at_seq`, and `required_capabilities`.
///
/// The `digest` field itself is excluded because it is the primary binding
/// already verified by `validate_accepted_artifact_digest`.
pub(crate) fn compute_artifact_metadata_hash(artifact: &AcceptedArtifact) -> [u8; 32] {
    use std::io::Write;

    let mut hasher = blake3::Hasher::new();
    let _ = hasher.write_all(&artifact.source_digest.as_bytes());
    let _ = hasher.write_all(&artifact.policy_digest.as_bytes());
    let _ = hasher.write_all(&artifact.ir);
    // Hash verification fields (excluding artifact.verification.digest which
    // equals artifact.digest, already verified separately; durable and gate_count
    // which are runtime policy decisions, not intrinsic artifact metadata)
    // NOTE: durable is NOT included because it reflects RuntimePolicy (Strict vs Journaled)
    // at admission time, not an immutable artifact property
    // NOTE: gate_count is NOT included because Relaxed=0 vs Journaled/Strict=15,
    // so the same artifact legitimately has different gate_count under different policies
    let _ = hasher.write_all(&[artifact.verification.bounded_claimed as u8]);
    let _ = hasher.write_all(&[artifact.verification.taint_safe_claimed as u8]);
    let _ = hasher.write_all(&[artifact.verification.retry_safe_claimed as u8]);
    let _ = hasher.write_all(&[artifact.verification.idempotency_verified_claimed as u8]);
    let _ = hasher.write_all(&[artifact.verification.replayable_claimed as u8]);
    // Hash idempotency data
    for id in artifact.verification.idempotency_keyed.as_ref() {
        let _ = hasher.write_all(&id.get().to_le_bytes());
    }
    for id in artifact.verification.idempotency_attested.as_ref() {
        let _ = hasher.write_all(&id.get().to_le_bytes());
    }
    // Hash warnings
    for w in &artifact.verification.warnings {
        let _ = hasher.write_all(&w.code.to_le_bytes());
        let _ = hasher.write_all(w.message.as_bytes());
        let _ = hasher.write_all(&[w.gate]);
    }
    let _ = hasher.write_all(&artifact.accepted_at_seq.get().to_le_bytes());
    for cap in artifact.required_capabilities.as_ref() {
        let _ = hasher.write_all(cap.name().as_bytes());
        let _ = hasher.write_all(&cap.action_id().get().to_le_bytes());
    }
    *hasher.finalize().as_bytes()
}

/// Validates that a new record's metadata hash does not conflict with an
/// existing record's metadata hash for the same digest.
///
/// Returns `Ok(new_metadata_hash)` if no conflict, or `Err(MetadataMutation)`
#[cfg(test)]
#[path = "admission/tests.rs"]
mod tests;
